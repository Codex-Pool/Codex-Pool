use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::Engine;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use codex_pool_core::api::{
    ImportOAuthRefreshTokenRequest, OAuthImportErrorSummary, OAuthImportItemStatus,
    OAuthImportJobActionResponse, OAuthImportJobItem, OAuthImportJobItemsResponse,
    OAuthImportJobStatus, OAuthImportJobSummary,
};
use codex_pool_core::model::UpstreamMode;
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::store::UpsertOneTimeSessionAccountRequest;
use crate::store::ControlPlaneStore;

const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
const DB_STATUS_QUEUED: &str = "queued";
const DB_STATUS_RUNNING: &str = "running";
const DB_STATUS_COMPLETED: &str = "completed";
const DB_STATUS_FAILED: &str = "failed";
const DB_STATUS_CANCELLED: &str = "cancelled";

const DB_ITEM_PENDING: &str = "pending";
const DB_ITEM_PROCESSING: &str = "processing";
const DB_ITEM_CREATED: &str = "created";
const DB_ITEM_UPDATED: &str = "updated";
const DB_ITEM_FAILED: &str = "failed";
const DB_ITEM_SKIPPED: &str = "skipped";
const DB_ITEM_CANCELLED: &str = "cancelled";

#[derive(Debug, Clone)]
pub struct ImportUploadFile {
    pub file_name: String,
    pub content: Bytes,
}

#[derive(Debug, Clone)]
pub struct CreateOAuthImportJobOptions {
    pub base_url: String,
    pub default_priority: i32,
    pub default_enabled: bool,
    pub default_mode: UpstreamMode,
}

impl Default for CreateOAuthImportJobOptions {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            default_priority: 100,
            default_enabled: true,
            default_mode: UpstreamMode::ChatGptSession,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PersistedImportItem {
    pub item: OAuthImportJobItem,
    pub request: Option<ImportTaskRequest>,
    pub raw_record: Option<Value>,
    pub normalized_record: Option<Value>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "payload")]
