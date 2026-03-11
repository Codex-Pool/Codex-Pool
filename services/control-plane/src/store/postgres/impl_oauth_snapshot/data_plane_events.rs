fn outbox_event_type_to_db(event_type: DataPlaneSnapshotEventType) -> &'static str {
    match event_type {
        DataPlaneSnapshotEventType::AccountUpsert => OUTBOX_EVENT_ACCOUNT_UPSERT,
        DataPlaneSnapshotEventType::AccountDelete => OUTBOX_EVENT_ACCOUNT_DELETE,
        DataPlaneSnapshotEventType::RoutingPlanRefresh => "routing_plan_refresh",
    }
}

fn parse_outbox_event_type(raw: &str) -> Result<DataPlaneSnapshotEventType> {
    match raw {
        OUTBOX_EVENT_ACCOUNT_UPSERT => Ok(DataPlaneSnapshotEventType::AccountUpsert),
        OUTBOX_EVENT_ACCOUNT_DELETE => Ok(DataPlaneSnapshotEventType::AccountDelete),
        "routing_plan_refresh" => Ok(DataPlaneSnapshotEventType::RoutingPlanRefresh),
        _ => Err(anyhow!("unsupported data-plane outbox event type: {raw}")),
    }
}

impl PostgresStore {
    async fn append_data_plane_outbox_event_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event_type: DataPlaneSnapshotEventType,
        account_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO data_plane_outbox (event_type, account_id, created_at)
            VALUES ($1, $2, now())
            "#,
        )
        .bind(outbox_event_type_to_db(event_type))
        .bind(account_id)
        .execute(tx.as_mut())
        .await
        .context("failed to append data-plane outbox event")?;
        Ok(())
    }

    async fn data_plane_outbox_cursor_inner(&self) -> Result<u64> {
        let max_id = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT MAX(id)
            FROM data_plane_outbox
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to read data-plane outbox cursor")?
        .unwrap_or(0);
        u64::try_from(max_id.max(0)).context("data-plane outbox cursor must be non-negative")
    }

    async fn data_plane_outbox_min_cursor_inner(&self) -> Result<Option<u64>> {
        let min_id = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT MIN(id)
            FROM data_plane_outbox
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to read data-plane outbox min cursor")?;
        min_id
            .map(|value| u64::try_from(value.max(0)).context("data-plane outbox min must be non-negative"))
            .transpose()
    }

    async fn cleanup_data_plane_outbox_inner(&self, retention: chrono::Duration) -> Result<u64> {
        let threshold = Utc::now() - retention.max(chrono::Duration::zero());
        let deleted = sqlx::query(
            r#"
            DELETE FROM data_plane_outbox
            WHERE created_at < $1
            "#,
        )
        .bind(threshold)
        .execute(&self.pool)
        .await
        .context("failed to cleanup data-plane outbox")?
        .rows_affected();
        Ok(deleted)
    }

    async fn load_snapshot_account_by_id(&self, account_id: Uuid) -> Result<Option<UpstreamAccount>> {
        let row = sqlx::query(
            r#"
            SELECT
                a.id,
                a.label,
                a.mode,
                a.base_url,
                a.bearer_token,
                a.chatgpt_account_id,
                a.auth_provider,
                a.enabled,
                a.priority,
                a.created_at,
                c.access_token_enc,
                c.token_expires_at,
                c.last_refresh_status,
                c.refresh_reused_detected,
                c.last_refresh_error_code,
                rl.expires_at AS rate_limits_expires_at,
                rl.last_error_code AS rate_limits_last_error_code,
                rl.last_error_message AS rate_limits_last_error
            FROM upstream_accounts a
            LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            LEFT JOIN upstream_account_rate_limit_snapshots rl ON rl.account_id = a.id
            WHERE a.id = $1
              AND a.pool_state = $2
            "#,
        )
        .bind(account_id)
        .bind(POOL_STATE_ACTIVE)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load account for data-plane outbox event")?;

        let Some(row) = row else {
            return Ok(None);
        };

        let now = Utc::now();
        let auth_provider =
            parse_upstream_auth_provider(row.try_get::<String, _>("auth_provider")?.as_str())?;
        let mode = parse_upstream_mode(row.try_get::<String, _>("mode")?.as_str())?;
        let token_expires_at = row.try_get::<Option<DateTime<Utc>>, _>("token_expires_at")?;
        let last_refresh_status = parse_oauth_refresh_status(
            row.try_get::<Option<String>, _>("last_refresh_status")?
                .unwrap_or_else(|| "never".to_string())
                .as_str(),
        )?;
        let refresh_reused_detected = row
            .try_get::<Option<bool>, _>("refresh_reused_detected")?
            .unwrap_or(false);
        let last_refresh_error_code = row.try_get::<Option<String>, _>("last_refresh_error_code")?;
        let rate_limits_expires_at = row.try_get::<Option<DateTime<Utc>>, _>("rate_limits_expires_at")?;
        let rate_limits_last_error_code =
            row.try_get::<Option<String>, _>("rate_limits_last_error_code")?;
        let rate_limits_last_error = row.try_get::<Option<String>, _>("rate_limits_last_error")?;
        let credential_kind = match (auth_provider.clone(), mode.clone()) {
            (UpstreamAuthProvider::OAuthRefreshToken, _) => Some(SessionCredentialKind::RefreshRotatable),
            (UpstreamAuthProvider::LegacyBearer, UpstreamMode::ChatGptSession)
            | (UpstreamAuthProvider::LegacyBearer, UpstreamMode::CodexOauth) => {
                Some(SessionCredentialKind::OneTimeAccessToken)
            }
            _ => None,
        };

        let mut enabled = oauth_effective_enabled(
            row.try_get::<bool, _>("enabled")?,
            &auth_provider,
            credential_kind.as_ref(),
            token_expires_at,
            &last_refresh_status,
            refresh_reused_detected,
            last_refresh_error_code.as_deref(),
            rate_limits_expires_at,
            rate_limits_last_error_code.as_deref(),
            rate_limits_last_error.as_deref(),
            now,
        );
        let mut bearer_token = row.try_get::<String, _>("bearer_token")?;
        if auth_provider == UpstreamAuthProvider::OAuthRefreshToken {
            if enabled {
                let access_token_enc = row.try_get::<Option<String>, _>("access_token_enc")?;
                if let (Some(access_token_enc), Some(cipher)) = (access_token_enc, self.credential_cipher.as_ref()) {
                    match cipher.decrypt(&access_token_enc) {
                        Ok(access_token) => bearer_token = access_token,
                        Err(_) => {
                            enabled = false;
                            bearer_token.clear();
                        }
                    }
                } else {
                    enabled = false;
                    bearer_token.clear();
                }
            } else {
                bearer_token.clear();
            }
        }

        Ok(Some(UpstreamAccount {
            id: row.try_get("id")?,
            label: row.try_get("label")?,
            mode,
            base_url: row.try_get("base_url")?,
            bearer_token,
            chatgpt_account_id: row.try_get("chatgpt_account_id")?,
            enabled,
            priority: row.try_get("priority")?,
            created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
        }))
    }

    async fn load_data_plane_snapshot_events_inner(
        &self,
        after: u64,
        limit: u32,
    ) -> Result<DataPlaneSnapshotEventsResponse> {
        let high_watermark = self.data_plane_outbox_cursor_inner().await?;
        if after > 0 {
            if let Some(min_cursor) = self.data_plane_outbox_min_cursor_inner().await? {
                if min_cursor > 0 && after.saturating_add(1) < min_cursor {
                    return Err(anyhow!("cursor_gone"));
                }
            }
        }

        let capped_limit = limit.clamp(1, 5_000);
        let rows = sqlx::query(
            r#"
            SELECT id, event_type, account_id, created_at
            FROM data_plane_outbox
            WHERE id > $1
            ORDER BY id ASC
            LIMIT $2
            "#,
        )
        .bind(i64::try_from(after).unwrap_or(i64::MAX))
        .bind(i64::from(capped_limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to list data-plane snapshot events")?;

        if rows.is_empty() {
            let cursor = high_watermark.max(after);
            return Ok(DataPlaneSnapshotEventsResponse {
                cursor,
                high_watermark,
                events: Vec::new(),
            });
        }

        let mut cursor = after;
        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let id_i64 = row.try_get::<i64, _>("id")?;
            let id = u64::try_from(id_i64.max(0))
                .context("data-plane outbox event id must be non-negative")?;
            cursor = cursor.max(id);
            let event_type = parse_outbox_event_type(row.try_get::<String, _>("event_type")?.as_str())?;
            let account_id = row.try_get::<Uuid, _>("account_id")?;
            let account = if matches!(event_type, DataPlaneSnapshotEventType::AccountUpsert) {
                self.load_snapshot_account_by_id(account_id).await?
            } else {
                None
            };
            let compiled_routing_plan = if matches!(
                event_type,
                DataPlaneSnapshotEventType::AccountUpsert
                    | DataPlaneSnapshotEventType::AccountDelete
                    | DataPlaneSnapshotEventType::RoutingPlanRefresh
            ) {
                self.load_compiled_routing_plan_inner().await?
            } else {
                None
            };
            events.push(DataPlaneSnapshotEvent {
                id,
                event_type,
                account_id,
                account,
                compiled_routing_plan,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
            });
        }

        Ok(DataPlaneSnapshotEventsResponse {
            cursor,
            high_watermark,
            events,
        })
    }
}
