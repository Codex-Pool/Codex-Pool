impl PostgresStore {
    async fn claim_due_probe_accounts_inner(
        &self,
        limit: usize,
        seen_ok_suppress_sec: i64,
        lock_ttl_sec: i64,
        claimed_by: &str,
    ) -> Result<Vec<ClaimedProbeAccount>> {
        let seen_ok_suppress_sec = i32::try_from(seen_ok_suppress_sec.max(0)).unwrap_or(i32::MAX);
        let lock_ttl_sec = i32::try_from(lock_ttl_sec.max(1)).unwrap_or(i32::MAX);
        let limit = i64::try_from(limit.max(1)).unwrap_or(i64::MAX);

        let rows = sqlx::query(
            r#"
            WITH due_raw AS (
                SELECT
                    a.id AS account_id,
                    COALESCE(h.failure_count, 0)::INT AS failure_count,
                    COALESCE(h.next_probe_at, a.created_at) AS ord_probe_at,
                    a.created_at AS ord_created_at
                FROM upstream_accounts a
                LEFT JOIN upstream_account_health_state h ON h.account_id = a.id
                WHERE
                    a.pool_state = $1
                    AND a.enabled = true
                    AND (h.next_probe_at IS NULL OR h.next_probe_at <= now())
                    AND (
                        h.seen_ok_at IS NULL
                        OR h.seen_ok_at <= now() - make_interval(secs => $2::INT)
                    )
                ORDER BY ord_probe_at ASC, ord_created_at ASC, a.id ASC
                LIMIT $3
            ),
            due AS (
                SELECT
                    account_id,
                    failure_count,
                    ROW_NUMBER() OVER () AS ord
                FROM due_raw
            ),
            claimed AS (
                INSERT INTO upstream_account_ops_locks (
                    account_id,
                    op_type,
                    inflight_until,
                    claimed_at,
                    claimed_by
                )
                SELECT
                    due.account_id,
                    $4,
                    now() + make_interval(secs => $5::INT),
                    now(),
                    $6
                FROM due
                ON CONFLICT (account_id, op_type) DO UPDATE
                SET
                    inflight_until = EXCLUDED.inflight_until,
                    claimed_at = EXCLUDED.claimed_at,
                    claimed_by = EXCLUDED.claimed_by
                WHERE upstream_account_ops_locks.inflight_until <= now()
                RETURNING account_id
            )
            SELECT due.account_id, due.failure_count
            FROM due
            INNER JOIN claimed ON claimed.account_id = due.account_id
            ORDER BY due.ord ASC
            "#,
        )
        .bind(POOL_STATE_ACTIVE)
        .bind(seen_ok_suppress_sec)
        .bind(limit)
        .bind(UPSTREAM_OP_TYPE_PROBE)
        .bind(lock_ttl_sec)
        .bind(claimed_by)
        .fetch_all(&self.pool)
        .await
        .context("failed to claim due probe accounts")?;

        let mut claimed = Vec::with_capacity(rows.len());
        for row in rows {
            let failure_count = row.try_get::<i32, _>("failure_count")?;
            claimed.push(ClaimedProbeAccount {
                account_id: row.try_get::<Uuid, _>("account_id")?,
                failure_count: u32::try_from(failure_count.max(0)).unwrap_or(0),
            });
        }
        Ok(claimed)
    }

    async fn release_upstream_op_lock_inner(&self, account_id: Uuid, op_type: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM upstream_account_ops_locks
            WHERE account_id = $1
              AND op_type = $2
            "#,
        )
        .bind(account_id)
        .bind(op_type)
        .execute(&self.pool)
        .await
        .context("failed to release upstream op lock")?;
        Ok(())
    }

    async fn record_upstream_probe_inner(&self, account_id: Uuid, write: UpstreamProbeWrite) -> Result<()> {
        let status = match write.status {
            UpstreamProbeStatus::Ok => "ok",
            UpstreamProbeStatus::Fail => "fail",
        };
        let observed_at = write.observed_at;
        let next_probe_at = write.next_probe_at;
        let http_status = write.http_status.map(i32::from);
        let error_code = write.error_code.unwrap_or_default();
        let error_message = write.error_message.unwrap_or_default();

        match write.status {
            UpstreamProbeStatus::Ok => {
                sqlx::query(
                    r#"
                    INSERT INTO upstream_account_health_state (
                        account_id,
                        last_probe_at,
                        last_probe_status,
                        last_probe_http_status,
                        last_probe_error_code,
                        last_probe_error_message,
                        failure_count,
                        next_probe_at,
                        created_at,
                        updated_at
                    )
                    SELECT
                        a.id,
                        $2,
                        $3,
                        $4,
                        NULL,
                        NULL,
                        0,
                        $5,
                        $2,
                        $2
                    FROM upstream_accounts a
                    WHERE a.id = $1
                    ON CONFLICT (account_id) DO UPDATE
                    SET
                        last_probe_at = EXCLUDED.last_probe_at,
                        last_probe_status = EXCLUDED.last_probe_status,
                        last_probe_http_status = EXCLUDED.last_probe_http_status,
                        last_probe_error_code = NULL,
                        last_probe_error_message = NULL,
                        failure_count = 0,
                        next_probe_at = EXCLUDED.next_probe_at,
                        updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(account_id)
                .bind(observed_at)
                .bind(status)
                .bind(http_status)
                .bind(next_probe_at)
                .execute(&self.pool)
                .await
                .context("failed to persist upstream probe success")?;
            }
            UpstreamProbeStatus::Fail => {
                sqlx::query(
                    r#"
                    INSERT INTO upstream_account_health_state (
                        account_id,
                        last_probe_at,
                        last_probe_status,
                        last_probe_http_status,
                        last_probe_error_code,
                        last_probe_error_message,
                        failure_count,
                        next_probe_at,
                        created_at,
                        updated_at
                    )
                    SELECT
                        a.id,
                        $2,
                        $3,
                        $4,
                        NULLIF($5, ''),
                        NULLIF($6, ''),
                        1,
                        $7,
                        $2,
                        $2
                    FROM upstream_accounts a
                    WHERE a.id = $1
                    ON CONFLICT (account_id) DO UPDATE
                    SET
                        last_probe_at = EXCLUDED.last_probe_at,
                        last_probe_status = EXCLUDED.last_probe_status,
                        last_probe_http_status = EXCLUDED.last_probe_http_status,
                        last_probe_error_code = EXCLUDED.last_probe_error_code,
                        last_probe_error_message = EXCLUDED.last_probe_error_message,
                        failure_count = upstream_account_health_state.failure_count + 1,
                        next_probe_at = EXCLUDED.next_probe_at,
                        updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(account_id)
                .bind(observed_at)
                .bind(status)
                .bind(http_status)
                .bind(error_code)
                .bind(error_message)
                .bind(next_probe_at)
                .execute(&self.pool)
                .await
                .context("failed to persist upstream probe failure")?;
            }
        }

        Ok(())
    }

    async fn mark_account_seen_ok_inner(
        &self,
        account_id: Uuid,
        seen_ok_at: DateTime<Utc>,
        min_write_interval_sec: i64,
    ) -> Result<bool> {
        let threshold = seen_ok_at - Duration::seconds(min_write_interval_sec.max(0));
        let result = sqlx::query(
            r#"
            INSERT INTO upstream_account_health_state (
                account_id,
                seen_ok_at,
                failure_count,
                created_at,
                updated_at
            )
            SELECT
                a.id,
                $2,
                0,
                now(),
                now()
            FROM upstream_accounts a
            WHERE a.id = $1
            ON CONFLICT (account_id) DO UPDATE
            SET
                seen_ok_at = GREATEST(
                    COALESCE(upstream_account_health_state.seen_ok_at, EXCLUDED.seen_ok_at),
                    EXCLUDED.seen_ok_at
                ),
                updated_at = now()
            WHERE
                upstream_account_health_state.seen_ok_at IS NULL
                OR upstream_account_health_state.seen_ok_at <= $3
            "#,
        )
        .bind(account_id)
        .bind(seen_ok_at)
        .bind(threshold)
        .execute(&self.pool)
        .await
        .context("failed to persist seen_ok signal")?;

        Ok(result.rows_affected() > 0)
    }
}
