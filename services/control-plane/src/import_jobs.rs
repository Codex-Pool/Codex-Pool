#![cfg_attr(not(feature = "postgres-backend"), allow(dead_code, unused_imports))]

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::Engine;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use codex_pool_core::model::UpstreamMode;
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use sqlx_sqlite::SqlitePool;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::contracts::{
    ImportOAuthRefreshTokenRequest, OAuthImportAdmissionCounts, OAuthImportErrorSummary,
    OAuthImportItemStatus, OAuthImportJobActionResponse, OAuthImportJobItem,
    OAuthImportJobItemsResponse, OAuthImportJobStatus, OAuthImportJobSummary, OAuthInventoryRecord,
    OAuthInventoryFailureStage, OAuthVaultRecordStatus,
};
use crate::store::ControlPlaneStore;
#[cfg(feature = "postgres-backend")]
use crate::store::PgPool;
use crate::store::UpsertOneTimeSessionAccountRequest;

const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
const DB_STATUS_QUEUED: &str = "queued";
const DB_STATUS_RUNNING: &str = "running";
const DB_STATUS_PAUSED: &str = "paused";
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
const DB_FAILURE_STAGE_ADMISSION_PROBE: &str = "admission_probe";
const DB_FAILURE_STAGE_ACTIVATION_REFRESH: &str = "activation_refresh";
const DB_FAILURE_STAGE_ACTIVATION_RATE_LIMITS: &str = "activation_rate_limits";
const DB_FAILURE_STAGE_RUNTIME_REFRESH: &str = "runtime_refresh";

fn failure_stage_to_db(stage: OAuthInventoryFailureStage) -> &'static str {
    match stage {
        OAuthInventoryFailureStage::AdmissionProbe => DB_FAILURE_STAGE_ADMISSION_PROBE,
        OAuthInventoryFailureStage::ActivationRefresh => DB_FAILURE_STAGE_ACTIVATION_REFRESH,
        OAuthInventoryFailureStage::ActivationRateLimits => DB_FAILURE_STAGE_ACTIVATION_RATE_LIMITS,
        OAuthInventoryFailureStage::RuntimeRefresh => DB_FAILURE_STAGE_RUNTIME_REFRESH,
    }
}

