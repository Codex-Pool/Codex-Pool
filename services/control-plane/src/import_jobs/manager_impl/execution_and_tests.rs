async fn execute_import_with_retry(
    data_store: Arc<dyn ControlPlaneStore>,
    request: ImportTaskRequest,
) -> Result<ImportTaskSuccess> {
    const MAX_ATTEMPTS: u32 = 3;

    let mut attempt = 0_u32;
    loop {
        attempt = attempt.saturating_add(1);
        let result: Result<ImportTaskSuccess> = match request.clone() {
            ImportTaskRequest::OAuthRefresh(req) => {
                let created = data_store.queue_oauth_refresh_token(req.clone()).await?;
                let admission = find_inventory_admission_for_request(data_store.clone(), &req).await?;
                Ok(ImportTaskSuccess {
                    created,
                    account_id: None,
                    chatgpt_account_id: req.chatgpt_account_id,
                    admission_status: admission
                        .as_ref()
                        .map(|record| record.vault_status),
                    admission_source: admission
                        .as_ref()
                        .and_then(|record| record.admission_source.clone()),
                    admission_reason: admission.as_ref().and_then(|record| {
                        record
                            .admission_error_code
                            .clone()
                            .or_else(|| record.admission_error_message.clone())
                    }),
                    failure_stage: admission.as_ref().and_then(|record| record.failure_stage),
                    attempt_count: admission
                        .as_ref()
                        .map(|record| record.attempt_count)
                        .unwrap_or(0),
                    transient_retry_count: admission
                        .as_ref()
                        .map(|record| record.transient_retry_count)
                        .unwrap_or(0),
                    next_retry_at: admission
                        .as_ref()
                        .and_then(|record| record.next_retry_at),
                    retryable: admission
                        .as_ref()
                        .map(|record| record.retryable)
                        .unwrap_or(false),
                    terminal_reason: admission
                        .as_ref()
                        .and_then(|record| record.terminal_reason.clone()),
                })
            }
            ImportTaskRequest::OneTimeAccessToken(req) => {
                let upserted = data_store.upsert_one_time_session_account(req).await?;
                Ok(ImportTaskSuccess {
                    created: upserted.created,
                    account_id: Some(upserted.account.id),
                    chatgpt_account_id: upserted.account.chatgpt_account_id,
                    admission_status: None,
                    admission_source: None,
                    admission_reason: None,
                    failure_stage: None,
                    attempt_count: 0,
                    transient_retry_count: 0,
                    next_retry_at: None,
                    retryable: false,
                    terminal_reason: None,
                })
            }
            ImportTaskRequest::ManualRefreshAccount(req) => {
                data_store.refresh_oauth_account(req.account_id).await?;
                let account = data_store
                    .list_upstream_accounts()
                    .await?
                    .into_iter()
                    .find(|item| item.id == req.account_id)
                    .ok_or_else(|| anyhow!("account not found"))?;
                Ok(ImportTaskSuccess {
                    created: false,
                    account_id: Some(account.id),
                    chatgpt_account_id: account.chatgpt_account_id,
                    admission_status: None,
                    admission_source: None,
                    admission_reason: None,
                    failure_stage: None,
                    attempt_count: 0,
                    transient_retry_count: 0,
                    next_retry_at: None,
                    retryable: false,
                    terminal_reason: None,
                })
            }
        };
        match result {
            Ok(upserted) => return Ok(upserted),
            Err(err) => {
                let message = err.to_string();
                if attempt >= MAX_ATTEMPTS || !is_retryable_import_error(&message) {
                    return Err(anyhow!(message));
                }

                let backoff_ms = 1_000_u64.saturating_mul(1_u64 << (attempt - 1));
                sleep(Duration::from_millis(backoff_ms.min(8_000))).await;
            }
        }
    }
}

async fn find_inventory_admission_for_request(
    data_store: Arc<dyn ControlPlaneStore>,
    req: &ImportOAuthRefreshTokenRequest,
) -> Result<Option<OAuthInventoryRecord>> {
    let records = data_store.oauth_inventory_records().await?;
    Ok(records
        .into_iter()
        .filter(|record| {
            record.label == req.label
                && record.chatgpt_account_id == req.chatgpt_account_id
                && record.has_access_token_fallback == req
                    .fallback_access_token
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|value| !value.is_empty())
        })
        .max_by(|left, right| {
            left.updated_at
                .cmp(&right.updated_at)
                .then_with(|| left.id.cmp(&right.id))
        }))
}