pub enum ImportTaskRequest {
    OAuthRefresh(ImportOAuthRefreshTokenRequest),
    OneTimeAccessToken(UpsertOneTimeSessionAccountRequest),
    ManualRefreshAccount(ManualRefreshTaskRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualRefreshTaskRequest {
    pub account_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ImportJobTask {
    pub item_id: u64,
    pub request: ImportTaskRequest,
}

#[derive(Debug, Clone)]
pub struct ImportTaskSuccess {
    pub created: bool,
    pub account_id: Option<Uuid>,
    pub chatgpt_account_id: Option<String>,
}

#[async_trait]
pub trait OAuthImportJobStore: Send + Sync {
    async fn create_job(
        &self,
        summary: OAuthImportJobSummary,
        items: Vec<PersistedImportItem>,
    ) -> Result<()>;

    async fn get_job_summary(&self, job_id: Uuid) -> Result<OAuthImportJobSummary>;

    async fn get_job_items(
        &self,
        job_id: Uuid,
        status: Option<OAuthImportItemStatus>,
        cursor: Option<u64>,
        limit: u64,
    ) -> Result<OAuthImportJobItemsResponse>;

    async fn start_job(&self, job_id: Uuid, limit: usize) -> Result<Vec<ImportJobTask>>;

    async fn mark_item_success(
        &self,
        job_id: Uuid,
        item_id: u64,
        outcome: &ImportTaskSuccess,
    ) -> Result<()>;

    async fn mark_item_failed(
        &self,
        job_id: Uuid,
        item_id: u64,
        error_code: &str,
        error_message: &str,
    ) -> Result<()>;

    async fn finish_job(&self, job_id: Uuid) -> Result<OAuthImportJobSummary>;

    async fn cancel_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse>;

    async fn retry_failed(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse>;

    async fn recoverable_job_ids(&self) -> Result<Vec<Uuid>>;
}

#[derive(Default)]
pub struct InMemoryOAuthImportJobStore {
    jobs: RwLock<HashMap<Uuid, Arc<Mutex<InMemoryJobState>>>>,
}

struct InMemoryJobState {
    summary: OAuthImportJobSummary,
    items: Vec<PersistedImportItem>,
    cancel_requested: bool,
}

#[async_trait]
impl OAuthImportJobStore for InMemoryOAuthImportJobStore {
    async fn create_job(
        &self,
        summary: OAuthImportJobSummary,
        items: Vec<PersistedImportItem>,
    ) -> Result<()> {
        let state = InMemoryJobState {
            summary,
            items,
            cancel_requested: false,
        };
        self.jobs
            .write()
            .await
            .insert(state.summary.job_id, Arc::new(Mutex::new(state)));
        Ok(())
    }

    async fn get_job_summary(&self, job_id: Uuid) -> Result<OAuthImportJobSummary> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let mut guard = job.lock().await;
        let cancel_requested = guard.cancel_requested;
        let items = guard.items.clone();
        refresh_summary_counts(&mut guard.summary, &items, cancel_requested);
        Ok(guard.summary.clone())
    }

    async fn get_job_items(
        &self,
        job_id: Uuid,
        status: Option<OAuthImportItemStatus>,
        cursor: Option<u64>,
        limit: u64,
    ) -> Result<OAuthImportJobItemsResponse> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let guard = job.lock().await;

        let effective_limit = limit.clamp(1, 500) as usize;
        let mut filtered = guard
            .items
            .iter()
            .filter(|item| {
                status
                    .as_ref()
                    .map(|target| target == &item.item.status)
                    .unwrap_or(true)
            })
            .map(|item| item.item.clone())
            .collect::<Vec<_>>();
        filtered.sort_by_key(|item| item.item_id);

        let start_idx = if let Some(cursor) = cursor {
            filtered
                .iter()
                .position(|item| item.item_id > cursor)
                .unwrap_or(filtered.len())
        } else {
            0
        };

        let items = filtered
            .iter()
            .skip(start_idx)
            .take(effective_limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor = items.last().map(|item| item.item_id).and_then(|last| {
            filtered
                .iter()
                .position(|item| item.item_id == last)
                .and_then(|idx| {
                    if idx + 1 < filtered.len() {
                        Some(last)
                    } else {
                        None
                    }
                })
        });

        Ok(OAuthImportJobItemsResponse { items, next_cursor })
    }

    async fn start_job(&self, job_id: Uuid, limit: usize) -> Result<Vec<ImportJobTask>> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let mut guard = job.lock().await;

        if guard.cancel_requested {
            return Ok(Vec::new());
        }
        if guard.summary.status == OAuthImportJobStatus::Running {
            return Ok(Vec::new());
        }

        guard.summary.status = OAuthImportJobStatus::Running;
        if guard.summary.started_at.is_none() {
            guard.summary.started_at = Some(Utc::now());
        }
        guard.summary.finished_at = None;

        let mut tasks = Vec::new();
        let mut claimed = 0usize;
        for item in &mut guard.items {
            if claimed >= limit {
                break;
            }
            if item.item.status != OAuthImportItemStatus::Pending {
                continue;
            }
            let Some(request) = item.request.clone() else {
                continue;
            };
            item.item.status = OAuthImportItemStatus::Processing;
            tasks.push(ImportJobTask {
                item_id: item.item.item_id,
                request,
            });
            claimed = claimed.saturating_add(1);
        }

        Ok(tasks)
    }

    async fn mark_item_success(
        &self,
        job_id: Uuid,
        item_id: u64,
        outcome: &ImportTaskSuccess,
    ) -> Result<()> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let mut guard = job.lock().await;

        if let Some(state) = guard
            .items
            .iter_mut()
            .find(|item| item.item.item_id == item_id)
        {
            state.item.status = if outcome.created {
                OAuthImportItemStatus::Created
            } else {
                OAuthImportItemStatus::Updated
            };
            state.item.account_id = outcome.account_id;
            state.item.chatgpt_account_id = outcome.chatgpt_account_id.clone();
            state.item.error_code = None;
            state.item.error_message = None;
            state.normalized_record = Some(serde_json::to_value(&state.request)?);
        }

        let cancel_requested = guard.cancel_requested;
        let items = guard.items.clone();
        refresh_summary_counts(&mut guard.summary, &items, cancel_requested);
        Ok(())
    }

    async fn mark_item_failed(
        &self,
        job_id: Uuid,
        item_id: u64,
        error_code: &str,
        error_message: &str,
    ) -> Result<()> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let mut guard = job.lock().await;

        if let Some(state) = guard
            .items
            .iter_mut()
            .find(|item| item.item.item_id == item_id)
        {
            state.item.status = OAuthImportItemStatus::Failed;
            state.item.error_code = Some(error_code.to_string());
            state.item.error_message = Some(error_message.to_string());
        }

        let cancel_requested = guard.cancel_requested;
        let items = guard.items.clone();
        refresh_summary_counts(&mut guard.summary, &items, cancel_requested);
        Ok(())
    }

    async fn finish_job(&self, job_id: Uuid) -> Result<OAuthImportJobSummary> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let mut guard = job.lock().await;