fn parse_failure_stage(raw: &str) -> Result<OAuthInventoryFailureStage> {
    match raw {
        DB_FAILURE_STAGE_ADMISSION_PROBE => Ok(OAuthInventoryFailureStage::AdmissionProbe),
        DB_FAILURE_STAGE_ACTIVATION_REFRESH => Ok(OAuthInventoryFailureStage::ActivationRefresh),
        DB_FAILURE_STAGE_ACTIVATION_RATE_LIMITS => {
            Ok(OAuthInventoryFailureStage::ActivationRateLimits)
        }
        DB_FAILURE_STAGE_RUNTIME_REFRESH => Ok(OAuthInventoryFailureStage::RuntimeRefresh),
        _ => Err(anyhow!("unsupported oauth import failure stage: {raw}")),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportCredentialMode {
    Auto,
    RefreshToken,
    AccessToken,
}

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
    pub credential_mode: ImportCredentialMode,
}

impl Default for CreateOAuthImportJobOptions {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            default_priority: 100,
            default_enabled: true,
            default_mode: UpstreamMode::ChatGptSession,
            credential_mode: ImportCredentialMode::Auto,
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
    pub admission_status: Option<OAuthVaultRecordStatus>,
    pub admission_source: Option<String>,
    pub admission_reason: Option<String>,
    pub failure_stage: Option<OAuthInventoryFailureStage>,
    pub attempt_count: u32,
    pub transient_retry_count: u32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub retryable: bool,
    pub terminal_reason: Option<String>,
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

    async fn pause_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse>;

    async fn resume_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse>;

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
        let resuming_running_job = guard.summary.status == OAuthImportJobStatus::Running;

        if guard.cancel_requested {
            return Ok(Vec::new());
        }
        if matches!(
            guard.summary.status,
            OAuthImportJobStatus::Paused
                | OAuthImportJobStatus::Completed
                | OAuthImportJobStatus::Failed
                | OAuthImportJobStatus::Cancelled
        ) {
            return Ok(Vec::new());
        }

        guard.summary.status = OAuthImportJobStatus::Running;
        if guard.summary.started_at.is_none() {
            guard.summary.started_at = Some(Utc::now());
        }
        guard.summary.finished_at = None;

        if resuming_running_job {
            for item in &mut guard.items {
                if item.item.status == OAuthImportItemStatus::Processing {
                    item.item.status = OAuthImportItemStatus::Pending;
                }
            }
        }

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
            state.item.admission_status = outcome.admission_status;
            state.item.admission_source = outcome.admission_source.clone();
            state.item.admission_reason = outcome.admission_reason.clone();
            state.item.failure_stage = outcome.failure_stage;
            state.item.attempt_count = outcome.attempt_count;
            state.item.transient_retry_count = outcome.transient_retry_count;
            state.item.next_retry_at = outcome.next_retry_at;
            state.item.retryable = outcome.retryable;
            state.item.terminal_reason = outcome.terminal_reason.clone();
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
            state.item.admission_status = Some(OAuthVaultRecordStatus::Failed);
            state.item.admission_source = None;
            state.item.admission_reason = Some(error_code.to_string());
            state.item.failure_stage = None;
            state.item.attempt_count = 0;
            state.item.transient_retry_count = 0;
            state.item.next_retry_at = None;
            state.item.retryable = false;
            state.item.terminal_reason = Some(error_code.to_string());
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

    async fn pause_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let mut guard = job.lock().await;

        let accepted = matches!(
            guard.summary.status,
            OAuthImportJobStatus::Queued | OAuthImportJobStatus::Running
        ) && !guard.cancel_requested;

        if accepted {
            guard.summary.status = OAuthImportJobStatus::Paused;
            guard.summary.finished_at = None;
        }

        Ok(OAuthImportJobActionResponse { job_id, accepted })
    }

    async fn resume_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let job = self
            .jobs
            .read()
            .await
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))?;
        let mut guard = job.lock().await;

        let accepted =
            guard.summary.status == OAuthImportJobStatus::Paused && !guard.cancel_requested;
        if accepted {
            guard.summary.status = OAuthImportJobStatus::Queued;
            guard.summary.finished_at = None;
            guard.summary.throughput_per_min = None;
            for item in &mut guard.items {
                if item.item.status == OAuthImportItemStatus::Processing {
                    item.item.status = OAuthImportItemStatus::Pending;
                }
            }
            let items = guard.items.clone();
            refresh_summary_counts(&mut guard.summary, &items, false);
        }

        Ok(OAuthImportJobActionResponse { job_id, accepted })
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
        if matches!(
            guard.summary.status,
            OAuthImportJobStatus::Queued | OAuthImportJobStatus::Paused
        ) {
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

#[cfg(feature = "postgres-backend")]
#[derive(Clone)]
pub struct PostgresOAuthImportJobStore {
    pool: PgPool,
}

#[derive(Clone)]
pub struct SqliteOAuthImportJobStore {
    pool: SqlitePool,
}

impl SqliteOAuthImportJobStore {
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let this = Self { pool };
        this.ensure_schema().await?;
        Ok(this)
    }

    async fn ensure_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_import_jobs (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                cancel_requested INTEGER NOT NULL DEFAULT 0,
                total INTEGER NOT NULL,
                processed INTEGER NOT NULL,
                created_count INTEGER NOT NULL,
                updated_count INTEGER NOT NULL,
                failed_count INTEGER NOT NULL,
                skipped_count INTEGER NOT NULL,
                started_at TEXT NULL,
                finished_at TEXT NULL,
                created_at TEXT NOT NULL,
                throughput_per_min REAL NULL,
                error_summary TEXT NOT NULL DEFAULT '[]'
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create sqlite oauth_import_jobs table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_import_job_items (
                job_id TEXT NOT NULL,
                item_id INTEGER NOT NULL,
                source_file TEXT NOT NULL,
                line_no INTEGER NOT NULL,
                status TEXT NOT NULL,
                label TEXT NOT NULL,
                email TEXT NULL,
                chatgpt_account_id TEXT NULL,
                account_id TEXT NULL,
                error_code TEXT NULL,
                error_message TEXT NULL,
                admission_status TEXT NULL,
                admission_source TEXT NULL,
                admission_reason TEXT NULL,
                failure_stage TEXT NULL,
                attempt_count INTEGER NOT NULL DEFAULT 0,
                transient_retry_count INTEGER NOT NULL DEFAULT 0,
                next_retry_at TEXT NULL,
                retryable INTEGER NOT NULL DEFAULT 0,
                terminal_reason TEXT NULL,
                request_json TEXT NULL,
                raw_record TEXT NULL,
                normalized_record TEXT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (job_id, item_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create sqlite oauth_import_job_items table")?;

        Self::ensure_column_exists(
            &self.pool,
            "oauth_import_job_items",
            "admission_status",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            &self.pool,
            "oauth_import_job_items",
            "admission_source",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            &self.pool,
            "oauth_import_job_items",
            "admission_reason",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            &self.pool,
            "oauth_import_job_items",
            "failure_stage",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            &self.pool,
            "oauth_import_job_items",
            "attempt_count",
            "INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        Self::ensure_column_exists(
            &self.pool,
            "oauth_import_job_items",
            "transient_retry_count",
            "INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        Self::ensure_column_exists(
            &self.pool,
            "oauth_import_job_items",
            "next_retry_at",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            &self.pool,
            "oauth_import_job_items",
            "retryable",
            "INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        Self::ensure_column_exists(
            &self.pool,
            "oauth_import_job_items",
            "terminal_reason",
            "TEXT NULL",
        )
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_oauth_import_job_items_status
            ON oauth_import_job_items (job_id, status, item_id)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create sqlite idx_oauth_import_job_items_status")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_oauth_import_job_items_cursor
            ON oauth_import_job_items (job_id, item_id)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to create sqlite idx_oauth_import_job_items_cursor")?;

        Ok(())
    }

    async fn ensure_column_exists(
        pool: &SqlitePool,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<()> {
        let pragma = format!("PRAGMA table_info({table})");
        let columns = sqlx::query(&pragma)
            .fetch_all(pool)
            .await
            .with_context(|| format!("failed to read sqlite schema for {table}"))?;
        let exists = columns.into_iter().any(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == column)
                .unwrap_or(false)
        });
        if exists {
            return Ok(());
        }

        let alter = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
        sqlx::query(&alter)
            .execute(pool)
            .await
            .with_context(|| format!("failed to add sqlite {table}.{column} column"))?;
        Ok(())
    }

    async fn load_error_summary(&self, job_id: Uuid) -> Result<Vec<OAuthImportErrorSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT
                COALESCE(error_code, 'unknown') AS error_code,
                COUNT(*) AS count
            FROM oauth_import_job_items
            WHERE job_id = ?1 AND status = ?2
            GROUP BY COALESCE(error_code, 'unknown')
            ORDER BY COUNT(*) DESC, error_code ASC
            LIMIT 20
            "#,
        )
        .bind(job_id.to_string())
        .bind(DB_ITEM_FAILED)
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite oauth import error summary")?;

        let mut summary = Vec::with_capacity(rows.len());
        for row in rows {
            summary.push(OAuthImportErrorSummary {
                error_code: row.try_get("error_code")?,
                count: u64::try_from(row.try_get::<i64, _>("count")?).unwrap_or_default(),
            });
        }
        Ok(summary)
    }

    async fn load_admission_counts(&self, job_id: Uuid) -> Result<OAuthImportAdmissionCounts> {
        let row = sqlx::query(
            r#"
            SELECT
                SUM(CASE WHEN admission_status = ?2 THEN 1 ELSE 0 END) AS ready_count,
                SUM(CASE WHEN admission_status = ?3 THEN 1 ELSE 0 END) AS needs_refresh_count,
                SUM(CASE WHEN admission_status = ?4 THEN 1 ELSE 0 END) AS no_quota_count,
                SUM(CASE WHEN admission_status = ?5 THEN 1 ELSE 0 END) AS failed_count
            FROM oauth_import_job_items
            WHERE job_id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .bind("ready")
        .bind("needs_refresh")
        .bind("no_quota")
        .bind("failed")
        .fetch_one(&self.pool)
        .await
        .context("failed to query sqlite oauth import admission counts")?;

        Ok(OAuthImportAdmissionCounts {
            ready: u64::try_from(
                row.try_get::<Option<i64>, _>("ready_count")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            needs_refresh: u64::try_from(
                row.try_get::<Option<i64>, _>("needs_refresh_count")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            no_quota: u64::try_from(
                row.try_get::<Option<i64>, _>("no_quota_count")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            failed: u64::try_from(
                row.try_get::<Option<i64>, _>("failed_count")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
        })
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
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("failed to query sqlite oauth import job")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let started_at = row.try_get::<Option<DateTime<Utc>>, _>("started_at")?;
        let finished_at = row.try_get::<Option<DateTime<Utc>>, _>("finished_at")?;
        let processed = u64::try_from(row.try_get::<i64, _>("processed")?).unwrap_or_default();

        Ok(OAuthImportJobSummary {
            job_id: Uuid::parse_str(&row.try_get::<String, _>("id")?)
                .context("failed to parse sqlite oauth import job id")?,
            status: parse_job_status(row.try_get::<String, _>("status")?.as_str())?,
            total: u64::try_from(row.try_get::<i64, _>("total")?).unwrap_or_default(),
            processed,
            created_count: u64::try_from(row.try_get::<i64, _>("created_count")?)
                .unwrap_or_default(),
            updated_count: u64::try_from(row.try_get::<i64, _>("updated_count")?)
                .unwrap_or_default(),
            failed_count: u64::try_from(row.try_get::<i64, _>("failed_count")?)
                .unwrap_or_default(),
            skipped_count: u64::try_from(row.try_get::<i64, _>("skipped_count")?)
                .unwrap_or_default(),
            started_at,
            finished_at,
            created_at: row.try_get("created_at")?,
            throughput_per_min: row
                .try_get::<Option<f64>, _>("throughput_per_min")?
                .or_else(|| compute_throughput_per_min(started_at, finished_at, processed)),
            error_summary: Vec::new(),
            admission_counts: self.load_admission_counts(job_id).await?,
        })
    }

    async fn recompute_counts(&self, job_id: Uuid) -> Result<(u64, u64, u64, u64, u64)> {
        let row = sqlx::query(
            r#"
            SELECT
                SUM(CASE WHEN status IN (?2, ?3, ?4) THEN 1 ELSE 0 END) AS processed,
                SUM(CASE WHEN status = ?2 THEN 1 ELSE 0 END) AS created_count,
                SUM(CASE WHEN status = ?3 THEN 1 ELSE 0 END) AS updated_count,
                SUM(CASE WHEN status = ?4 THEN 1 ELSE 0 END) AS failed_count,
                SUM(CASE WHEN status = ?5 THEN 1 ELSE 0 END) AS skipped_count
            FROM oauth_import_job_items
            WHERE job_id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .bind(DB_ITEM_CREATED)
        .bind(DB_ITEM_UPDATED)
        .bind(DB_ITEM_FAILED)
        .bind(DB_ITEM_SKIPPED)
        .fetch_one(&self.pool)
        .await
        .context("failed to recompute sqlite oauth import counts")?;

        Ok((
            u64::try_from(
                row.try_get::<Option<i64>, _>("processed")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            u64::try_from(
                row.try_get::<Option<i64>, _>("created_count")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            u64::try_from(
                row.try_get::<Option<i64>, _>("updated_count")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            u64::try_from(
                row.try_get::<Option<i64>, _>("failed_count")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            u64::try_from(
                row.try_get::<Option<i64>, _>("skipped_count")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "postgres-backend")]
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
                admission_status TEXT NULL,
                admission_source TEXT NULL,
                admission_reason TEXT NULL,
                failure_stage TEXT NULL,
                attempt_count INT NOT NULL DEFAULT 0,
                transient_retry_count INT NOT NULL DEFAULT 0,
                next_retry_at TIMESTAMPTZ NULL,
                retryable BOOLEAN NOT NULL DEFAULT false,
                terminal_reason TEXT NULL,
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
            ALTER TABLE oauth_import_job_items
            ADD COLUMN IF NOT EXISTS admission_status TEXT NULL
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to add oauth_import_job_items.admission_status column")?;

        sqlx::query(
            r#"
            ALTER TABLE oauth_import_job_items
            ADD COLUMN IF NOT EXISTS admission_source TEXT NULL
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to add oauth_import_job_items.admission_source column")?;

        sqlx::query(
            r#"
            ALTER TABLE oauth_import_job_items
            ADD COLUMN IF NOT EXISTS admission_reason TEXT NULL
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to add oauth_import_job_items.admission_reason column")?;

        sqlx::query(
            r#"
            ALTER TABLE oauth_import_job_items
            ADD COLUMN IF NOT EXISTS failure_stage TEXT NULL
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to add oauth_import_job_items.failure_stage column")?;

        sqlx::query(
            r#"
            ALTER TABLE oauth_import_job_items
            ADD COLUMN IF NOT EXISTS attempt_count INT NOT NULL DEFAULT 0
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to add oauth_import_job_items.attempt_count column")?;

        sqlx::query(
            r#"
            ALTER TABLE oauth_import_job_items
            ADD COLUMN IF NOT EXISTS transient_retry_count INT NOT NULL DEFAULT 0
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to add oauth_import_job_items.transient_retry_count column")?;

        sqlx::query(
            r#"
            ALTER TABLE oauth_import_job_items
            ADD COLUMN IF NOT EXISTS next_retry_at TIMESTAMPTZ NULL
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to add oauth_import_job_items.next_retry_at column")?;

        sqlx::query(
            r#"
            ALTER TABLE oauth_import_job_items
            ADD COLUMN IF NOT EXISTS retryable BOOLEAN NOT NULL DEFAULT false
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to add oauth_import_job_items.retryable column")?;

        sqlx::query(
            r#"
            ALTER TABLE oauth_import_job_items
            ADD COLUMN IF NOT EXISTS terminal_reason TEXT NULL
            "#,
        )
        .execute(&self.pool)
        .await
        .context("failed to add oauth_import_job_items.terminal_reason column")?;

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

    async fn load_admission_counts(&self, job_id: Uuid) -> Result<OAuthImportAdmissionCounts> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE admission_status = $2)::BIGINT AS ready_count,
                COUNT(*) FILTER (WHERE admission_status = $3)::BIGINT AS needs_refresh_count,
                COUNT(*) FILTER (WHERE admission_status = $4)::BIGINT AS no_quota_count,
                COUNT(*) FILTER (WHERE admission_status = $5)::BIGINT AS failed_count
            FROM oauth_import_job_items
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .bind("ready")
        .bind("needs_refresh")
        .bind("no_quota")
        .bind("failed")
        .fetch_one(&self.pool)
        .await
        .context("failed to query oauth import admission counts")?;

        Ok(OAuthImportAdmissionCounts {
            ready: u64::try_from(row.try_get::<i64, _>("ready_count")?).unwrap_or_default(),
            needs_refresh: u64::try_from(row.try_get::<i64, _>("needs_refresh_count")?)
                .unwrap_or_default(),
            no_quota: u64::try_from(row.try_get::<i64, _>("no_quota_count")?).unwrap_or_default(),
            failed: u64::try_from(row.try_get::<i64, _>("failed_count")?).unwrap_or_default(),
        })
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
            admission_counts: self.load_admission_counts(job_id).await?,
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

#[async_trait]
impl OAuthImportJobStore for SqliteOAuthImportJobStore {
    async fn create_job(
        &self,
        summary: OAuthImportJobSummary,
        items: Vec<PersistedImportItem>,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start create sqlite oauth import job transaction")?;

        sqlx::query(
            r#"
            INSERT INTO oauth_import_jobs (
                id,
                status,
                cancel_requested,
                total,
                processed,
                created_count,
                updated_count,
                failed_count,
                skipped_count,
                started_at,
                finished_at,
                created_at,
                throughput_per_min,
                error_summary
            )
            VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, '[]')
            "#,
        )
        .bind(summary.job_id.to_string())
        .bind(job_status_to_db(summary.status))
        .bind(i64::try_from(summary.total).unwrap_or(i64::MAX))
        .bind(i64::try_from(summary.processed).unwrap_or(i64::MAX))
        .bind(i64::try_from(summary.created_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(summary.updated_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(summary.failed_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(summary.skipped_count).unwrap_or(i64::MAX))
        .bind(summary.started_at)
        .bind(summary.finished_at)
        .bind(summary.created_at)
        .execute(tx.as_mut())
        .await
        .context("failed to insert sqlite oauth import job")?;

        for persisted in items {
            let request_json = persisted
                .request
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .context("failed to serialize sqlite import request")?;
            let raw_record_json = persisted
                .raw_record
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .context("failed to serialize sqlite raw import record")?;
            let normalized_record_json = persisted
                .normalized_record
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .context("failed to serialize sqlite normalized import record")?
                .or_else(|| request_json.clone());
            let now = Utc::now();

            sqlx::query(
                r#"
                INSERT INTO oauth_import_job_items (
                    job_id,
                    item_id,
                    source_file,
                    line_no,
                    status,
                    label,
                    email,
                    chatgpt_account_id,
                    account_id,
                    error_code,
                    error_message,
                    admission_status,
                    admission_source,
                    admission_reason,
                    failure_stage,
                    attempt_count,
                    transient_retry_count,
                    next_retry_at,
                    retryable,
                    terminal_reason,
                    request_json,
                    raw_record,
                    normalized_record,
                    retry_count,
                    created_at,
                    updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?25)
                "#,
            )
            .bind(summary.job_id.to_string())
            .bind(i64::try_from(persisted.item.item_id).unwrap_or(i64::MAX))
            .bind(persisted.item.source_file)
            .bind(i64::try_from(persisted.item.line_no).unwrap_or(i64::MAX))
            .bind(item_status_to_db(persisted.item.status))
            .bind(persisted.item.label)
            .bind(persisted.item.email)
            .bind(persisted.item.chatgpt_account_id)
            .bind(persisted.item.account_id.map(|value| value.to_string()))
            .bind(persisted.item.error_code)
            .bind(persisted.item.error_message)
            .bind(persisted.item.admission_status.map(admission_status_to_db))
            .bind(persisted.item.admission_source)
            .bind(persisted.item.admission_reason)
            .bind(persisted.item.failure_stage.map(failure_stage_to_db))
            .bind(i64::from(persisted.item.attempt_count))
            .bind(i64::from(persisted.item.transient_retry_count))
            .bind(persisted.item.next_retry_at)
            .bind(i32::from(persisted.item.retryable))
            .bind(persisted.item.terminal_reason)
            .bind(request_json)
            .bind(raw_record_json)
            .bind(normalized_record_json)
            .bind(i64::from(persisted.retry_count))
            .bind(now)
            .execute(tx.as_mut())
            .await
            .context("failed to insert sqlite oauth import job item")?;
        }

        tx.commit()
            .await
            .context("failed to commit create sqlite oauth import job transaction")?;
        Ok(())
    }

    async fn get_job_summary(&self, job_id: Uuid) -> Result<OAuthImportJobSummary> {
        let mut summary = self.load_job_row(job_id).await?;
        summary.error_summary = self.load_error_summary(job_id).await?;
        summary.throughput_per_min =
            compute_throughput_per_min(summary.started_at, summary.finished_at, summary.processed)
                .or(summary.throughput_per_min);
        Ok(summary)
    }

    async fn get_job_items(
        &self,
        job_id: Uuid,
        status: Option<OAuthImportItemStatus>,
        cursor: Option<u64>,
        limit: u64,
    ) -> Result<OAuthImportJobItemsResponse> {
        let effective_limit = limit.clamp(1, 500);
        let fetch_limit = effective_limit.saturating_add(1);
        let cursor = i64::try_from(cursor.unwrap_or(0)).unwrap_or_default();

        let rows = if let Some(status) = status {
            sqlx::query(
                r#"
                SELECT
                    item_id,
                    source_file,
                    line_no,
                    status,
                    label,
                    email,
                    chatgpt_account_id,
                    account_id,
                    error_code,
                    error_message,
                    admission_status,
                    admission_source,
                    admission_reason
                FROM oauth_import_job_items
                WHERE job_id = ?1 AND status = ?2 AND item_id > ?3
                ORDER BY item_id ASC
                LIMIT ?4
                "#,
            )
            .bind(job_id.to_string())
            .bind(item_status_to_db(status))
            .bind(cursor)
            .bind(i64::try_from(fetch_limit).unwrap_or(i64::MAX))
            .fetch_all(&self.pool)
            .await
            .context("failed to list sqlite oauth import job items by status")?
        } else {
            sqlx::query(
                r#"
                SELECT
                    item_id,
                    source_file,
                    line_no,
                    status,
                    label,
                    email,
                    chatgpt_account_id,
                    account_id,
                    error_code,
                    error_message,
                    admission_status,
                    admission_source,
                    admission_reason,
                    failure_stage,
                    attempt_count,
                    transient_retry_count,
                    next_retry_at,
                    retryable,
                    terminal_reason
                FROM oauth_import_job_items
                WHERE job_id = ?1 AND item_id > ?2
                ORDER BY item_id ASC
                LIMIT ?3
                "#,
            )
            .bind(job_id.to_string())
            .bind(cursor)
            .bind(i64::try_from(fetch_limit).unwrap_or(i64::MAX))
            .fetch_all(&self.pool)
            .await
            .context("failed to list sqlite oauth import job items")?
        };

        let mut items = Vec::new();
        for row in rows.iter().take(effective_limit as usize) {
            items.push(OAuthImportJobItem {
                item_id: u64::try_from(row.try_get::<i64, _>("item_id")?).unwrap_or_default(),
                source_file: row.try_get("source_file")?,
                line_no: u64::try_from(row.try_get::<i64, _>("line_no")?).unwrap_or_default(),
                status: parse_item_status(row.try_get::<String, _>("status")?.as_str())?,
                label: row.try_get("label")?,
                email: row.try_get("email")?,
                chatgpt_account_id: row.try_get("chatgpt_account_id")?,
                account_id: row
                    .try_get::<Option<String>, _>("account_id")?
                    .map(|raw| Uuid::parse_str(&raw))
                    .transpose()
                    .context("failed to parse sqlite oauth import item account_id")?,
                error_code: row.try_get("error_code")?,
                error_message: row.try_get("error_message")?,
                admission_status: row
                    .try_get::<Option<String>, _>("admission_status")?
                    .map(|raw| parse_admission_status(raw.as_str()))
                    .transpose()?,
                admission_source: row.try_get("admission_source")?,
                admission_reason: row.try_get("admission_reason")?,
                failure_stage: row
                    .try_get::<Option<String>, _>("failure_stage")?
                    .map(|raw| parse_failure_stage(raw.as_str()))
                    .transpose()?,
                attempt_count: u32::try_from(
                    row.try_get::<i64, _>("attempt_count")?.max(0),
                )
                .unwrap_or_default(),
                transient_retry_count: u32::try_from(
                    row.try_get::<i64, _>("transient_retry_count")?.max(0),
                )
                .unwrap_or_default(),
                next_retry_at: row.try_get("next_retry_at")?,
                retryable: row.try_get::<i32, _>("retryable")? != 0,
                terminal_reason: row.try_get("terminal_reason")?,
            });
        }

        let next_cursor = if (rows.len() as u64) > effective_limit {
            items.last().map(|item| item.item_id)
        } else {
            None
        };

        Ok(OAuthImportJobItemsResponse { items, next_cursor })
    }

    async fn start_job(&self, job_id: Uuid, limit: usize) -> Result<Vec<ImportJobTask>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start sqlite oauth import start-job transaction")?;

        let job_row = sqlx::query(
            r#"
            SELECT status, cancel_requested
            FROM oauth_import_jobs
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to query sqlite oauth import job for start")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status = job_row.try_get::<String, _>("status")?;
        let cancel_requested = job_row.try_get::<i64, _>("cancel_requested")? != 0;
        let resuming_running_job = status == DB_STATUS_RUNNING;

        if cancel_requested {
            tx.commit()
                .await
                .context("failed to commit cancelled sqlite start-job transaction")?;
            return Ok(Vec::new());
        }
        if matches!(
            status.as_str(),
            DB_STATUS_PAUSED | DB_STATUS_COMPLETED | DB_STATUS_FAILED | DB_STATUS_CANCELLED
        ) {
            tx.commit()
                .await
                .context("failed to commit terminal sqlite start-job transaction")?;
            return Ok(Vec::new());
        }

        if resuming_running_job {
            sqlx::query(
                r#"
                UPDATE oauth_import_job_items
                SET status = ?2,
                    updated_at = ?3
                WHERE job_id = ?1
                  AND status = ?4
                "#,
            )
            .bind(job_id.to_string())
            .bind(DB_ITEM_PENDING)
            .bind(Utc::now())
            .bind(DB_ITEM_PROCESSING)
            .execute(tx.as_mut())
            .await
            .context("failed to reset sqlite processing items when resuming running job")?;
        }

        let started_at = if status == DB_STATUS_RUNNING {
            None
        } else {
            Some(Utc::now())
        };
        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET status = ?2,
                started_at = COALESCE(started_at, ?3),
                finished_at = NULL
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .bind(DB_STATUS_RUNNING)
        .bind(started_at)
        .execute(tx.as_mut())
        .await
        .context("failed to mark sqlite oauth import job running")?;

        let rows = sqlx::query(
            r#"
            SELECT item_id, request_json
            FROM oauth_import_job_items
            WHERE job_id = ?1
              AND status = ?2
            ORDER BY item_id ASC
            LIMIT ?3
            "#,
        )
        .bind(job_id.to_string())
        .bind(DB_ITEM_PENDING)
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(tx.as_mut())
        .await
        .context("failed to load sqlite pending oauth import items")?;

        let now = Utc::now();
        let mut tasks = Vec::with_capacity(rows.len());
        for row in rows {
            let item_id = u64::try_from(row.try_get::<i64, _>("item_id")?).unwrap_or_default();
            let request_json = row
                .try_get::<Option<String>, _>("request_json")?
                .ok_or_else(|| anyhow!("missing request_json for processing item"))?;
            let request_value: Value = serde_json::from_str(&request_json)
                .context("failed to decode sqlite import request json")?;
            let request: ImportTaskRequest = serde_json::from_value(request_value.clone())
                .or_else(|_| {
                    serde_json::from_value::<ImportOAuthRefreshTokenRequest>(request_value)
                        .map(ImportTaskRequest::OAuthRefresh)
                })
                .context("failed to decode sqlite import request payload")?;

            sqlx::query(
                r#"
                UPDATE oauth_import_job_items
                SET status = ?3,
                    updated_at = ?4
                WHERE job_id = ?1
                  AND item_id = ?2
                  AND status = ?5
                "#,
            )
            .bind(job_id.to_string())
            .bind(i64::try_from(item_id).unwrap_or(i64::MAX))
            .bind(DB_ITEM_PROCESSING)
            .bind(now)
            .bind(DB_ITEM_PENDING)
            .execute(tx.as_mut())
            .await
            .context("failed to mark sqlite oauth import item processing")?;

            tasks.push(ImportJobTask { item_id, request });
        }

        tx.commit()
            .await
            .context("failed to commit sqlite start-job transaction")?;

        Ok(tasks)
    }

    async fn mark_item_success(
        &self,
        job_id: Uuid,
        item_id: u64,
        outcome: &ImportTaskSuccess,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start sqlite mark_item_success transaction")?;

        let status = if outcome.created {
            DB_ITEM_CREATED
        } else {
            DB_ITEM_UPDATED
        };
        let now = Utc::now();
        let updated = sqlx::query(
            r#"
            UPDATE oauth_import_job_items
            SET
                status = ?3,
                account_id = ?4,
                chatgpt_account_id = ?5,
                admission_status = ?6,
                admission_source = ?7,
                admission_reason = ?8,
                failure_stage = ?9,
                attempt_count = ?10,
                transient_retry_count = ?11,
                next_retry_at = ?12,
                retryable = ?13,
                terminal_reason = ?14,
                error_code = NULL,
                error_message = NULL,
                updated_at = ?15
            WHERE job_id = ?1
              AND item_id = ?2
              AND status = ?16
            "#,
        )
        .bind(job_id.to_string())
        .bind(i64::try_from(item_id).unwrap_or(i64::MAX))
        .bind(status)
        .bind(outcome.account_id.map(|value| value.to_string()))
        .bind(outcome.chatgpt_account_id.clone())
        .bind(outcome.admission_status.map(admission_status_to_db))
        .bind(outcome.admission_source.clone())
        .bind(outcome.admission_reason.clone())
        .bind(outcome.failure_stage.map(failure_stage_to_db))
        .bind(i64::from(outcome.attempt_count))
        .bind(i64::from(outcome.transient_retry_count))
        .bind(outcome.next_retry_at)
        .bind(i32::from(outcome.retryable))
        .bind(outcome.terminal_reason.clone())
        .bind(now)
        .bind(DB_ITEM_PROCESSING)
        .execute(tx.as_mut())
        .await
        .context("failed to update sqlite oauth import item success")?
        .rows_affected();

        if updated > 0 {
            let counter_col = if outcome.created {
                "created_count"
            } else {
                "updated_count"
            };
            let sql = format!(
                "UPDATE oauth_import_jobs SET processed = processed + 1, {counter_col} = {counter_col} + 1 WHERE id = ?1"
            );
            sqlx::query(&sql)
                .bind(job_id.to_string())
                .execute(tx.as_mut())
                .await
                .context("failed to increment sqlite oauth import success counters")?;
        }

        tx.commit()
            .await
            .context("failed to commit sqlite mark_item_success transaction")?;
        Ok(())
    }

    async fn mark_item_failed(
        &self,
        job_id: Uuid,
        item_id: u64,
        error_code: &str,
        error_message: &str,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start sqlite mark_item_failed transaction")?;
        let now = Utc::now();
        let updated = sqlx::query(
            r#"
            UPDATE oauth_import_job_items
            SET
                status = ?3,
                error_code = ?4,
                error_message = ?5,
                admission_status = ?6,
                admission_source = NULL,
                admission_reason = ?7,
                failure_stage = NULL,
                attempt_count = 0,
                transient_retry_count = 0,
                next_retry_at = NULL,
                retryable = 0,
                terminal_reason = ?7,
                updated_at = ?8
            WHERE job_id = ?1
              AND item_id = ?2
              AND status = ?9
            "#,
        )
        .bind(job_id.to_string())
        .bind(i64::try_from(item_id).unwrap_or(i64::MAX))
        .bind(DB_ITEM_FAILED)
        .bind(error_code)
        .bind(error_message)
        .bind("failed")
        .bind(error_code)
        .bind(now)
        .bind(DB_ITEM_PROCESSING)
        .execute(tx.as_mut())
        .await
        .context("failed to update sqlite oauth import item failed")?
        .rows_affected();

        if updated > 0 {
            sqlx::query(
                r#"
                UPDATE oauth_import_jobs
                SET processed = processed + 1,
                    failed_count = failed_count + 1
                WHERE id = ?1
                "#,
            )
            .bind(job_id.to_string())
            .execute(tx.as_mut())
            .await
            .context("failed to increment sqlite oauth import failure counters")?;
        }

        tx.commit()
            .await
            .context("failed to commit sqlite mark_item_failed transaction")?;
        Ok(())
    }

    async fn finish_job(&self, job_id: Uuid) -> Result<OAuthImportJobSummary> {
        let row = sqlx::query(
            r#"
            SELECT cancel_requested, started_at
            FROM oauth_import_jobs
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("failed to query sqlite oauth import job for finish")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let cancel_requested = row.try_get::<i64, _>("cancel_requested")? != 0;
        let started_at = row.try_get::<Option<DateTime<Utc>>, _>("started_at")?;
        let now = Utc::now();

        if cancel_requested {
            sqlx::query(
                r#"
                UPDATE oauth_import_job_items
                SET status = ?2,
                    updated_at = ?3
                WHERE job_id = ?1
                  AND status IN (?4, ?5)
                "#,
            )
            .bind(job_id.to_string())
            .bind(DB_ITEM_CANCELLED)
            .bind(now)
            .bind(DB_ITEM_PENDING)
            .bind(DB_ITEM_PROCESSING)
            .execute(&self.pool)
            .await
            .context("failed to mark sqlite pending/processing items cancelled")?;
        }

        let (processed, created_count, updated_count, failed_count, skipped_count) =
            self.recompute_counts(job_id).await?;
        let error_summary = self.load_error_summary(job_id).await?;
        let status = if cancel_requested {
            OAuthImportJobStatus::Cancelled
        } else if failed_count > 0 {
            OAuthImportJobStatus::Failed
        } else {
            OAuthImportJobStatus::Completed
        };
        let throughput = compute_throughput_per_min(started_at, Some(now), processed);
        let error_summary_json = serde_json::to_string(&error_summary)
            .context("failed to encode sqlite oauth import error summary")?;

        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET
                status = ?2,
                processed = ?3,
                created_count = ?4,
                updated_count = ?5,
                failed_count = ?6,
                skipped_count = ?7,
                finished_at = ?8,
                throughput_per_min = ?9,
                error_summary = ?10
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .bind(job_status_to_db(status))
        .bind(i64::try_from(processed).unwrap_or(i64::MAX))
        .bind(i64::try_from(created_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(updated_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(failed_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(skipped_count).unwrap_or(i64::MAX))
        .bind(now)
        .bind(throughput)
        .bind(error_summary_json)
        .execute(&self.pool)
        .await
        .context("failed to update finished sqlite oauth import job")?;

        self.get_job_summary(job_id).await
    }

    async fn pause_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let row = sqlx::query(
            r#"
            SELECT status, cancel_requested
            FROM oauth_import_jobs
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("failed to query sqlite oauth import job for pause")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status = row.try_get::<String, _>("status")?;
        let cancel_requested = row.try_get::<i64, _>("cancel_requested")? != 0;
        let accepted =
            !cancel_requested && matches!(status.as_str(), DB_STATUS_QUEUED | DB_STATUS_RUNNING);

        if accepted {
            sqlx::query(
                r#"
                UPDATE oauth_import_jobs
                SET status = ?2,
                    finished_at = NULL
                WHERE id = ?1
                "#,
            )
            .bind(job_id.to_string())
            .bind(DB_STATUS_PAUSED)
            .execute(&self.pool)
            .await
            .context("failed to pause sqlite oauth import job")?;
        }

        Ok(OAuthImportJobActionResponse { job_id, accepted })
    }

    async fn resume_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let row = sqlx::query(
            r#"
            SELECT status, cancel_requested
            FROM oauth_import_jobs
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("failed to query sqlite oauth import job for resume")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status = row.try_get::<String, _>("status")?;
        let cancel_requested = row.try_get::<i64, _>("cancel_requested")? != 0;
        let accepted = !cancel_requested && status == DB_STATUS_PAUSED;
        if accepted {
            let now = Utc::now();
            let mut tx = self
                .pool
                .begin()
                .await
                .context("failed to start sqlite resume_job transaction")?;

            sqlx::query(
                r#"
                UPDATE oauth_import_job_items
                SET status = ?2,
                    updated_at = ?3
                WHERE job_id = ?1
                  AND status = ?4
                "#,
            )
            .bind(job_id.to_string())
            .bind(DB_ITEM_PENDING)
            .bind(now)
            .bind(DB_ITEM_PROCESSING)
            .execute(tx.as_mut())
            .await
            .context("failed to reset sqlite processing items on resume")?;

            sqlx::query(
                r#"
                UPDATE oauth_import_jobs
                SET status = ?2,
                    cancel_requested = 0,
                    finished_at = NULL,
                    throughput_per_min = NULL
                WHERE id = ?1
                "#,
            )
            .bind(job_id.to_string())
            .bind(DB_STATUS_QUEUED)
            .execute(tx.as_mut())
            .await
            .context("failed to resume sqlite oauth import job")?;

            tx.commit()
                .await
                .context("failed to commit sqlite resume_job transaction")?;
        }

        Ok(OAuthImportJobActionResponse { job_id, accepted })
    }

    async fn cancel_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET cancel_requested = 1,
                status = CASE WHEN status IN (?2, ?3) THEN ?4 ELSE status END,
                finished_at = CASE WHEN status IN (?2, ?3) THEN ?5 ELSE finished_at END
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .bind(DB_STATUS_QUEUED)
        .bind(DB_STATUS_PAUSED)
        .bind(DB_STATUS_CANCELLED)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("failed to cancel sqlite oauth import job")?;

        Ok(OAuthImportJobActionResponse {
            job_id,
            accepted: true,
        })
    }

    async fn retry_failed(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let row = sqlx::query(
            r#"
            SELECT status
            FROM oauth_import_jobs
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("failed to query sqlite oauth import job for retry")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status = row.try_get::<String, _>("status")?;
        if status == DB_STATUS_RUNNING {
            return Ok(OAuthImportJobActionResponse {
                job_id,
                accepted: false,
            });
        }

        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start sqlite retry_failed transaction")?;

        sqlx::query(
            r#"
            UPDATE oauth_import_job_items
            SET status = ?2,
                error_code = NULL,
                error_message = NULL,
                account_id = NULL,
                admission_status = NULL,
                admission_source = NULL,
                admission_reason = NULL,
                failure_stage = NULL,
                attempt_count = 0,
                transient_retry_count = 0,
                next_retry_at = NULL,
                retryable = 0,
                terminal_reason = NULL,
                retry_count = retry_count + 1,
                updated_at = ?4
            WHERE job_id = ?1
              AND status = ?3
              AND request_json IS NOT NULL
            "#,
        )
        .bind(job_id.to_string())
        .bind(DB_ITEM_PENDING)
        .bind(DB_ITEM_FAILED)
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to reset sqlite failed oauth import items")?;

        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET
                status = ?2,
                cancel_requested = 0,
                started_at = NULL,
                finished_at = NULL,
                throughput_per_min = NULL,
                error_summary = '[]'
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .bind(DB_STATUS_QUEUED)
        .execute(tx.as_mut())
        .await
        .context("failed to reset sqlite oauth import job to queued")?;

        tx.commit()
            .await
            .context("failed to commit sqlite retry_failed transaction")?;

        let (processed, created_count, updated_count, failed_count, skipped_count) =
            self.recompute_counts(job_id).await?;

        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET
                processed = ?2,
                created_count = ?3,
                updated_count = ?4,
                failed_count = ?5,
                skipped_count = ?6
            WHERE id = ?1
            "#,
        )
        .bind(job_id.to_string())
        .bind(i64::try_from(processed).unwrap_or(i64::MAX))
        .bind(i64::try_from(created_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(updated_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(failed_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(skipped_count).unwrap_or(i64::MAX))
        .execute(&self.pool)
        .await
        .context("failed to refresh sqlite counters after retry_failed")?;

        Ok(OAuthImportJobActionResponse {
            job_id,
            accepted: true,
        })
    }

    async fn recoverable_job_ids(&self) -> Result<Vec<Uuid>> {
        let rows = sqlx::query(
            r#"
            SELECT id
            FROM oauth_import_jobs
            WHERE status IN (?1, ?2)
              AND cancel_requested = 0
            ORDER BY created_at ASC
            "#,
        )
        .bind(DB_STATUS_QUEUED)
        .bind(DB_STATUS_RUNNING)
        .fetch_all(&self.pool)
        .await
        .context("failed to load sqlite recoverable oauth import jobs")?;

        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            ids.push(
                Uuid::parse_str(&row.try_get::<String, _>("id")?)
                    .context("failed to parse sqlite recoverable oauth import job id")?,
            );
        }
        Ok(ids)
    }
}

#[cfg(feature = "postgres-backend")]
include!("import_jobs/store_impl.rs");
include!("import_jobs/manager_impl.rs");

#[cfg(test)]
mod sqlite_store_tests {
    use super::*;
    use sqlx_core::pool::PoolOptions;

    fn temp_sqlite_url(prefix: &str) -> String {
        let mut path = std::env::temp_dir();
        path.push(format!("{prefix}-{}.sqlite", Uuid::new_v4()));
        format!("sqlite://{}?mode=rwc", path.display())
    }

    fn sample_summary(job_id: Uuid) -> OAuthImportJobSummary {
        OAuthImportJobSummary {
            job_id,
            status: OAuthImportJobStatus::Queued,
            total: 2,
            processed: 0,
            created_count: 0,
            updated_count: 0,
            failed_count: 0,
            skipped_count: 0,
            started_at: None,
            finished_at: None,
            created_at: Utc::now(),
            throughput_per_min: None,
            error_summary: Vec::new(),
            admission_counts: OAuthImportAdmissionCounts::default(),
        }
    }

    fn sample_items() -> Vec<PersistedImportItem> {
        vec![
            PersistedImportItem {
                item: OAuthImportJobItem {
                    item_id: 1,
                    source_file: "sample.jsonl".to_string(),
                    line_no: 1,
                    status: OAuthImportItemStatus::Pending,
                    label: "sqlite-import-1".to_string(),
                    email: Some("one@example.com".to_string()),
                    chatgpt_account_id: Some("acct_sqlite_1".to_string()),
                    account_id: None,
                    error_code: None,
                    error_message: None,
                    admission_status: None,
                    admission_source: None,
                    admission_reason: None,
                    failure_stage: None,
                    attempt_count: 0,
                    transient_retry_count: 0,
                    next_retry_at: None,
                    retryable: false,
                    terminal_reason: None,
                },
                request: Some(ImportTaskRequest::OAuthRefresh(ImportOAuthRefreshTokenRequest {
                    label: "sqlite-import-1".to_string(),
                    base_url: DEFAULT_BASE_URL.to_string(),
                    refresh_token: "rt-sqlite-import-1".to_string(),
                    fallback_access_token: Some("ak-sqlite-import-1".to_string()),
                    fallback_token_expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
                    chatgpt_account_id: Some("acct_sqlite_1".to_string()),
                    mode: Some(UpstreamMode::CodexOauth),
                    enabled: Some(true),
                    priority: Some(100),
                    chatgpt_plan_type: Some("free".to_string()),
                    source_type: Some("codex".to_string()),
                })),
                raw_record: Some(serde_json::json!({"raw": 1})),
                normalized_record: None,
                retry_count: 0,
            },
            PersistedImportItem {
                item: OAuthImportJobItem {
                    item_id: 2,
                    source_file: "sample.jsonl".to_string(),
                    line_no: 2,
                    status: OAuthImportItemStatus::Pending,
                    label: "sqlite-import-2".to_string(),
                    email: Some("two@example.com".to_string()),
                    chatgpt_account_id: Some("acct_sqlite_2".to_string()),
                    account_id: None,
                    error_code: None,
                    error_message: None,
                    admission_status: None,
                    admission_source: None,
                    admission_reason: None,
                    failure_stage: None,
                    attempt_count: 0,
                    transient_retry_count: 0,
                    next_retry_at: None,
                    retryable: false,
                    terminal_reason: None,
                },
                request: Some(ImportTaskRequest::OAuthRefresh(ImportOAuthRefreshTokenRequest {
                    label: "sqlite-import-2".to_string(),
                    base_url: DEFAULT_BASE_URL.to_string(),
                    refresh_token: "rt-sqlite-import-2".to_string(),
                    fallback_access_token: None,
                    fallback_token_expires_at: None,
                    chatgpt_account_id: Some("acct_sqlite_2".to_string()),
                    mode: Some(UpstreamMode::CodexOauth),
                    enabled: Some(true),
                    priority: Some(100),
                    chatgpt_plan_type: Some("plus".to_string()),
                    source_type: Some("codex".to_string()),
                })),
                raw_record: Some(serde_json::json!({"raw": 2})),
                normalized_record: Some(serde_json::json!({"normalized": 2})),
                retry_count: 0,
            },
        ]
    }

    #[tokio::test]
    async fn sqlite_import_job_store_persists_summary_and_items_across_reopen() {
        let database_url = temp_sqlite_url("oauth-import-job-store");
        let pool = PoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("connect sqlite import job pool");
        let store = SqliteOAuthImportJobStore::new(pool.clone())
            .await
            .expect("create sqlite import job store");
        let job_id = Uuid::new_v4();

        store
            .create_job(sample_summary(job_id), sample_items())
            .await
            .expect("create sqlite import job");
        drop(store);
        drop(pool);

        let reopened_pool = PoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("reconnect sqlite import job pool");
        let reopened = SqliteOAuthImportJobStore::new(reopened_pool)
            .await
            .expect("reopen sqlite import job store");

        let summary = reopened
            .get_job_summary(job_id)
            .await
            .expect("load sqlite job summary after reopen");
        assert_eq!(summary.job_id, job_id);
        assert_eq!(summary.status, OAuthImportJobStatus::Queued);
        assert_eq!(summary.total, 2);

        let items = reopened
            .get_job_items(job_id, None, None, 50)
            .await
            .expect("load sqlite job items after reopen");
        assert_eq!(items.items.len(), 2);
        assert_eq!(items.items[0].label, "sqlite-import-1");
        assert_eq!(items.items[1].chatgpt_account_id.as_deref(), Some("acct_sqlite_2"));
    }

    #[tokio::test]
    async fn sqlite_import_job_store_recovers_running_jobs_after_reopen() {
        let database_url = temp_sqlite_url("oauth-import-job-store-recovery");
        let pool = PoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("connect sqlite import job recovery pool");
        let store = SqliteOAuthImportJobStore::new(pool.clone())
            .await
            .expect("create sqlite import job recovery store");
        let job_id = Uuid::new_v4();

        store
            .create_job(sample_summary(job_id), sample_items())
            .await
            .expect("create sqlite recoverable job");
        let started = store
            .start_job(job_id, 1)
            .await
            .expect("start sqlite recoverable job");
        assert_eq!(started.len(), 1);
        drop(store);
        drop(pool);

        let reopened_pool = PoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("reconnect sqlite import job recovery pool");
        let reopened = SqliteOAuthImportJobStore::new(reopened_pool)
            .await
            .expect("reopen sqlite import job recovery store");

        let recoverable = reopened
            .recoverable_job_ids()
            .await
            .expect("load sqlite recoverable job ids");
        assert_eq!(recoverable, vec![job_id]);
    }
}
