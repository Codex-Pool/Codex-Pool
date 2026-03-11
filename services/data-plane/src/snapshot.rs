use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use codex_pool_core::api::{
    DataPlaneSnapshot, DataPlaneSnapshotEventType, DataPlaneSnapshotEventsResponse,
};
use reqwest::StatusCode;
use tracing::warn;

use crate::app::AppState;

const SNAPSHOT_RETRY_INTERVAL_MS_ENV: &str = "SNAPSHOT_POLL_INTERVAL_MS";
const SNAPSHOT_EVENTS_WAIT_MS_ENV: &str = "SNAPSHOT_EVENTS_WAIT_MS";
const SNAPSHOT_EVENTS_LIMIT_ENV: &str = "SNAPSHOT_EVENTS_LIMIT";
const DEFAULT_SNAPSHOT_RETRY_INTERVAL_MS: u64 = 1_000;
const MIN_SNAPSHOT_RETRY_INTERVAL_MS: u64 = 20;
const MAX_SNAPSHOT_RETRY_INTERVAL_MS: u64 = 10_000;
const DEFAULT_SNAPSHOT_EVENTS_WAIT_MS: u64 = 30_000;
const MIN_SNAPSHOT_EVENTS_WAIT_MS: u64 = 0;
const MAX_SNAPSHOT_EVENTS_WAIT_MS: u64 = 60_000;
const DEFAULT_SNAPSHOT_EVENTS_LIMIT: u32 = 500;
const MIN_SNAPSHOT_EVENTS_LIMIT: u32 = 1;
const MAX_SNAPSHOT_EVENTS_LIMIT: u32 = 5_000;

impl AppState {
    pub fn apply_snapshot(&self, snapshot: DataPlaneSnapshot) {
        let current_revision = self.snapshot_revision.load(Ordering::Relaxed);
        let current_cursor = self.snapshot_cursor.load(Ordering::Relaxed);
        if snapshot.revision < current_revision
            || (snapshot.revision == current_revision && snapshot.cursor <= current_cursor)
        {
            return;
        }

        let DataPlaneSnapshot {
            revision,
            cursor,
            accounts,
            compiled_routing_plan,
            ..
        } = snapshot;

        self.router.replace_accounts(accounts);
        self.router.replace_compiled_routing_plan(compiled_routing_plan);
        self.snapshot_revision
            .store(revision, Ordering::Relaxed);
        self.snapshot_cursor.store(cursor, Ordering::Relaxed);
        self.snapshot_remote_cursor
            .store(cursor, Ordering::Relaxed);
        self.notify_route_updated();
    }

    pub fn apply_snapshot_events(&self, response: DataPlaneSnapshotEventsResponse) -> u64 {
        let mut max_cursor = response.cursor;
        let mut remote_cursor = response.high_watermark.max(response.cursor);
        let mut applied = 0_u64;
        let mut routing_changed = false;

        for event in response.events {
            max_cursor = max_cursor.max(event.id);
            remote_cursor = remote_cursor.max(event.id);
            match event.event_type {
                DataPlaneSnapshotEventType::AccountUpsert => {
                    if let Some(account) = event.account {
                        self.router.upsert_account(account);
                        routing_changed = true;
                    } else if self.router.delete_account(event.account_id) {
                        routing_changed = true;
                    }
                    if event.compiled_routing_plan.is_some() {
                        self.router
                            .replace_compiled_routing_plan(event.compiled_routing_plan);
                        routing_changed = true;
                    }
                    applied = applied.saturating_add(1);
                }
                DataPlaneSnapshotEventType::AccountDelete => {
                    if self.router.delete_account(event.account_id) {
                        routing_changed = true;
                    }
                    if event.compiled_routing_plan.is_some() {
                        self.router
                            .replace_compiled_routing_plan(event.compiled_routing_plan);
                        routing_changed = true;
                    }
                    applied = applied.saturating_add(1);
                }
                DataPlaneSnapshotEventType::RoutingPlanRefresh => {
                    self.router
                        .replace_compiled_routing_plan(event.compiled_routing_plan);
                    routing_changed = true;
                    applied = applied.saturating_add(1);
                }
            }
        }

        if applied > 0 {
            self.snapshot_events_apply_total
                .fetch_add(applied, Ordering::Relaxed);
        }
        if max_cursor > self.snapshot_cursor.load(Ordering::Relaxed) {
            self.snapshot_cursor.store(max_cursor, Ordering::Relaxed);
        }
        if remote_cursor > self.snapshot_remote_cursor.load(Ordering::Relaxed) {
            self.snapshot_remote_cursor
                .store(remote_cursor, Ordering::Relaxed);
        }
        if routing_changed {
            self.notify_route_updated();
        }
        self.snapshot_cursor.load(Ordering::Relaxed)
    }
}

pub struct SnapshotPoller {
    client: reqwest::Client,
    control_plane_base_url: String,
    retry_interval: Duration,
    events_wait_ms: u64,
    events_limit: u32,
    state: Arc<AppState>,
}

enum PollEventsError {
    CursorGone,
    Other(anyhow::Error),
}

