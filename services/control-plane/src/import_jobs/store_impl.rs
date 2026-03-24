#[async_trait]
impl OAuthImportJobStore for PostgresOAuthImportJobStore {
    async fn create_job(
        &self,
        summary: OAuthImportJobSummary,
        items: Vec<PersistedImportItem>,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start create oauth import job transaction")?;

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
            VALUES ($1, $2, false, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULL, '[]'::jsonb)
            "#,
        )
        .bind(summary.job_id)
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
        .context("failed to insert oauth import job")?;

        for persisted in items {
            let request_json = persisted
                .request
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .context("failed to serialize import request")?;
            let normalized_record = persisted
                .normalized_record
                .clone()
                .or_else(|| request_json.clone());

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
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, now(), now()
                )
                "#,
            )
            .bind(summary.job_id)
            .bind(i64::try_from(persisted.item.item_id).unwrap_or(i64::MAX))
            .bind(persisted.item.source_file)
            .bind(i64::try_from(persisted.item.line_no).unwrap_or(i64::MAX))
            .bind(item_status_to_db(persisted.item.status))
            .bind(persisted.item.label)
            .bind(persisted.item.email)
            .bind(persisted.item.chatgpt_account_id)
            .bind(persisted.item.account_id)
            .bind(persisted.item.error_code)
            .bind(persisted.item.error_message)
            .bind(persisted.item.admission_status.map(admission_status_to_db))
            .bind(persisted.item.admission_source)
            .bind(persisted.item.admission_reason)
            .bind(persisted.item.failure_stage.map(failure_stage_to_db))
            .bind(i32::try_from(persisted.item.attempt_count).unwrap_or(i32::MAX))
            .bind(i32::try_from(persisted.item.transient_retry_count).unwrap_or(i32::MAX))
            .bind(persisted.item.next_retry_at)
            .bind(persisted.item.retryable)
            .bind(persisted.item.terminal_reason)
            .bind(request_json)
            .bind(persisted.raw_record)
            .bind(normalized_record)
            .bind(i32::try_from(persisted.retry_count).unwrap_or(i32::MAX))
            .execute(tx.as_mut())
            .await
            .context("failed to insert oauth import job item")?;
        }

        tx.commit()
            .await
            .context("failed to commit create oauth import job transaction")?;
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

        let rows = match status {
            Some(status) => sqlx::query(
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
                    WHERE job_id = $1 AND status = $2 AND item_id > $3
                    ORDER BY item_id ASC
                    LIMIT $4
                    "#,
            )
            .bind(job_id)
            .bind(item_status_to_db(status))
            .bind(i64::try_from(cursor.unwrap_or(0)).unwrap_or_default())
            .bind(i64::try_from(fetch_limit).unwrap_or(i64::MAX))
            .fetch_all(&self.pool)
            .await
            .context("failed to list oauth import job items by status")?,
            None => sqlx::query(
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
                    WHERE job_id = $1 AND item_id > $2
                    ORDER BY item_id ASC
                    LIMIT $3
                    "#,
            )
            .bind(job_id)
            .bind(i64::try_from(cursor.unwrap_or(0)).unwrap_or_default())
            .bind(i64::try_from(fetch_limit).unwrap_or(i64::MAX))
            .fetch_all(&self.pool)
            .await
            .context("failed to list oauth import job items")?,
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
                account_id: row.try_get("account_id")?,
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
                attempt_count: row
                    .try_get::<i32, _>("attempt_count")?
                    .max(0)
                    .try_into()
                    .unwrap_or_default(),
                transient_retry_count: row
                    .try_get::<i32, _>("transient_retry_count")?
                    .max(0)
                    .try_into()
                    .unwrap_or_default(),
                next_retry_at: row.try_get("next_retry_at")?,
                retryable: row.try_get("retryable")?,
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
            .context("failed to start oauth import start-job transaction")?;

        let job_row = sqlx::query(
            r#"
            SELECT status, cancel_requested
            FROM oauth_import_jobs
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(job_id)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to lock oauth import job")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status = job_row.try_get::<String, _>("status")?;
        let cancel_requested = job_row.try_get::<bool, _>("cancel_requested")?;
        if cancel_requested
            || matches!(
                status.as_str(),
                DB_STATUS_PAUSED | DB_STATUS_COMPLETED | DB_STATUS_FAILED | DB_STATUS_CANCELLED
            )
        {
            tx.commit()
                .await
                .context("failed to commit no-op start-job")?;
            return Ok(Vec::new());
        }

        if status == DB_STATUS_RUNNING {
            sqlx::query(
                r#"
                UPDATE oauth_import_job_items
                SET status = $2,
                    updated_at = now()
                WHERE job_id = $1
                  AND status = $3
                "#,
            )
            .bind(job_id)
            .bind(DB_ITEM_PENDING)
            .bind(DB_ITEM_PROCESSING)
            .execute(tx.as_mut())
            .await
            .context("failed to reset processing oauth import items on recovery")?;
        }

        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET status = $2,
                started_at = COALESCE(started_at, now()),
                finished_at = NULL,
                throughput_per_min = NULL
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(DB_STATUS_RUNNING)
        .execute(tx.as_mut())
        .await
        .context("failed to set oauth import job running")?;

        let rows = sqlx::query(
            r#"
            WITH candidates AS (
                SELECT item_id, request_json
                FROM oauth_import_job_items
                WHERE job_id = $1
                  AND status = $3
                  AND request_json IS NOT NULL
                ORDER BY item_id ASC
                LIMIT $4
                FOR UPDATE SKIP LOCKED
            ),
            picked AS (
                UPDATE oauth_import_job_items AS items
                SET status = $2,
                    updated_at = now()
                FROM candidates
                WHERE items.job_id = $1
                  AND items.item_id = candidates.item_id
                RETURNING items.item_id, candidates.request_json
            )
            SELECT item_id, request_json
            FROM picked
            ORDER BY item_id ASC
            "#,
        )
        .bind(job_id)
        .bind(DB_ITEM_PROCESSING)
        .bind(DB_ITEM_PENDING)
        .bind(i64::try_from(limit.max(1)).unwrap_or(i64::MAX))
        .fetch_all(tx.as_mut())
        .await
        .context("failed to claim oauth import pending items")?;

        tx.commit()
            .await
            .context("failed to commit oauth import start-job transaction")?;

        let mut tasks = Vec::with_capacity(rows.len());
        for row in rows {
            let request_json: Value = row
                .try_get::<Option<Value>, _>("request_json")?
                .ok_or_else(|| anyhow!("missing request_json for processing item"))?;
            let request: ImportTaskRequest = serde_json::from_value(request_json.clone())
                .or_else(|_| {
                    serde_json::from_value::<ImportOAuthRefreshTokenRequest>(request_json)
                        .map(ImportTaskRequest::OAuthRefresh)
                })
                .context("failed to decode import request json")?;
            tasks.push(ImportJobTask {
                item_id: u64::try_from(row.try_get::<i64, _>("item_id")?).unwrap_or_default(),
                request,
            });
        }

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
            .context("failed to start mark_item_success transaction")?;

        let status = if outcome.created {
            DB_ITEM_CREATED
        } else {
            DB_ITEM_UPDATED
        };

        let updated = sqlx::query(
            r#"
            UPDATE oauth_import_job_items
            SET
                status = $3,
                account_id = $4,
                chatgpt_account_id = $5,
                admission_status = $6,
                admission_source = $7,
                admission_reason = $8,
                failure_stage = $9,
                attempt_count = $10,
                transient_retry_count = $11,
                next_retry_at = $12,
                retryable = $13,
                terminal_reason = $14,
                error_code = NULL,
                error_message = NULL,
                updated_at = now()
            WHERE job_id = $1
              AND item_id = $2
              AND status = $15
            "#,
        )
        .bind(job_id)
        .bind(i64::try_from(item_id).unwrap_or(i64::MAX))
        .bind(status)
        .bind(outcome.account_id)
        .bind(outcome.chatgpt_account_id.clone())
        .bind(outcome.admission_status.map(admission_status_to_db))
        .bind(outcome.admission_source.clone())
        .bind(outcome.admission_reason.clone())
        .bind(outcome.failure_stage.map(failure_stage_to_db))
        .bind(i32::try_from(outcome.attempt_count).unwrap_or(i32::MAX))
        .bind(i32::try_from(outcome.transient_retry_count).unwrap_or(i32::MAX))
        .bind(outcome.next_retry_at)
        .bind(outcome.retryable)
        .bind(outcome.terminal_reason.clone())
        .bind(DB_ITEM_PROCESSING)
        .execute(tx.as_mut())
        .await
        .context("failed to update oauth import item success")?
        .rows_affected();

        if updated > 0 {
            let counter_col = if outcome.created {
                "created_count"
            } else {
                "updated_count"
            };
            let sql = format!(
                "UPDATE oauth_import_jobs SET processed = processed + 1, {counter_col} = {counter_col} + 1 WHERE id = $1"
            );
            sqlx::query(&sql)
                .bind(job_id)
                .execute(tx.as_mut())
                .await
                .context("failed to increment oauth import success counters")?;
        }

        tx.commit()
            .await
            .context("failed to commit mark_item_success transaction")?;
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
            .context("failed to start mark_item_failed transaction")?;

        let updated = sqlx::query(
            r#"
            UPDATE oauth_import_job_items
            SET
                status = $3,
                error_code = $4,
                error_message = $5,
                admission_status = $6,
                admission_source = NULL,
                admission_reason = $7,
                failure_stage = NULL,
                attempt_count = 0,
                transient_retry_count = 0,
                next_retry_at = NULL,
                retryable = false,
                terminal_reason = $7,
                updated_at = now()
            WHERE job_id = $1
              AND item_id = $2
              AND status = $8
            "#,
        )
        .bind(job_id)
        .bind(i64::try_from(item_id).unwrap_or(i64::MAX))
        .bind(DB_ITEM_FAILED)
        .bind(error_code)
        .bind(error_message)
        .bind("failed")
        .bind(error_code)
        .bind(DB_ITEM_PROCESSING)
        .execute(tx.as_mut())
        .await
        .context("failed to update oauth import item failed")?
        .rows_affected();

        if updated > 0 {
            sqlx::query(
                r#"
                UPDATE oauth_import_jobs
                SET processed = processed + 1,
                    failed_count = failed_count + 1
                WHERE id = $1
                "#,
            )
            .bind(job_id)
            .execute(tx.as_mut())
            .await
            .context("failed to increment oauth import failure counters")?;
        }

        tx.commit()
            .await
            .context("failed to commit mark_item_failed transaction")?;
        Ok(())
    }

    async fn finish_job(&self, job_id: Uuid) -> Result<OAuthImportJobSummary> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start finish_job transaction")?;

        let row = sqlx::query(
            r#"
            SELECT cancel_requested, started_at
            FROM oauth_import_jobs
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(job_id)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to lock oauth import job for finish")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let cancel_requested = row.try_get::<bool, _>("cancel_requested")?;
        let started_at = row.try_get::<Option<DateTime<Utc>>, _>("started_at")?;

        if cancel_requested {
            sqlx::query(
                r#"
                UPDATE oauth_import_job_items
                SET status = $2,
                    updated_at = now()
                WHERE job_id = $1
                  AND status IN ($3, $4)
                "#,
            )
            .bind(job_id)
            .bind(DB_ITEM_CANCELLED)
            .bind(DB_ITEM_PENDING)
            .bind(DB_ITEM_PROCESSING)
            .execute(tx.as_mut())
            .await
            .context("failed to mark pending/processing items cancelled")?;
        }

        tx.commit()
            .await
            .context("failed to commit pre-finish transaction")?;

        let (processed, created_count, updated_count, failed_count, skipped_count) =
            self.recompute_counts(job_id).await?;
        let error_summary = self.load_error_summary(job_id).await?;
        let finished_at = Utc::now();
        let status = if cancel_requested {
            OAuthImportJobStatus::Cancelled
        } else if failed_count > 0 {
            OAuthImportJobStatus::Failed
        } else {
            OAuthImportJobStatus::Completed
        };
        let throughput = compute_throughput_per_min(started_at, Some(finished_at), processed);

        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET
                status = $2,
                processed = $3,
                created_count = $4,
                updated_count = $5,
                failed_count = $6,
                skipped_count = $7,
                finished_at = $8,
                throughput_per_min = $9,
                error_summary = $10
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(job_status_to_db(status))
        .bind(i64::try_from(processed).unwrap_or(i64::MAX))
        .bind(i64::try_from(created_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(updated_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(failed_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(skipped_count).unwrap_or(i64::MAX))
        .bind(finished_at)
        .bind(throughput)
        .bind(serde_json::to_value(&error_summary).context("failed to encode error summary")?)
        .execute(&self.pool)
        .await
        .context("failed to update finished oauth import job")?;

        self.get_job_summary(job_id).await
    }

    async fn pause_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start pause_job transaction")?;

        let row = sqlx::query(
            r#"
            SELECT status, cancel_requested
            FROM oauth_import_jobs
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(job_id)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to lock oauth import job for pause")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status = row.try_get::<String, _>("status")?;
        let cancel_requested = row.try_get::<bool, _>("cancel_requested")?;
        let accepted =
            !cancel_requested && matches!(status.as_str(), DB_STATUS_QUEUED | DB_STATUS_RUNNING);

        if accepted {
            sqlx::query(
                r#"
                UPDATE oauth_import_jobs
                SET status = $2,
                    finished_at = NULL
                WHERE id = $1
                "#,
            )
            .bind(job_id)
            .bind(DB_STATUS_PAUSED)
            .execute(tx.as_mut())
            .await
            .context("failed to pause oauth import job")?;
        }

        tx.commit()
            .await
            .context("failed to commit pause_job transaction")?;
        Ok(OAuthImportJobActionResponse { job_id, accepted })
    }

    async fn resume_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start resume_job transaction")?;

        let row = sqlx::query(
            r#"
            SELECT status, cancel_requested
            FROM oauth_import_jobs
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(job_id)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to lock oauth import job for resume")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status = row.try_get::<String, _>("status")?;
        let cancel_requested = row.try_get::<bool, _>("cancel_requested")?;
        let accepted = !cancel_requested && status == DB_STATUS_PAUSED;
        if accepted {
            sqlx::query(
                r#"
                UPDATE oauth_import_job_items
                SET status = $2,
                    updated_at = now()
                WHERE job_id = $1
                  AND status = $3
                "#,
            )
            .bind(job_id)
            .bind(DB_ITEM_PENDING)
            .bind(DB_ITEM_PROCESSING)
            .execute(tx.as_mut())
            .await
            .context("failed to reset processing oauth import items on resume")?;

            sqlx::query(
                r#"
                UPDATE oauth_import_jobs
                SET status = $2,
                    cancel_requested = false,
                    finished_at = NULL,
                    throughput_per_min = NULL
                WHERE id = $1
                "#,
            )
            .bind(job_id)
            .bind(DB_STATUS_QUEUED)
            .execute(tx.as_mut())
            .await
            .context("failed to resume oauth import job")?;
        }

        tx.commit()
            .await
            .context("failed to commit resume_job transaction")?;
        Ok(OAuthImportJobActionResponse { job_id, accepted })
    }

    async fn cancel_job(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET cancel_requested = true,
                status = CASE WHEN status IN ($2, $3) THEN $4 ELSE status END,
                finished_at = CASE WHEN status IN ($2, $3) THEN now() ELSE finished_at END
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(DB_STATUS_QUEUED)
        .bind(DB_STATUS_PAUSED)
        .bind(DB_STATUS_CANCELLED)
        .execute(&self.pool)
        .await
        .context("failed to cancel oauth import job")?;

        Ok(OAuthImportJobActionResponse {
            job_id,
            accepted: true,
        })
    }

    async fn retry_failed(&self, job_id: Uuid) -> Result<OAuthImportJobActionResponse> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start retry_failed transaction")?;

        let row = sqlx::query(
            r#"
            SELECT status
            FROM oauth_import_jobs
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(job_id)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to lock oauth import job for retry")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status = row.try_get::<String, _>("status")?;
        if status == DB_STATUS_RUNNING {
            tx.commit()
                .await
                .context("failed to commit running retry")?;
            return Ok(OAuthImportJobActionResponse {
                job_id,
                accepted: false,
            });
        }

        sqlx::query(
            r#"
            UPDATE oauth_import_job_items
            SET status = $2,
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
                retryable = false,
                terminal_reason = NULL,
                retry_count = retry_count + 1,
                updated_at = now()
            WHERE job_id = $1
              AND status = $3
              AND request_json IS NOT NULL
            "#,
        )
        .bind(job_id)
        .bind(DB_ITEM_PENDING)
        .bind(DB_ITEM_FAILED)
        .execute(tx.as_mut())
        .await
        .context("failed to reset failed oauth import items")?;

        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET
                status = $2,
                cancel_requested = false,
                started_at = NULL,
                finished_at = NULL,
                throughput_per_min = NULL,
                error_summary = '[]'::jsonb
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(DB_STATUS_QUEUED)
        .execute(tx.as_mut())
        .await
        .context("failed to reset oauth import job to queued")?;

        tx.commit()
            .await
            .context("failed to commit retry_failed transaction")?;

        let (processed, created_count, updated_count, failed_count, skipped_count) =
            self.recompute_counts(job_id).await?;

        sqlx::query(
            r#"
            UPDATE oauth_import_jobs
            SET
                processed = $2,
                created_count = $3,
                updated_count = $4,
                failed_count = $5,
                skipped_count = $6
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(i64::try_from(processed).unwrap_or(i64::MAX))
        .bind(i64::try_from(created_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(updated_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(failed_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(skipped_count).unwrap_or(i64::MAX))
        .execute(&self.pool)
        .await
        .context("failed to refresh counters after retry_failed")?;

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
            WHERE status IN ($1, $2)
              AND cancel_requested = false
            ORDER BY created_at ASC
            "#,
        )
        .bind(DB_STATUS_QUEUED)
        .bind(DB_STATUS_RUNNING)
        .fetch_all(&self.pool)
        .await
        .context("failed to load recoverable oauth import jobs")?;

        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            ids.push(row.try_get("id")?);
        }
        Ok(ids)
    }
}