        if guard.cancel_requested {
            for item in &mut guard.items {
                if matches!(
                    item.item.status,
                    OAuthImportItemStatus::Pending | OAuthImportItemStatus::Processing
                ) {
                    item.item.status = OAuthImportItemStatus::Cancelled;
                }
            }
        }

        let cancel_requested = guard.cancel_requested;
        let items = guard.items.clone();
        refresh_summary_counts(&mut guard.summary, &items, cancel_requested);
        guard.summary.finished_at = Some(Utc::now());
        guard.summary.status = if guard.cancel_requested {
            OAuthImportJobStatus::Cancelled
        } else if guard.summary.failed_count > 0 {
            OAuthImportJobStatus::Failed
        } else {
            OAuthImportJobStatus::Completed
        };
        guard.summary.throughput_per_min = compute_throughput_per_min(
            guard.summary.started_at,
            guard.summary.finished_at,
            guard.summary.processed,
        );

        Ok(guard.summary.clone())
    }

    async fn cancel_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let mut guard = job.lock().await;

        guard.cancel_requested = true;
        if guard.summary.status == OAuthImportJobStatus::Queued {
            for item in &mut guard.items {
                if matches!(
                    item.item.status,
                    OAuthImportItemStatus::Pending | OAuthImportItemStatus::Processing
                ) {
                    item.item.status = OAuthImportItemStatus::Cancelled;
                }
            }
            let items = guard.items.clone();
            refresh_summary_counts(&mut guard.summary, &items, true);
            guard.summary.status = OAuthImportJobStatus::Cancelled;
            guard.summary.finished_at = Some(Utc::now());
        }

        Ok(OAuthImportJobActionResponse {
            job_id,
            accepted: true,
        })
    }

    async fn retry_failed(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let mut guard = job.lock().await;

        if guard.summary.status == OAuthImportJobStatus::Running {
            return Ok(OAuthImportJobActionResponse {
                job_id,
                accepted: false,
            });
        }

        guard.cancel_requested = false;
        for item in &mut guard.items {
            if item.item.status != OAuthImportItemStatus::Failed {
                continue;
            }
            if item.request.is_none() {
                continue;
            }
            item.item.status = OAuthImportItemStatus::Pending;
            item.item.error_code = None;
            item.item.error_message = None;
            item.item.account_id = None;
            item.retry_count = item.retry_count.saturating_add(1);
        }

        guard.summary.status = OAuthImportJobStatus::Queued;
        guard.summary.started_at = None;
        guard.summary.finished_at = None;
        guard.summary.throughput_per_min = None;
        let items = guard.items.clone();
        refresh_summary_counts(&mut guard.summary, &items, false);

        Ok(OAuthImportJobActionResponse {
            job_id,
            accepted: true,
        })
    }

    async fn recoverable_job_ids(&self) -> Result<Vec<Uuid>> {
        Ok(Vec::new())
    }
}

#[derive(Clone)]
pub struct PostgresOAuthImportJobStore {
    pool: PgPool,
}

impl PostgresOAuthImportJobStore {
    pub async fn new(pool: PgPool) -> Result<Self> {
        let this = Self { pool };
        this.ensure_schema().await?;
        Ok(this)
    }