fn is_retryable_import_error(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();

    if lowered.contains("invalid refresh token") {
        return false;
    }

    [
        "timeout",
        "timed out",
        "connection reset",
        "connection refused",
        "connection closed",
        "transport error",
        "temporarily unavailable",
        "503",
        "502",
        "500",
        "429",
        "rate limit",
        "too many requests",
    ]
    .iter()
    .any(|flag| lowered.contains(flag))
}

fn job_status_to_db(status: OAuthImportJobStatus) -> &'static str {
    match status {
        OAuthImportJobStatus::Queued => DB_STATUS_QUEUED,
        OAuthImportJobStatus::Running => DB_STATUS_RUNNING,
        OAuthImportJobStatus::Paused => DB_STATUS_PAUSED,
        OAuthImportJobStatus::Completed => DB_STATUS_COMPLETED,
        OAuthImportJobStatus::Failed => DB_STATUS_FAILED,
        OAuthImportJobStatus::Cancelled => DB_STATUS_CANCELLED,
    }
}

fn parse_job_status(raw: &str) -> Result<OAuthImportJobStatus> {
    match raw {
        DB_STATUS_QUEUED => Ok(OAuthImportJobStatus::Queued),
        DB_STATUS_RUNNING => Ok(OAuthImportJobStatus::Running),
        DB_STATUS_PAUSED => Ok(OAuthImportJobStatus::Paused),
        DB_STATUS_COMPLETED => Ok(OAuthImportJobStatus::Completed),
        DB_STATUS_FAILED => Ok(OAuthImportJobStatus::Failed),
        DB_STATUS_CANCELLED => Ok(OAuthImportJobStatus::Cancelled),
        _ => Err(anyhow!("unsupported oauth import job status: {raw}")),
    }
}

fn item_status_to_db(status: OAuthImportItemStatus) -> &'static str {
    match status {
        OAuthImportItemStatus::Pending => DB_ITEM_PENDING,
        OAuthImportItemStatus::Processing => DB_ITEM_PROCESSING,
        OAuthImportItemStatus::Created => DB_ITEM_CREATED,
        OAuthImportItemStatus::Updated => DB_ITEM_UPDATED,
        OAuthImportItemStatus::Failed => DB_ITEM_FAILED,
        OAuthImportItemStatus::Skipped => DB_ITEM_SKIPPED,
        OAuthImportItemStatus::Cancelled => DB_ITEM_CANCELLED,
    }
}

fn parse_item_status(raw: &str) -> Result<OAuthImportItemStatus> {
    match raw {
        DB_ITEM_PENDING => Ok(OAuthImportItemStatus::Pending),
        DB_ITEM_PROCESSING => Ok(OAuthImportItemStatus::Processing),
        DB_ITEM_CREATED => Ok(OAuthImportItemStatus::Created),
        DB_ITEM_UPDATED => Ok(OAuthImportItemStatus::Updated),
        DB_ITEM_FAILED => Ok(OAuthImportItemStatus::Failed),
        DB_ITEM_SKIPPED => Ok(OAuthImportItemStatus::Skipped),
        DB_ITEM_CANCELLED => Ok(OAuthImportItemStatus::Cancelled),
        _ => Err(anyhow!("unsupported oauth import item status: {raw}")),
    }
}

fn admission_status_to_db(status: OAuthVaultRecordStatus) -> &'static str {
    match status {
        OAuthVaultRecordStatus::Queued => "queued",
        OAuthVaultRecordStatus::Ready => "ready",
        OAuthVaultRecordStatus::NeedsRefresh => "needs_refresh",
        OAuthVaultRecordStatus::NoQuota => "no_quota",
        OAuthVaultRecordStatus::Failed => "failed",
    }
}