impl SnapshotPoller {
    pub fn from_env(client: reqwest::Client, state: Arc<AppState>) -> Option<Self> {
        let control_plane_base_url = std::env::var("CONTROL_PLANE_BASE_URL").ok()?;
        let retry_interval_ms = parse_u64_env(
            SNAPSHOT_RETRY_INTERVAL_MS_ENV,
            DEFAULT_SNAPSHOT_RETRY_INTERVAL_MS,
            MIN_SNAPSHOT_RETRY_INTERVAL_MS,
            MAX_SNAPSHOT_RETRY_INTERVAL_MS,
        );
        let events_wait_ms = parse_u64_env(
            SNAPSHOT_EVENTS_WAIT_MS_ENV,
            DEFAULT_SNAPSHOT_EVENTS_WAIT_MS,
            MIN_SNAPSHOT_EVENTS_WAIT_MS,
            MAX_SNAPSHOT_EVENTS_WAIT_MS,
        );
        let events_limit = parse_u32_env(
            SNAPSHOT_EVENTS_LIMIT_ENV,
            DEFAULT_SNAPSHOT_EVENTS_LIMIT,
            MIN_SNAPSHOT_EVENTS_LIMIT,
            MAX_SNAPSHOT_EVENTS_LIMIT,
        );

        Some(Self::new(
            client,
            control_plane_base_url,
            Duration::from_millis(retry_interval_ms),
            events_wait_ms,
            events_limit,
            state,
        ))
    }

    pub fn new(
        client: reqwest::Client,
        control_plane_base_url: impl Into<String>,
        retry_interval: Duration,
        events_wait_ms: u64,
        events_limit: u32,
        state: Arc<AppState>,
    ) -> Self {
        Self {
            client,
            control_plane_base_url: control_plane_base_url.into(),
            retry_interval,
            events_wait_ms,
            events_limit,
            state,
        }
    }

    pub async fn run(self) {
        let snapshot_url = format!(
            "{}/api/v1/data-plane/snapshot",
            self.control_plane_base_url.trim_end_matches('/')
        );
        let events_url = format!(
            "{}/api/v1/data-plane/snapshot/events",
            self.control_plane_base_url.trim_end_matches('/')
        );
        let mut cursor = self.state.snapshot_cursor.load(Ordering::Relaxed);

        loop {
            if cursor == 0 {
                match self.fetch_full_snapshot(&snapshot_url).await {
                    Ok(next_cursor) => {
                        cursor = next_cursor;
                    }
                    Err(error) => {
                        warn!(
                            snapshot_url = %snapshot_url,
                            error = %error,
                            "initial snapshot load failed"
                        );
                        tokio::time::sleep(self.retry_interval).await;
                        continue;
                    }
                }
            }

            match self.poll_events_once(&events_url, cursor).await {
                Ok(next_cursor) => {
                    cursor = next_cursor;
                }
                Err(PollEventsError::CursorGone) => {
                    self.state
                        .snapshot_events_cursor_gone_total
                        .fetch_add(1, Ordering::Relaxed);
                    warn!(
                        events_url = %events_url,
                        cursor,
                        "snapshot events cursor gone, reloading full snapshot"
                    );
                    match self.fetch_full_snapshot(&snapshot_url).await {
                        Ok(next_cursor) => {
                            cursor = next_cursor;
                        }
                        Err(error) => {
                            warn!(
                                snapshot_url = %snapshot_url,
                                error = %error,
                                "full snapshot reload failed after cursor gone"
                            );
                            tokio::time::sleep(self.retry_interval).await;
                        }
                    }
                }
                Err(PollEventsError::Other(error)) => {
                    warn!(events_url = %events_url, error = %error, "snapshot events poll failed");
                    tokio::time::sleep(self.retry_interval).await;
                }
            }
        }
    }

    async fn fetch_full_snapshot(&self, snapshot_url: &str) -> anyhow::Result<u64> {
        let snapshot = self
            .client
            .get(snapshot_url)
            .bearer_auth(self.state.control_plane_internal_auth_token.as_ref())
            .send()
            .await?
            .error_for_status()?
            .json::<DataPlaneSnapshot>()
            .await?;

        self.state.apply_snapshot(snapshot);
        Ok(self.state.snapshot_cursor.load(Ordering::Relaxed))
    }

    async fn poll_events_once(&self, events_url: &str, after: u64) -> Result<u64, PollEventsError> {
        let request_url = format!(
            "{events_url}?after={after}&limit={}&wait_ms={}",
            self.events_limit, self.events_wait_ms
        );
        let response = self
            .client
            .get(&request_url)
            .bearer_auth(self.state.control_plane_internal_auth_token.as_ref())
            .send()
            .await
            .map_err(|err| PollEventsError::Other(err.into()))?;

        let status = response.status();
        if status == StatusCode::GONE {
            return Err(PollEventsError::CursorGone);
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(PollEventsError::Other(anyhow!(
                "events poll failed with status {}: {}",
                status,
                truncate_body(body)
            )));
        }

        let payload = response
            .json::<DataPlaneSnapshotEventsResponse>()
            .await
            .context("failed to decode snapshot events response")
            .map_err(PollEventsError::Other)?;
        let next_cursor = self.state.apply_snapshot_events(payload);
        Ok(next_cursor.max(after))
    }
}

fn parse_u64_env(key: &str, default: u64, min: u64, max: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn parse_u32_env(key: &str, default: u32, min: u32, max: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn truncate_body(mut body: String) -> String {
    const MAX_LEN: usize = 200;
    body.truncate(MAX_LEN);
    body
}
