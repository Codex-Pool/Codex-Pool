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
                Ok(ImportTaskSuccess {
                    created,
                    account_id: None,
                    chatgpt_account_id: req.chatgpt_account_id,
                })
            }
            ImportTaskRequest::OneTimeAccessToken(req) => {
                let upserted = data_store.upsert_one_time_session_account(req).await?;
                Ok(ImportTaskSuccess {
                    created: upserted.created,
                    account_id: Some(upserted.account.id),
                    chatgpt_account_id: upserted.account.chatgpt_account_id,
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
        OAuthImportJobStatus::Completed => DB_STATUS_COMPLETED,
        OAuthImportJobStatus::Failed => DB_STATUS_FAILED,
        OAuthImportJobStatus::Cancelled => DB_STATUS_CANCELLED,
    }
}

fn parse_job_status(raw: &str) -> Result<OAuthImportJobStatus> {
    match raw {
        DB_STATUS_QUEUED => Ok(OAuthImportJobStatus::Queued),
        DB_STATUS_RUNNING => Ok(OAuthImportJobStatus::Running),
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

#[cfg(test)]
mod tests {
    use super::{
        classify_import_failure_code, parse_file_records, CreateOAuthImportJobOptions,
        ImportTaskRequest, ImportUploadFile, InMemoryOAuthImportJobStore, OAuthImportJobStore,
    };
    use bytes::Bytes;
    use codex_pool_core::model::UpstreamMode;

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