    async fn ensure_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_import_jobs (
                id UUID PRIMARY KEY,
                status TEXT NOT NULL,
                cancel_requested BOOLEAN NOT NULL DEFAULT FALSE,
                total BIGINT NOT NULL,
                processed BIGINT NOT NULL,
                created_count BIGINT NOT NULL,
                updated_count BIGINT NOT NULL,
                failed_count BIGINT NOT NULL,
                skipped_count BIGINT NOT NULL,
                started_at TIMESTAMPTZ NULL,
                finished_at TIMESTAMPTZ NULL,
                created_at TIMESTAMPTZ NOT NULL,
                throughput_per_min DOUBLE PRECISION NULL,
                error_summary JSONB NOT NULL DEFAULT '[]'::jsonb
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create oauth_import_jobs table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_import_job_items (
                job_id UUID NOT NULL REFERENCES oauth_import_jobs(id) ON DELETE CASCADE,
                item_id BIGINT NOT NULL,
                source_file TEXT NOT NULL,
                line_no BIGINT NOT NULL,
                status TEXT NOT NULL,
                label TEXT NOT NULL,
                email TEXT NULL,
                chatgpt_account_id TEXT NULL,
                account_id UUID NULL,
                error_code TEXT NULL,
                error_message TEXT NULL,
                request_json JSONB NULL,
                raw_record JSONB NULL,
                normalized_record JSONB NULL,
                retry_count INT NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                PRIMARY KEY (job_id, item_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create oauth_import_job_items table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_oauth_import_job_items_status
            ON oauth_import_job_items (job_id, status, item_id)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create idx_oauth_import_job_items_status")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_oauth_import_job_items_cursor
            ON oauth_import_job_items (job_id, item_id)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create idx_oauth_import_job_items_cursor")?;

        Ok(())
    }

    async fn load_error_summary(&self, job_id: Uuid) -> Result<Vec<OAuthImportErrorSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT COALESCE(error_code, 'unknown') AS error_code, COUNT(*)::BIGINT AS count
            FROM oauth_import_job_items
            WHERE job_id = $1 AND status = $2
            GROUP BY COALESCE(error_code, 'unknown')
            ORDER BY COUNT(*) DESC
            LIMIT 20
            "#,
        )
        .bind(job_id)
        .bind(DB_ITEM_FAILED)
        .fetch_all(&self.pool)
        .await
        .context("failed to query oauth import error summary")?;

        let mut summary = Vec::with_capacity(rows.len());
        for row in rows {
            let count = row.try_get::<i64, _>("count")?;
            summary.push(OAuthImportErrorSummary {
                error_code: row.try_get("error_code")?,
                count: u64::try_from(count).unwrap_or_default(),
            });
        }
        Ok(summary)
    }

    async fn load_job_row(&self, job_id: Uuid) -> Result<OAuthImportJobSummary> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                status,
                total,
                processed,
                created_count,
                updated_count,
                failed_count,
                skipped_count,
                started_at,
                finished_at,
                created_at,
                throughput_per_min
            FROM oauth_import_jobs
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query oauth import job")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let started_at = row.try_get::<Option<DateTime<Utc>>, _>("started_at")?;
        let finished_at = row.try_get::<Option<DateTime<Utc>>, _>("finished_at")?;
        let processed = u64::try_from(row.try_get::<i64, _>("processed")?).unwrap_or_default();

        Ok(OAuthImportJobSummary {
            job_id: row.try_get("id")?,
            status: parse_job_status(row.try_get::<String, _>("status")?.as_str())?,
            total: u64::try_from(row.try_get::<i64, _>("total")?).unwrap_or_default(),
            processed,
            created_count: u64::try_from(row.try_get::<i64, _>("created_count")?)
                .unwrap_or_default(),
            updated_count: u64::try_from(row.try_get::<i64, _>("updated_count")?)
                .unwrap_or_default(),
            failed_count: u64::try_from(row.try_get::<i64, _>("failed_count")?).unwrap_or_default(),
            skipped_count: u64::try_from(row.try_get::<i64, _>("skipped_count")?)
                .unwrap_or_default(),
            started_at,
            finished_at,
            created_at: row.try_get("created_at")?,
            throughput_per_min: row
                .try_get::<Option<f64>, _>("throughput_per_min")?
                .or_else(|| compute_throughput_per_min(started_at, finished_at, processed)),
            error_summary: Vec::new(),
        })
    }

    async fn recompute_counts(&self, job_id: Uuid) -> Result<(u64, u64, u64, u64, u64)> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status IN ($2, $3, $4))::BIGINT AS processed,
                COUNT(*) FILTER (WHERE status = $2)::BIGINT AS created_count,
                COUNT(*) FILTER (WHERE status = $3)::BIGINT AS updated_count,
                COUNT(*) FILTER (WHERE status = $4)::BIGINT AS failed_count,
                COUNT(*) FILTER (WHERE status = $5)::BIGINT AS skipped_count
            FROM oauth_import_job_items
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .bind(DB_ITEM_CREATED)
        .bind(DB_ITEM_UPDATED)
        .bind(DB_ITEM_FAILED)
        .bind(DB_ITEM_SKIPPED)
        .fetch_one(&self.pool)
        .await
        .context("failed to recompute oauth import counts")?;

        Ok((
            u64::try_from(row.try_get::<i64, _>("processed")?).unwrap_or_default(),
            u64::try_from(row.try_get::<i64, _>("created_count")?).unwrap_or_default(),
            u64::try_from(row.try_get::<i64, _>("updated_count")?).unwrap_or_default(),
            u64::try_from(row.try_get::<i64, _>("failed_count")?).unwrap_or_default(),
            u64::try_from(row.try_get::<i64, _>("skipped_count")?).unwrap_or_default(),
        ))
    }
}

include!("import_jobs/store_impl.rs");
include!("import_jobs/manager_impl.rs");
