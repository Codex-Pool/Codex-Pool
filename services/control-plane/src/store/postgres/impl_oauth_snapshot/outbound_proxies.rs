const OUTBOUND_PROXY_POOL_SETTINGS_SINGLETON_ROW: bool = true;

fn proxy_fail_mode_to_db(mode: ProxyFailMode) -> &'static str {
    match mode {
        ProxyFailMode::StrictProxy => "strict_proxy",
        ProxyFailMode::AllowDirectFallback => "allow_direct_fallback",
    }
}

fn parse_proxy_fail_mode(raw: &str) -> Result<ProxyFailMode> {
    match raw {
        "strict_proxy" => Ok(ProxyFailMode::StrictProxy),
        "allow_direct_fallback" => Ok(ProxyFailMode::AllowDirectFallback),
        _ => Err(anyhow!("unsupported outbound proxy fail mode in postgres: {raw}")),
    }
}

fn parse_outbound_proxy_node_row(row: &sqlx_postgres::PgRow) -> Result<OutboundProxyNode> {
    Ok(OutboundProxyNode {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        proxy_url: row.try_get("proxy_url")?,
        enabled: row.try_get("enabled")?,
        weight: u32::try_from(row.try_get::<i64, _>("weight")?)
            .context("outbound proxy weight must be non-negative")?,
        last_test_status: row.try_get("last_test_status")?,
        last_latency_ms: row
            .try_get::<Option<i64>, _>("last_latency_ms")?
            .map(|value| u64::try_from(value).context("outbound proxy latency must be non-negative"))
            .transpose()?,
        last_error: row.try_get("last_error")?,
        last_tested_at: row.try_get("last_tested_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

impl PostgresStore {
    async fn load_outbound_proxy_pool_settings_inner(&self) -> Result<OutboundProxyPoolSettings> {
        let row = sqlx::query(
            r#"
            SELECT enabled, fail_mode, updated_at
            FROM outbound_proxy_pool_settings
            WHERE singleton = $1
            "#,
        )
        .bind(OUTBOUND_PROXY_POOL_SETTINGS_SINGLETON_ROW)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load outbound proxy pool settings")?;

        let Some(row) = row else {
            return Ok(OutboundProxyPoolSettings::default());
        };

        Ok(OutboundProxyPoolSettings {
            enabled: row.try_get("enabled")?,
            fail_mode: parse_proxy_fail_mode(&row.try_get::<String, _>("fail_mode")?)?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    async fn update_outbound_proxy_pool_settings_inner(
        &self,
        req: UpdateOutboundProxyPoolSettingsRequest,
    ) -> Result<OutboundProxyPoolSettings> {
        let updated_at = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start outbound proxy settings transaction")?;

        sqlx::query(
            r#"
            INSERT INTO outbound_proxy_pool_settings (singleton, enabled, fail_mode, updated_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (singleton) DO UPDATE
            SET
                enabled = EXCLUDED.enabled,
                fail_mode = EXCLUDED.fail_mode,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(OUTBOUND_PROXY_POOL_SETTINGS_SINGLETON_ROW)
        .bind(req.enabled)
        .bind(proxy_fail_mode_to_db(req.fail_mode))
        .bind(updated_at)
        .execute(tx.as_mut())
        .await
        .context("failed to update outbound proxy pool settings")?;

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit outbound proxy settings transaction")?;

        Ok(OutboundProxyPoolSettings {
            enabled: req.enabled,
            fail_mode: req.fail_mode,
            updated_at,
        })
    }

    async fn list_outbound_proxy_nodes_inner(&self) -> Result<Vec<OutboundProxyNode>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                label,
                proxy_url,
                enabled,
                weight,
                last_test_status,
                last_latency_ms,
                last_error,
                last_tested_at,
                created_at,
                updated_at
            FROM outbound_proxy_nodes
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list outbound proxy nodes")?;

        rows.iter().map(parse_outbound_proxy_node_row).collect()
    }

    async fn create_outbound_proxy_node_inner(
        &self,
        req: CreateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start outbound proxy create transaction")?;
        let now = Utc::now();
        let node_id = Uuid::new_v4();
        let row = sqlx::query(
            r#"
            INSERT INTO outbound_proxy_nodes (
                id,
                label,
                proxy_url,
                enabled,
                weight,
                last_test_status,
                last_latency_ms,
                last_error,
                last_tested_at,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, NULL, NULL, NULL, NULL, $6, $6)
            RETURNING
                id,
                label,
                proxy_url,
                enabled,
                weight,
                last_test_status,
                last_latency_ms,
                last_error,
                last_tested_at,
                created_at,
                updated_at
            "#,
        )
        .bind(node_id)
        .bind(req.label)
        .bind(req.proxy_url)
        .bind(req.enabled.unwrap_or(true))
        .bind(i64::from(req.weight.unwrap_or(1)))
        .bind(now)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to create outbound proxy node")?;

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit outbound proxy create transaction")?;

        parse_outbound_proxy_node_row(&row)
    }

    async fn update_outbound_proxy_node_inner(
        &self,
        node_id: Uuid,
        req: UpdateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        let existing = sqlx::query(
            r#"
            SELECT
                id,
                label,
                proxy_url,
                enabled,
                weight,
                last_test_status,
                last_latency_ms,
                last_error,
                last_tested_at,
                created_at,
                updated_at
            FROM outbound_proxy_nodes
            WHERE id = $1
            "#,
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load outbound proxy node for update")?;
        let Some(existing) = existing else {
            return Err(anyhow!("outbound proxy node not found"));
        };

        let existing = parse_outbound_proxy_node_row(&existing)?;
        let updated_at = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start outbound proxy update transaction")?;
        let row = sqlx::query(
            r#"
            UPDATE outbound_proxy_nodes
            SET
                label = $2,
                proxy_url = $3,
                enabled = $4,
                weight = $5,
                updated_at = $6
            WHERE id = $1
            RETURNING
                id,
                label,
                proxy_url,
                enabled,
                weight,
                last_test_status,
                last_latency_ms,
                last_error,
                last_tested_at,
                created_at,
                updated_at
            "#,
        )
        .bind(node_id)
        .bind(req.label.unwrap_or(existing.label))
        .bind(req.proxy_url.unwrap_or(existing.proxy_url))
        .bind(req.enabled.unwrap_or(existing.enabled))
        .bind(i64::from(req.weight.unwrap_or(existing.weight)))
        .bind(updated_at)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to update outbound proxy node")?;

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit outbound proxy update transaction")?;

        parse_outbound_proxy_node_row(&row)
    }

    async fn delete_outbound_proxy_node_inner(&self, node_id: Uuid) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start outbound proxy delete transaction")?;
        let deleted = sqlx::query(
            r#"
            DELETE FROM outbound_proxy_nodes
            WHERE id = $1
            "#,
        )
        .bind(node_id)
        .execute(tx.as_mut())
        .await
        .context("failed to delete outbound proxy node")?
        .rows_affected();

        if deleted == 0 {
            tx.rollback()
                .await
                .context("failed to rollback missing outbound proxy delete")?;
            return Err(anyhow!("outbound proxy node not found"));
        }

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit outbound proxy delete transaction")?;
        Ok(())
    }

    async fn record_outbound_proxy_test_result_inner(
        &self,
        node_id: Uuid,
        last_test_status: Option<String>,
        last_latency_ms: Option<u64>,
        last_error: Option<String>,
        last_tested_at: Option<DateTime<Utc>>,
    ) -> Result<OutboundProxyNode> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start outbound proxy test-result transaction")?;
        let row = sqlx::query(
            r#"
            UPDATE outbound_proxy_nodes
            SET
                last_test_status = $2,
                last_latency_ms = $3,
                last_error = $4,
                last_tested_at = $5,
                updated_at = now()
            WHERE id = $1
            RETURNING
                id,
                label,
                proxy_url,
                enabled,
                weight,
                last_test_status,
                last_latency_ms,
                last_error,
                last_tested_at,
                created_at,
                updated_at
            "#,
        )
        .bind(node_id)
        .bind(last_test_status)
        .bind(last_latency_ms.map(|value| i64::try_from(value).unwrap_or(i64::MAX)))
        .bind(last_error)
        .bind(last_tested_at)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to update outbound proxy test result")?;
        let Some(row) = row else {
            tx.rollback()
                .await
                .context("failed to rollback missing outbound proxy test-result update")?;
            return Err(anyhow!("outbound proxy node not found"));
        };

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit outbound proxy test-result transaction")?;

        parse_outbound_proxy_node_row(&row)
    }
}
