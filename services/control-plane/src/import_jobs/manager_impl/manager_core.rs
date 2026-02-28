#[derive(Clone)]
pub struct OAuthImportJobManager {
    data_store: Arc<dyn ControlPlaneStore>,
    job_store: Arc<dyn OAuthImportJobStore>,
    concurrency: usize,
    claim_batch_size: usize,
}

#[derive(Debug, Deserialize)]
struct CredentialRecord {
    refresh_token: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    bearer_token: Option<String>,
    #[serde(default, rename = "type", alias = "typo")]
    record_type: Option<String>,
    #[serde(default)]
    exp: Option<i64>,
    #[serde(default)]
    expired: Option<String>,
    #[serde(default)]
    chatgpt_plan_type: Option<String>,
    #[serde(default, rename = "https://api.openai.com/auth")]
    openai_auth: Option<CredentialOpenAiAuth>,
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    priority: Option<i32>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    token_info: Option<CredentialTokenInfo>,
}

#[derive(Debug, Deserialize)]
struct CredentialTokenInfo {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CredentialOpenAiAuth {
    #[serde(default)]
    chatgpt_plan_type: Option<String>,
}

impl OAuthImportJobManager {
    pub fn new(
        data_store: Arc<dyn ControlPlaneStore>,
        job_store: Arc<dyn OAuthImportJobStore>,
        concurrency: usize,
        claim_batch_size: usize,
    ) -> Self {
        Self {
            data_store,
            job_store,
            concurrency: concurrency.max(1),
            claim_batch_size: claim_batch_size.max(1),
        }
    }

    pub async fn create_job(
        &self,
        files: Vec<ImportUploadFile>,
        options: CreateOAuthImportJobOptions,
    ) -> Result<OAuthImportJobSummary> {
        if files.is_empty() {
            return Err(anyhow!("no files uploaded"));
        }

        let mut items = Vec::new();
        let mut item_id: u64 = 0;

        for file in files {
            let parsed_items = parse_file_records(&file, &options)
                .with_context(|| format!("failed to parse file {}", file.file_name))?;
            for mut state in parsed_items {
                item_id = item_id.saturating_add(1);
                state.item.item_id = item_id;
                items.push(state);
            }
        }

        let now = Utc::now();
        let mut summary = OAuthImportJobSummary {
            job_id: Uuid::new_v4(),
            status: OAuthImportJobStatus::Queued,
            total: items.len() as u64,
            processed: 0,
            created_count: 0,
            updated_count: 0,
            failed_count: 0,
            skipped_count: 0,
            started_at: None,
            finished_at: None,
            created_at: now,
            throughput_per_min: None,
            error_summary: Vec::new(),
        };
        refresh_summary_counts(&mut summary, &items, false);

        self.job_store.create_job(summary.clone(), items).await?;
        self.spawn_job(summary.job_id);
        self.job_store.get_job_summary(summary.job_id).await
    }

    pub async fn create_manual_refresh_job(&self, account_id: Uuid) -> Result<OAuthImportJobSummary> {
        let now = Utc::now();
        let request = ImportTaskRequest::ManualRefreshAccount(ManualRefreshTaskRequest { account_id });
        let item = OAuthImportJobItem {
            item_id: 1,
            source_file: "manual_refresh".to_string(),
            line_no: 1,
            status: OAuthImportItemStatus::Pending,
            label: format!("manual-refresh-{account_id}"),
            email: None,
            chatgpt_account_id: None,
            account_id: Some(account_id),
            error_code: None,
            error_message: None,
        };
        let mut persisted = PersistedImportItem {
            item,
            request: Some(request.clone()),
            raw_record: Some(serde_json::json!({
                "kind": "manual_refresh_account",
                "account_id": account_id
            })),
            normalized_record: None,
            retry_count: 0,
        };
        persisted.normalized_record = Some(serde_json::to_value(&request)?);

        let mut summary = OAuthImportJobSummary {
            job_id: Uuid::new_v4(),
            status: OAuthImportJobStatus::Queued,
            total: 1,
            processed: 0,
            created_count: 0,
            updated_count: 0,
            failed_count: 0,
            skipped_count: 0,
            started_at: None,
            finished_at: None,
            created_at: now,
            throughput_per_min: None,
            error_summary: Vec::new(),
        };
        refresh_summary_counts(&mut summary, &[persisted.clone()], false);

        self.job_store.create_job(summary.clone(), vec![persisted]).await?;
        self.spawn_job(summary.job_id);
        self.job_store.get_job_summary(summary.job_id).await
    }

    pub async fn job_summary(&self, job_id: Uuid) -> Result<OAuthImportJobSummary> {
        self.job_store.get_job_summary(job_id).await
    }

    pub async fn job_items(
        &self,
        job_id: Uuid,
        status: Option<OAuthImportItemStatus>,
        cursor: Option<u64>,
        limit: u64,
    ) -> Result<OAuthImportJobItemsResponse> {
        self.job_store
            .get_job_items(job_id, status, cursor, limit)
            .await
    }

    pub async fn cancel_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        self.job_store.cancel_job(job_id).await
    }

    pub async fn retry_failed(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let response = self.job_store.retry_failed(job_id).await?;
        if response.accepted {
            self.spawn_job(job_id);
        }
        Ok(response)
    }

    pub fn resume_recoverable_jobs(&self) {
        let this = self.clone();
        tokio::spawn(async move {
            match this.job_store.recoverable_job_ids().await {
                Ok(job_ids) => {
                    for job_id in job_ids {
                        this.spawn_job(job_id);
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, "failed to recover oauth import jobs");
                }
            }
        });
    }

    fn spawn_job(&self, job_id: Uuid) {
        let this = self.clone();
        tokio::spawn(async move {
            let _ = this.run_job(job_id).await;
        });
    }

    async fn run_job(&self, job_id: Uuid) -> Result<()> {
        loop {
            let tasks = self
                .job_store
                .start_job(job_id, self.claim_batch_size)
                .await?;
            if tasks.is_empty() {
                break;
            }

            let data_store = self.data_store.clone();
            let job_store = self.job_store.clone();

            stream::iter(tasks)
                .map(|task| {
                    let data_store = data_store.clone();
                    async move {
                        let result = execute_import_with_retry(data_store, task.request).await;
                        (task.item_id, result)
                    }
                })
                .buffer_unordered(self.concurrency)
                .for_each(|(item_id, result)| {
                    let job_store = job_store.clone();
                    async move {
                        match result {
                            Ok(outcome) => {
                                let _ = job_store
                                    .mark_item_success(job_id, item_id, &outcome)
                                    .await;
                            }
                            Err(err) => {
                                let raw_message = err.to_string();
                                let error_code = classify_import_failure_code(&raw_message);
                                let _ = job_store
                                    .mark_item_failed(
                                        job_id,
                                        item_id,
                                        error_code,
                                        &truncate_error_message(raw_message),
                                    )
                                    .await;
                            }
                        }
                    }
                })
                .await;
        }

        let _ = self.job_store.finish_job(job_id).await?;
        Ok(())
    }
}