fn parse_admission_status(raw: &str) -> Result<OAuthVaultRecordStatus> {
    match raw {
        "queued" => Ok(OAuthVaultRecordStatus::Queued),
        "ready" => Ok(OAuthVaultRecordStatus::Ready),
        "needs_refresh" => Ok(OAuthVaultRecordStatus::NeedsRefresh),
        "no_quota" => Ok(OAuthVaultRecordStatus::NoQuota),
        "failed" => Ok(OAuthVaultRecordStatus::Failed),
        _ => Err(anyhow!("unsupported oauth import admission status: {raw}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_import_failure_code, parse_file_records, CreateOAuthImportJobOptions,
        ImportTaskRequest, ImportTaskSuccess, ImportUploadFile, InMemoryOAuthImportJobStore,
        OAuthImportJobStore, PersistedImportItem,
    };
    use bytes::Bytes;
    use chrono::Utc;
    use crate::contracts::{
        ImportOAuthRefreshTokenRequest, OAuthImportAdmissionCounts, OAuthImportItemStatus,
        OAuthImportJobItem, OAuthImportJobStatus, OAuthImportJobSummary,
    };
    use codex_pool_core::model::UpstreamMode;
    use crate::import_jobs::ImportCredentialMode;
    use uuid::Uuid;

    fn build_in_memory_job_state(job_id: Uuid) -> (OAuthImportJobSummary, Vec<PersistedImportItem>) {
        let summary = OAuthImportJobSummary {
            job_id,
            status: OAuthImportJobStatus::Queued,
            total: 1,
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
        };
        let item = PersistedImportItem {
            item: OAuthImportJobItem {
                item_id: 1,
                source_file: "pause-resume.jsonl".to_string(),
                line_no: 1,
                status: OAuthImportItemStatus::Pending,
                label: "pause-resume".to_string(),
                email: None,
                chatgpt_account_id: Some("acct-pause-resume".to_string()),
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
                label: "pause-resume".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-pause-resume".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct-pause-resume".to_string()),
                mode: Some(UpstreamMode::ChatGptSession),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: None,
            })),
            raw_record: None,
            normalized_record: None,
            retry_count: 0,
        };
        (summary, vec![item])
    }

    #[test]
    fn parse_record_reads_chatgpt_account_id_from_token_info() {
        let file = ImportUploadFile {
            file_name: "accounts.jsonl".to_string(),
            content: Bytes::from(
                r#"{"refresh_token":"rt_test","token_info":{"chatgpt_account_id":"acct_from_token_info"}}"#
                    .to_string(),
            ),
        };

        let items = parse_file_records(&file, &CreateOAuthImportJobOptions::default())
            .expect("parse file records");
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(
            item.item.chatgpt_account_id.as_deref(),
            Some("acct_from_token_info")
        );
        assert_eq!(
            item.request
                .as_ref()
                .and_then(|req| match req {
                    ImportTaskRequest::OAuthRefresh(req) => req.chatgpt_account_id.as_deref(),
                    ImportTaskRequest::OneTimeAccessToken(req) => req.chatgpt_account_id.as_deref(),
                    ImportTaskRequest::ManualRefreshAccount(_) => None,
                }),
            Some("acct_from_token_info")
        );
    }

    #[test]
    fn parse_record_supports_sub2api_and_cliproxy_aliases() {
        let file = ImportUploadFile {
            file_name: "accounts.json".to_string(),
            content: Bytes::from(
                r#"[
                  {
                    "refreshToken":"rt_alias_1",
                    "chatgptAccountId":"acct_alias_1",
                    "baseUrl":"https://chatgpt.com/backend-api/codex",
                    "name":"alias-account-1",
                    "is_enabled":"true",
                    "weight":"120"
                  },
                  {
                    "token_info":{"refresh_token":"rt_alias_2","chatgpt_account_id":"acct_alias_2"},
                    "mail":"alias2@example.com"
                  }
                ]"#,
            ),
        };

        let items = parse_file_records(&file, &CreateOAuthImportJobOptions::default())
            .expect("parse file records");
        assert_eq!(items.len(), 2);

        let first = &items[0];
        let first_req = match first.request.as_ref().expect("first request") {
            ImportTaskRequest::OAuthRefresh(req) => req,
            ImportTaskRequest::OneTimeAccessToken(_) => panic!("expected oauth refresh request"),
            ImportTaskRequest::ManualRefreshAccount(_) => panic!("expected oauth refresh request"),
        };
        assert_eq!(first_req.refresh_token, "rt_alias_1");
        assert_eq!(first_req.chatgpt_account_id.as_deref(), Some("acct_alias_1"));
        assert_eq!(first_req.priority, Some(120));
        assert_eq!(first_req.enabled, Some(true));
        assert_eq!(first_req.label, "alias-account-1");

        let second = &items[1];
        let second_req = match second.request.as_ref().expect("second request") {
            ImportTaskRequest::OAuthRefresh(req) => req,
            ImportTaskRequest::OneTimeAccessToken(_) => panic!("expected oauth refresh request"),
            ImportTaskRequest::ManualRefreshAccount(_) => panic!("expected oauth refresh request"),
        };
        assert_eq!(second_req.refresh_token, "rt_alias_2");
        assert_eq!(second_req.chatgpt_account_id.as_deref(), Some("acct_alias_2"));
        assert_eq!(second.item.email.as_deref(), Some("alias2@example.com"));
    }

    #[test]
    fn parse_record_supports_codex_one_time_access_token_with_plan_type() {
        let file = ImportUploadFile {
            file_name: "codex-one-time.json".to_string(),
            content: Bytes::from(
                r#"{
                  "type":"codex",
                  "email":"codex@example.com",
                  "access_token":"ak_test",
                  "account_id":"acct_codex_1",
                  "exp": 1893456000,
                  "https://api.openai.com/auth":{"chatgpt_plan_type":"free"}
                }"#,
            ),
        };

        let items = parse_file_records(&file, &CreateOAuthImportJobOptions::default())
            .expect("parse file records");
        assert_eq!(items.len(), 1);

        let req = match items[0].request.as_ref().expect("request") {
            ImportTaskRequest::OneTimeAccessToken(req) => req,
            ImportTaskRequest::OAuthRefresh(_) => panic!("expected one-time request"),
            ImportTaskRequest::ManualRefreshAccount(_) => panic!("expected one-time request"),
        };

        assert_eq!(req.mode, UpstreamMode::CodexOauth);
        assert_eq!(req.chatgpt_plan_type.as_deref(), Some("free"));
        assert_eq!(req.source_type.as_deref(), Some("codex"));
        assert_eq!(req.chatgpt_account_id.as_deref(), Some("acct_codex_1"));
        assert!(req.token_expires_at.is_some());
    }

    #[test]
    fn parse_record_keeps_access_token_as_refresh_fallback_when_both_are_present() {
        let file = ImportUploadFile {
            file_name: "codex-refresh-with-fallback.json".to_string(),
            content: Bytes::from(
                r#"{
                  "type":"codex",
                  "email":"codex-both@example.com",
                  "refresh_token":"rt_primary",
                  "access_token":"ak_fallback",
                  "account_id":"acct_codex_both",
                  "exp": 1893456000
                }"#,
            ),
        };

        let items = parse_file_records(&file, &CreateOAuthImportJobOptions::default())
            .expect("parse file records");
        assert_eq!(items.len(), 1);

        let req = match items[0].request.as_ref().expect("request") {
            ImportTaskRequest::OAuthRefresh(req) => req,
            ImportTaskRequest::OneTimeAccessToken(_) => panic!("expected oauth refresh request"),
            ImportTaskRequest::ManualRefreshAccount(_) => panic!("expected oauth refresh request"),
        };

        assert_eq!(req.refresh_token, "rt_primary");
        assert_eq!(req.fallback_access_token.as_deref(), Some("ak_fallback"));
        assert_eq!(
            req.fallback_token_expires_at.map(|ts| ts.timestamp()),
            Some(1_893_456_000)
        );
        assert_eq!(req.chatgpt_account_id.as_deref(), Some("acct_codex_both"));
    }

    #[test]
    fn parse_record_prefers_refresh_token_when_mode_is_refresh_token() {
        let file = ImportUploadFile {
            file_name: "mixed-creds.jsonl".to_string(),
            content: Bytes::from(
                r#"{"email":"mixed@example.com","refresh_token":"rt_selected","access_token":"ak_ignored"}"#
                    .to_string(),
            ),
        };
        let options = CreateOAuthImportJobOptions {
            credential_mode: ImportCredentialMode::RefreshToken,
            ..CreateOAuthImportJobOptions::default()
        };

        let items = parse_file_records(&file, &options).expect("parse file records");
        assert_eq!(items.len(), 1);

        let req = match items[0].request.as_ref().expect("request") {
            ImportTaskRequest::OAuthRefresh(req) => req,
            ImportTaskRequest::OneTimeAccessToken(_) => panic!("expected refresh-token import"),
            ImportTaskRequest::ManualRefreshAccount(_) => panic!("expected refresh-token import"),
        };
        assert_eq!(req.refresh_token, "rt_selected");
    }

    #[test]
    fn parse_record_prefers_access_token_when_mode_is_access_token() {
        let file = ImportUploadFile {
            file_name: "mixed-creds.jsonl".to_string(),
            content: Bytes::from(
                r#"{"email":"mixed@example.com","refresh_token":"rt_ignored","access_token":"ak_selected"}"#
                    .to_string(),
            ),
        };
        let options = CreateOAuthImportJobOptions {
            credential_mode: ImportCredentialMode::AccessToken,
            ..CreateOAuthImportJobOptions::default()
        };

        let items = parse_file_records(&file, &options).expect("parse file records");
        assert_eq!(items.len(), 1);

        let req = match items[0].request.as_ref().expect("request") {
            ImportTaskRequest::OneTimeAccessToken(req) => req,
            ImportTaskRequest::OAuthRefresh(_) => panic!("expected access-token import"),
            ImportTaskRequest::ManualRefreshAccount(_) => panic!("expected access-token import"),
        };
        assert_eq!(req.access_token, "ak_selected");
    }

    #[test]
    fn parse_record_reports_missing_access_token_when_selected_mode_requires_it() {
        let file = ImportUploadFile {
            file_name: "refresh-only.jsonl".to_string(),
            content: Bytes::from(r#"{"email":"rt@example.com","refresh_token":"rt_only"}"#.to_string()),
        };
        let options = CreateOAuthImportJobOptions {
            credential_mode: ImportCredentialMode::AccessToken,
            ..CreateOAuthImportJobOptions::default()
        };

        let items = parse_file_records(&file, &options).expect("parse file records");
        assert_eq!(items.len(), 1);
        assert!(items[0].request.is_none());
        assert_eq!(items[0].item.status, OAuthImportItemStatus::Failed);
        assert_eq!(items[0].item.error_code.as_deref(), Some("missing_access_token"));
    }

    #[test]
    fn parse_record_reports_missing_refresh_token_when_selected_mode_requires_it() {
        let file = ImportUploadFile {
            file_name: "access-only.jsonl".to_string(),
            content: Bytes::from(r#"{"email":"ak@example.com","access_token":"ak_only"}"#.to_string()),
        };
        let options = CreateOAuthImportJobOptions {
            credential_mode: ImportCredentialMode::RefreshToken,
            ..CreateOAuthImportJobOptions::default()
        };

        let items = parse_file_records(&file, &options).expect("parse file records");
        assert_eq!(items.len(), 1);
        assert!(items[0].request.is_none());
        assert_eq!(items[0].item.status, OAuthImportItemStatus::Failed);
        assert_eq!(items[0].item.error_code.as_deref(), Some("missing_refresh_token"));
    }

    #[test]
    fn parse_record_derives_one_time_exp_from_access_token_jwt() {
        let file = ImportUploadFile {
            file_name: "codex-one-time-jwt-exp.json".to_string(),
            content: Bytes::from(
                r#"{
                  "type":"codex",
                  "email":"codex-jwt@example.com",
                  "access_token":"eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJleHAiOjE4OTM0NTYwMDB9.signature"
                }"#,
            ),
        };

        let items = parse_file_records(&file, &CreateOAuthImportJobOptions::default())
            .expect("parse file records");
        assert_eq!(items.len(), 1);

        let req = match items[0].request.as_ref().expect("request") {
            ImportTaskRequest::OneTimeAccessToken(req) => req,
            ImportTaskRequest::OAuthRefresh(_) => panic!("expected one-time request"),
            ImportTaskRequest::ManualRefreshAccount(_) => panic!("expected one-time request"),
        };

        assert_eq!(
            req.token_expires_at.map(|ts| ts.timestamp()),
            Some(1_893_456_000)
        );
    }

    #[tokio::test]
    async fn in_memory_store_has_no_recoverable_jobs() {
        let store = InMemoryOAuthImportJobStore::default();
        let ids = store
            .recoverable_job_ids()
            .await
            .expect("query recoverable jobs");
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn in_memory_store_pause_and_resume_job() {
        let store = InMemoryOAuthImportJobStore::default();
        let job_id = Uuid::new_v4();
        let (summary, items) = build_in_memory_job_state(job_id);
        store.create_job(summary, items).await.expect("create job");

        let pause_response = store.pause_job(job_id).await.expect("pause job");
        assert!(pause_response.accepted);
        let paused = store
            .get_job_summary(job_id)
            .await
            .expect("get paused summary");
        assert_eq!(paused.status, OAuthImportJobStatus::Paused);
        let paused_claim = store.start_job(job_id, 1).await.expect("claim paused job");
        assert!(paused_claim.is_empty());

        let resume_response = store.resume_job(job_id).await.expect("resume job");
        assert!(resume_response.accepted);
        let resumed = store
            .get_job_summary(job_id)
            .await
            .expect("get resumed summary");
        assert_eq!(resumed.status, OAuthImportJobStatus::Queued);
        let resumed_claim = store.start_job(job_id, 1).await.expect("claim resumed job");
        assert_eq!(resumed_claim.len(), 1);
    }

    #[tokio::test]
    async fn in_memory_store_resume_rejects_terminal_job() {
        let store = InMemoryOAuthImportJobStore::default();
        let job_id = Uuid::new_v4();
        let (summary, items) = build_in_memory_job_state(job_id);
        store.create_job(summary, items).await.expect("create job");

        let tasks = store.start_job(job_id, 1).await.expect("start job");
        assert_eq!(tasks.len(), 1);
        store
            .mark_item_success(
                job_id,
                tasks[0].item_id,
                &ImportTaskSuccess {
                    created: true,
                    account_id: None,
                    chatgpt_account_id: Some("acct-pause-resume".to_string()),
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
            )
            .await
            .expect("mark success");
        store.finish_job(job_id).await.expect("finish job");

        let response = store.resume_job(job_id).await.expect("resume completed job");
        assert!(!response.accepted);
    }

    #[tokio::test]
    async fn in_memory_store_can_claim_multiple_batches_for_running_job() {
        let store = InMemoryOAuthImportJobStore::default();
        let job_id = Uuid::new_v4();
        let summary = OAuthImportJobSummary {
            job_id,
            status: OAuthImportJobStatus::Queued,
            total: 3,
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
        };
        let items = (1..=3)
            .map(|item_id| PersistedImportItem {
                item: OAuthImportJobItem {
                    item_id,
                    source_file: "multi-batch.jsonl".to_string(),
                    line_no: item_id,
                    status: OAuthImportItemStatus::Pending,
                    label: format!("multi-batch-{item_id}"),
                    email: None,
                    chatgpt_account_id: Some(format!("acct-multi-{item_id}")),
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
                    label: format!("multi-batch-{item_id}"),
                    base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                    refresh_token: format!("rt-multi-{item_id}"),
                    fallback_access_token: None,
                    fallback_token_expires_at: None,
                    chatgpt_account_id: Some(format!("acct-multi-{item_id}")),
                    mode: Some(UpstreamMode::ChatGptSession),
                    enabled: Some(true),
                    priority: Some(100),
                    chatgpt_plan_type: None,
                    source_type: None,
                })),
                raw_record: None,
                normalized_record: None,
                retry_count: 0,
            })
            .collect::<Vec<_>>();
        store.create_job(summary, items).await.expect("create job");

        let first_batch = store.start_job(job_id, 2).await.expect("claim first batch");
        assert_eq!(first_batch.len(), 2);

        for task in &first_batch {
            store
                .mark_item_success(
                    job_id,
                    task.item_id,
                    &ImportTaskSuccess {
                        created: true,
                        account_id: None,
                        chatgpt_account_id: Some(format!("acct-success-{}", task.item_id)),
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
                )
                .await
                .expect("mark success");
        }

        let second_batch = store.start_job(job_id, 2).await.expect("claim second batch");
        assert_eq!(second_batch.len(), 1);
        assert_eq!(second_batch[0].item_id, 3);
    }

    #[test]
    fn classify_import_failure_code_detects_refresh_token_reused() {
        let code = classify_import_failure_code(
            "invalid refresh token (refresh_token_reused): upstream says token already used",
        );
        assert_eq!(code, "refresh_token_reused");
    }

    #[test]
    fn classify_import_failure_code_detects_invalid_refresh_token() {
        let code = classify_import_failure_code("invalid refresh token: revoked");
        assert_eq!(code, "invalid_refresh_token");
    }
}
