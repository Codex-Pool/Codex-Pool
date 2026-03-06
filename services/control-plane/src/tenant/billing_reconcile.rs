impl TenantAuthService {
    pub async fn billing_precheck(&self, tenant_id: Uuid) -> Result<BillingPrecheckResponse> {
        let row = sqlx::query(
            r#"
            SELECT t.status, COALESCE(c.balance_microcredits, 0) AS balance_microcredits
            FROM tenants t
            LEFT JOIN tenant_credit_accounts c ON c.tenant_id = t.id
            WHERE t.id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query tenant for billing precheck")?
        .ok_or_else(|| anyhow!("tenant not found"))?;
        let tenant_status: String = row.try_get("status")?;
        let balance_microcredits: i64 = row.try_get("balance_microcredits")?;
        Ok(BillingPrecheckResponse {
            tenant_id,
            tenant_status: tenant_status.clone(),
            balance_microcredits,
            ok: tenant_status == "active" && balance_microcredits > 0,
        })
    }

    pub async fn billing_authorize(
        &self,
        req: BillingAuthorizeRequest,
    ) -> Result<BillingAuthorizeResponse> {
        let request_id = req.request_id.trim();
        if request_id.is_empty() {
            return Err(anyhow!("request_id must not be empty"));
        }
        let model = req.model.trim();
        if model.is_empty() {
            return Err(anyhow!("model must not be empty"));
        }
        if req.reserved_microcredits <= 0 {
            return Err(anyhow!("reserved_microcredits must be positive"));
        }

        let ttl_sec = req
            .ttl_sec
            .unwrap_or_else(default_billing_authorization_ttl_sec)
            .clamp(
                MIN_BILLING_AUTHORIZATION_TTL_SEC,
                MAX_BILLING_AUTHORIZATION_TTL_SEC,
            );
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(ttl_sec as i64);
        let request_kind = BillingRequestKind::from_optional(req.request_kind.as_deref());
        let session_key = req
            .session_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start billing authorize transaction")?;

        if let Some(existing) = self
            .fetch_authorization_for_update(&mut tx, req.tenant_id, request_id)
            .await?
        {
            if !authorization_reusable_for_authorize(&existing.status) {
                return Err(anyhow!(
                    "billing authorization is in invalid status for authorize reuse: {}",
                    existing.status
                ));
            }
            let balance_microcredits = self
                .current_credit_balance_for_update(&mut tx, req.tenant_id)
                .await?;
            tx.commit()
                .await
                .context("failed to commit billing authorize transaction")?;
            return Ok(BillingAuthorizeResponse {
                authorization_id: existing.id,
                tenant_id: existing.tenant_id,
                request_id: existing.request_id,
                status: existing.status,
                reserved_microcredits: existing.reserved_microcredits,
                captured_microcredits: existing.captured_microcredits,
                balance_microcredits,
                expires_at: existing.expires_at,
            });
        }

        let existing_session = match session_key.as_deref() {
            Some(value) => {
                self.fetch_billing_session_for_update(&mut tx, req.tenant_id, value)
                    .await?
            }
            None => None,
        };
        let pricing_decision = self
            .resolve_model_pricing_for_request(
                model,
                req.api_key_id,
                request_kind,
                existing_session
                    .as_ref()
                    .map(|record| BillingPricingBand::from_optional(Some(record.pricing_band.as_str()))),
                None,
                BillingResolutionPhase::Authorize,
            )
            .await?;

        let authorization_id = Uuid::new_v4();
        let balance_microcredits = self
            .apply_credit_delta_inner(
                &mut tx,
                CreditDeltaParams {
                    tenant_id: req.tenant_id,
                    api_key_id: req.api_key_id,
                    event_type: "authorize_hold",
                    delta_microcredits: -req.reserved_microcredits,
                    request_id: None,
                    model: Some(model.to_string()),
                    input_tokens: None,
                    output_tokens: None,
                    meta_json: Some(json!({
                        "phase": "authorize",
                        "authorization_id": authorization_id,
                        "request_id": request_id,
                        "trace_request_id": req.trace_request_id,
                        "session_key": session_key,
                        "request_kind": request_kind.as_str(),
                        "pricing_band": pricing_decision.band.as_str(),
                        "pricing_rule_id": pricing_decision.matched_rule_id.map(|id| id.to_string()),
                        "is_stream": req.is_stream,
                    })),
                    now,
                },
            )
            .await?;

        sqlx::query(
            r#"
            INSERT INTO tenant_credit_authorizations (
                id, tenant_id, api_key_id, request_id, model, reserved_microcredits,
                captured_microcredits, status, expires_at, meta_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 0, 'authorized', $7, $8, $9, $9)
            "#,
        )
        .bind(authorization_id)
        .bind(req.tenant_id)
        .bind(req.api_key_id)
        .bind(request_id)
        .bind(model)
        .bind(req.reserved_microcredits)
        .bind(expires_at)
        .bind(json!({
            "phase": "authorize",
            "source": "data_plane",
            "session_key": session_key,
            "request_kind": request_kind.as_str(),
            "pricing_band": pricing_decision.band.as_str(),
            "pricing_rule_id": pricing_decision.matched_rule_id.map(|id| id.to_string()),
            "is_stream": req.is_stream,
        }))
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to insert billing authorization")?;

        if let Some(value) = session_key.as_deref() {
            self.upsert_billing_session(
                &mut tx,
                req.tenant_id,
                value,
                model,
                pricing_decision.band,
                now,
            )
            .await?;
        }

        tx.commit()
            .await
            .context("failed to commit billing authorize transaction")?;

        Ok(BillingAuthorizeResponse {
            authorization_id,
            tenant_id: req.tenant_id,
            request_id: request_id.to_string(),
            status: "authorized".to_string(),
            reserved_microcredits: req.reserved_microcredits,
            captured_microcredits: 0,
            balance_microcredits,
            expires_at,
        })
    }

    pub async fn billing_capture(
        &self,
        req: BillingCaptureRequest,
    ) -> Result<BillingCaptureResponse> {
        let request_id = req.request_id.trim();
        if request_id.is_empty() {
            return Err(anyhow!("request_id must not be empty"));
        }
        let model = req.model.trim();
        if model.is_empty() {
            return Err(anyhow!("model must not be empty"));
        }
        let input_tokens = req.input_tokens.max(0);
        let cached_input_tokens = req.cached_input_tokens.max(0).min(input_tokens);
        let billable_input_tokens = input_tokens.saturating_sub(cached_input_tokens);
        let output_tokens = req.output_tokens.max(0);
        let reasoning_tokens = req.reasoning_tokens.max(0);
        let billable_output_tokens = output_tokens.max(reasoning_tokens);

        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start billing capture transaction")?;
        let authorization = self
            .fetch_authorization_for_update(&mut tx, req.tenant_id, request_id)
            .await?
            .ok_or_else(|| anyhow!("authorization not found"))?;

        if authorization_capture_is_idempotent(&authorization.status) {
            let balance_microcredits = self
                .current_credit_balance_for_update(&mut tx, req.tenant_id)
                .await?;
            tx.commit()
                .await
                .context("failed to commit billing capture transaction")?;
            return Ok(BillingCaptureResponse {
                authorization_id: authorization.id,
                tenant_id: authorization.tenant_id,
                request_id: authorization.request_id,
                status: authorization.status,
                reserved_microcredits: authorization.reserved_microcredits,
                captured_microcredits: authorization.captured_microcredits,
                charged_microcredits: authorization.captured_microcredits,
                balance_microcredits,
            });
        }
        if authorization.status != "authorized" {
            return Err(anyhow!(
                "billing authorization is in invalid status: {}",
                authorization.status
            ));
        }

        let session_key = req
            .session_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| authorization_session_key(authorization.meta_json.as_ref()));
        let request_kind = match BillingRequestKind::from_optional(req.request_kind.as_deref()) {
            BillingRequestKind::Unknown => authorization_request_kind(authorization.meta_json.as_ref())
                .unwrap_or(BillingRequestKind::Unknown),
            value => value,
        };
        let existing_session = match session_key.as_deref() {
            Some(value) => {
                self.fetch_billing_session_for_update(&mut tx, req.tenant_id, value)
                    .await?
            }
            None => None,
        };
        let pricing_decision = self
            .resolve_model_pricing_for_request(
                model,
                req.api_key_id,
                request_kind,
                existing_session
                    .as_ref()
                    .map(|record| BillingPricingBand::from_optional(Some(record.pricing_band.as_str())))
                    .or_else(|| authorization_pricing_band(authorization.meta_json.as_ref())),
                Some(input_tokens),
                BillingResolutionPhase::Capture,
            )
            .await?;
        let pricing = pricing_decision.pricing.clone();
        let charged_microcredits = calculate_charged_microcredits(
            input_tokens,
            cached_input_tokens,
            billable_output_tokens,
            &pricing,
        );

        let extra_charge_microcredits = charged_microcredits
            .saturating_sub(authorization.reserved_microcredits)
            .max(0);
        let balance_microcredits = self
            .apply_credit_delta_inner(
                &mut tx,
                CreditDeltaParams {
                    tenant_id: req.tenant_id,
                    api_key_id: req.api_key_id.or(authorization.api_key_id),
                    event_type: "capture",
                    delta_microcredits: -extra_charge_microcredits,
                    request_id: Some(request_id.to_string()),
                    model: Some(model.to_string()),
                    input_tokens: Some(input_tokens),
                    output_tokens: Some(output_tokens),
                    meta_json: Some(json!({
                        "phase": "capture",
                        "authorization_id": authorization.id,
                        "session_key": session_key,
                        "request_kind": request_kind.as_str(),
                        "pricing_band": pricing_decision.band.as_str(),
                        "pricing_rule_id": pricing_decision.matched_rule_id.map(|id| id.to_string()),
                        "reserved_microcredits": authorization.reserved_microcredits,
                        "charged_microcredits": charged_microcredits,
                        "extra_charge_microcredits": extra_charge_microcredits,
                        "pricing_source": pricing.source,
                        "input_price_microcredits": pricing.input_price_microcredits,
                        "cached_input_price_microcredits": pricing.cached_input_price_microcredits,
                        "output_price_microcredits": pricing.output_price_microcredits,
                        "billable_input_tokens": billable_input_tokens,
                        "cached_input_tokens": cached_input_tokens,
                        "reasoning_tokens": reasoning_tokens,
                        "billable_output_tokens": billable_output_tokens,
                        "is_stream": req.is_stream,
                    })),
                    now,
                },
            )
            .await?;

        sqlx::query(
            r#"
            UPDATE tenant_credit_authorizations
            SET api_key_id = COALESCE(api_key_id, $2),
                model = COALESCE(model, $3),
                captured_microcredits = $4,
                status = 'captured',
                meta_json = COALESCE(meta_json, '{}'::jsonb) || $5::jsonb,
                updated_at = $6
            WHERE id = $1
            "#,
        )
        .bind(authorization.id)
        .bind(req.api_key_id.or(authorization.api_key_id))
        .bind(authorization.model.as_deref().unwrap_or(model))
        .bind(charged_microcredits)
        .bind(json!({
            "phase": "capture",
            "session_key": session_key,
            "request_kind": request_kind.as_str(),
            "pricing_band": pricing_decision.band.as_str(),
            "pricing_rule_id": pricing_decision.matched_rule_id.map(|id| id.to_string()),
            "pricing_source": pricing.source,
            "charged_microcredits": charged_microcredits,
            "captured_at": now,
            "is_stream": req.is_stream,
        }))
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to update billing authorization capture status")?;

        if let Some(value) = session_key.as_deref() {
            self.upsert_billing_session(
                &mut tx,
                req.tenant_id,
                value,
                model,
                pricing_decision.band,
                now,
            )
            .await?;
        }

        tx.commit()
            .await
            .context("failed to commit billing capture transaction")?;

        Ok(BillingCaptureResponse {
            authorization_id: authorization.id,
            tenant_id: authorization.tenant_id,
            request_id: authorization.request_id,
            status: "captured".to_string(),
            reserved_microcredits: authorization.reserved_microcredits,
            captured_microcredits: charged_microcredits,
            charged_microcredits,
            balance_microcredits,
        })
    }

    pub async fn billing_release(
        &self,
        req: BillingReleaseRequest,
    ) -> Result<BillingReleaseResponse> {
        let request_id = req.request_id.trim();
        if request_id.is_empty() {
            return Err(anyhow!("request_id must not be empty"));
        }
        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start billing release transaction")?;
        let authorization = self
            .fetch_authorization_for_update(&mut tx, req.tenant_id, request_id)
            .await?
            .ok_or_else(|| anyhow!("authorization not found"))?;

        let release_microcredits = authorization
            .reserved_microcredits
            .saturating_sub(authorization.captured_microcredits)
            .max(0);
        if authorization.status == "released" {
            let balance_microcredits = self
                .current_credit_balance_for_update(&mut tx, req.tenant_id)
                .await?;
            tx.commit()
                .await
                .context("failed to commit billing release transaction")?;
            return Ok(BillingReleaseResponse {
                authorization_id: authorization.id,
                tenant_id: authorization.tenant_id,
                request_id: authorization.request_id,
                status: authorization.status,
                reserved_microcredits: authorization.reserved_microcredits,
                captured_microcredits: authorization.captured_microcredits,
                released_microcredits: release_microcredits,
                balance_microcredits,
            });
        }

        let balance_microcredits = if release_microcredits > 0 {
            let mut release_meta = serde_json::Map::new();
            release_meta.insert("phase".to_string(), json!("release"));
            release_meta.insert("authorization_id".to_string(), json!(authorization.id));
            release_meta.insert("request_id".to_string(), json!(authorization.request_id));
            release_meta.insert(
                "release_microcredits".to_string(),
                json!(release_microcredits),
            );
            release_meta.insert("is_stream".to_string(), json!(req.is_stream));
            if let Some(reason) = req.release_reason.as_ref() {
                release_meta.insert("release_reason".to_string(), json!(reason));
            }
            if let Some(status_code) = req.upstream_status_code {
                release_meta.insert("upstream_status_code".to_string(), json!(status_code));
            }
            if let Some(error_code) = req.upstream_error_code.as_ref() {
                release_meta.insert("upstream_error_code".to_string(), json!(error_code));
            }
            if let Some(action) = req.failover_action.as_ref() {
                release_meta.insert("failover_action".to_string(), json!(action));
            }
            if let Some(reason_class) = req.failover_reason_class.as_ref() {
                release_meta.insert("failover_reason_class".to_string(), json!(reason_class));
            }
            if let Some(action) = req.recovery_action.as_ref() {
                release_meta.insert("recovery_action".to_string(), json!(action));
            }
            if let Some(outcome) = req.recovery_outcome.as_ref() {
                release_meta.insert("recovery_outcome".to_string(), json!(outcome));
            }
            if let Some(attempted) = req.cross_account_failover_attempted {
                release_meta.insert(
                    "cross_account_failover_attempted".to_string(),
                    json!(attempted),
                );
            }
            self.apply_credit_delta_inner(
                &mut tx,
                CreditDeltaParams {
                    tenant_id: req.tenant_id,
                    api_key_id: authorization.api_key_id,
                    event_type: "release",
                    delta_microcredits: release_microcredits,
                    request_id: None,
                    model: authorization.model.clone(),
                    input_tokens: None,
                    output_tokens: None,
                    meta_json: Some(serde_json::Value::Object(release_meta)),
                    now,
                },
            )
            .await?
        } else {
            self.current_credit_balance_for_update(&mut tx, req.tenant_id)
                .await?
        };

        sqlx::query(
            r#"
            UPDATE tenant_credit_authorizations
            SET status = 'released',
                updated_at = $2
            WHERE id = $1
            "#,
        )
        .bind(authorization.id)
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to update billing authorization release status")?;

        tx.commit()
            .await
            .context("failed to commit billing release transaction")?;

        Ok(BillingReleaseResponse {
            authorization_id: authorization.id,
            tenant_id: authorization.tenant_id,
            request_id: authorization.request_id,
            status: "released".to_string(),
            reserved_microcredits: authorization.reserved_microcredits,
            captured_microcredits: authorization.captured_microcredits,
            released_microcredits: release_microcredits,
            balance_microcredits,
        })
    }

    pub async fn billing_reconcile_request_fact(
        &self,
        req: BillingReconcileFactRequest,
    ) -> Result<BillingReconcileStats> {
        let request_id = req.request_id.trim();
        if request_id.is_empty() {
            return Ok(BillingReconcileStats::default());
        }

        let mut stats = BillingReconcileStats::default();
        let Some(authorization) = self.fetch_authorization(req.tenant_id, request_id).await? else {
            return Ok(stats);
        };
        stats.scanned = 1;

        let model = req
            .model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| {
                authorization
                    .model
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
            });
        let input_tokens = req.input_tokens.map(|value| value.max(0));
        let cached_input_tokens = req.cached_input_tokens.map(|value| value.max(0));
        let output_tokens = req.output_tokens.map(|value| value.max(0));
        let reasoning_tokens = req.reasoning_tokens.map(|value| value.max(0));

        if authorization.status == "authorized" {
            if let (Some(model), Some(input_tokens), Some(output_tokens)) =
                (model.as_deref(), input_tokens, output_tokens)
            {
                let _ = self
                    .billing_capture(BillingCaptureRequest {
                        tenant_id: req.tenant_id,
                        api_key_id: req.api_key_id.or(authorization.api_key_id),
                        request_id: request_id.to_string(),
                        model: model.to_string(),
                        session_key: authorization_session_key(authorization.meta_json.as_ref()),
                        request_kind: authorization_request_kind(authorization.meta_json.as_ref())
                            .map(|value| value.as_str().to_string()),
                        input_tokens,
                        cached_input_tokens: cached_input_tokens.unwrap_or(0),
                        output_tokens,
                        reasoning_tokens: reasoning_tokens.unwrap_or(0),
                        is_stream: false,
                    })
                    .await?;
            }
        } else if (authorization.status == "captured" || authorization.status == "released")
            && model.is_some()
            && input_tokens.is_some()
            && output_tokens.is_some()
        {
            let adjusted = self
                .reconcile_adjust_captured_amount(
                    req.tenant_id,
                    request_id,
                    model.as_deref().unwrap_or_default(),
                    input_tokens.unwrap_or(0),
                    cached_input_tokens.unwrap_or(0),
                    output_tokens.unwrap_or(0),
                    reasoning_tokens.unwrap_or(0),
                    req.api_key_id,
                )
                .await?;
            if adjusted {
                stats.adjusted = stats.adjusted.saturating_add(1);
            }
        }

        let Some(latest) = self.fetch_authorization(req.tenant_id, request_id).await? else {
            return Ok(stats);
        };
        if latest.status != "released" {
            let _ = self
                .billing_release(BillingReleaseRequest {
                    tenant_id: req.tenant_id,
                    request_id: request_id.to_string(),
                    is_stream: false,
                    release_reason: None,
                    upstream_status_code: None,
                    upstream_error_code: None,
                    failover_action: None,
                    failover_reason_class: None,
                    recovery_action: None,
                    recovery_outcome: None,
                    cross_account_failover_attempted: None,
                })
                .await?;
            stats.released_authorizations = stats.released_authorizations.saturating_add(1);
        }

        Ok(stats)
    }

    pub async fn billing_reconcile_once(
        &self,
        req: BillingReconcileRequest,
    ) -> Result<BillingReconcileStats> {
        let stale_sec = req.stale_sec.clamp(60, 30 * 24 * 60 * 60);
        let batch_size = req.batch_size.clamp(1, 5000);
        let now = Utc::now();
        let stale_before = now - chrono::Duration::seconds(stale_sec as i64);
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start billing reconcile transaction")?;
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                tenant_id,
                api_key_id,
                request_id,
                model,
                reserved_microcredits,
                captured_microcredits,
                status,
                expires_at,
                meta_json
            FROM tenant_credit_authorizations
            WHERE
                (status = 'authorized' AND expires_at <= $1)
                OR (status = 'captured' AND updated_at <= $2)
            ORDER BY updated_at ASC
            LIMIT $3
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(now)
        .bind(stale_before)
        .bind(batch_size as i64)
        .fetch_all(tx.as_mut())
        .await
        .context("failed to query stale billing authorizations for reconcile")?;

        let mut stats = BillingReconcileStats::default();
        for row in rows {
            let authorization = BillingAuthorizationRecord {
                id: row.try_get("id")?,
                tenant_id: row.try_get("tenant_id")?,
                request_id: row.try_get("request_id")?,
                api_key_id: row.try_get("api_key_id")?,
                model: row.try_get("model")?,
                reserved_microcredits: row.try_get("reserved_microcredits")?,
                captured_microcredits: row.try_get("captured_microcredits")?,
                status: row.try_get("status")?,
                expires_at: row.try_get("expires_at")?,
                meta_json: Some(row.try_get("meta_json")?),
            };
            stats.scanned = stats.scanned.saturating_add(1);

            let release_microcredits = authorization
                .reserved_microcredits
                .saturating_sub(authorization.captured_microcredits)
                .max(0);
            if release_microcredits > 0 {
                let _ = self
                    .apply_credit_delta_inner(
                        &mut tx,
                        CreditDeltaParams {
                            tenant_id: authorization.tenant_id,
                            api_key_id: authorization.api_key_id,
                            event_type: "adjust",
                            delta_microcredits: release_microcredits,
                            request_id: None,
                            model: authorization.model.clone(),
                            input_tokens: None,
                            output_tokens: None,
                            meta_json: Some(json!({
                                "phase": "reconcile_adjust",
                                "reason": "stale_authorization_release",
                                "authorization_id": authorization.id,
                                "request_id": authorization.request_id.clone(),
                                "previous_status": authorization.status.clone(),
                                "release_microcredits": release_microcredits,
                            })),
                            now,
                        },
                    )
                    .await?;
                stats.adjusted = stats.adjusted.saturating_add(1);
                stats.adjusted_microcredits_total = stats
                    .adjusted_microcredits_total
                    .saturating_add(release_microcredits);
            }

            sqlx::query(
                r#"
                UPDATE tenant_credit_authorizations
                SET status = 'released',
                    meta_json = COALESCE(meta_json, '{}'::jsonb) || $2::jsonb,
                    updated_at = $3
                WHERE id = $1
                "#,
            )
            .bind(authorization.id)
            .bind(json!({
                "phase": "reconcile_release",
                "reason": "stale_authorization_close",
                "reconciled_at": now,
                "previous_status": authorization.status.clone(),
                "release_microcredits": release_microcredits,
            }))
            .bind(now)
            .execute(tx.as_mut())
            .await
            .context("failed to update billing authorization status in reconcile")?;
            stats.released_authorizations = stats.released_authorizations.saturating_add(1);
        }

        tx.commit()
            .await
            .context("failed to commit billing reconcile transaction")?;
        Ok(stats)
    }

    #[allow(clippy::too_many_arguments)]
    async fn reconcile_adjust_captured_amount(
        &self,
        tenant_id: Uuid,
        request_id: &str,
        model: &str,
        input_tokens: i64,
        cached_input_tokens: i64,
        output_tokens: i64,
        reasoning_tokens: i64,
        api_key_id: Option<Uuid>,
    ) -> Result<bool> {
        let normalized_input_tokens = input_tokens.max(0);
        let normalized_cached_input_tokens = cached_input_tokens.max(0).min(normalized_input_tokens);
        let billable_input_tokens =
            normalized_input_tokens.saturating_sub(normalized_cached_input_tokens);
        let normalized_reasoning_tokens = reasoning_tokens.max(0);
        let normalized_output_tokens = output_tokens.max(0);
        let billable_output_tokens = normalized_output_tokens.max(normalized_reasoning_tokens);

        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start reconcile adjust transaction")?;
        let Some(authorization) = self
            .fetch_authorization_for_update(&mut tx, tenant_id, request_id)
            .await?
        else {
            tx.commit()
                .await
                .context("failed to commit reconcile adjust noop transaction")?;
            return Ok(false);
        };

        let request_kind = authorization_request_kind(authorization.meta_json.as_ref())
            .unwrap_or(BillingRequestKind::Unknown);
        let pricing_decision = self
            .resolve_model_pricing_for_request(
                model,
                authorization.api_key_id,
                request_kind,
                authorization_pricing_band(authorization.meta_json.as_ref()),
                Some(normalized_input_tokens),
                BillingResolutionPhase::Capture,
            )
            .await?;
        let pricing = pricing_decision.pricing.clone();
        let expected_captured_microcredits = calculate_charged_microcredits(
            normalized_input_tokens,
            normalized_cached_input_tokens,
            billable_output_tokens,
            &pricing,
        );

        if authorization.captured_microcredits == expected_captured_microcredits {
            tx.commit()
                .await
                .context("failed to commit reconcile adjust noop transaction")?;
            return Ok(false);
        }

        let delta_microcredits = authorization
            .captured_microcredits
            .saturating_sub(expected_captured_microcredits);
        if delta_microcredits != 0 {
            let _ = self
                .apply_credit_delta_inner(
                    &mut tx,
                    CreditDeltaParams {
                        tenant_id,
                        api_key_id: api_key_id.or(authorization.api_key_id),
                        event_type: "adjust",
                        delta_microcredits,
                        request_id: None,
                        model: authorization
                            .model
                            .clone()
                            .or_else(|| Some(model.to_string())),
                        input_tokens: Some(normalized_input_tokens),
                        output_tokens: Some(normalized_output_tokens),
                        meta_json: Some(json!({
                            "phase": "reconcile_adjust",
                            "reason": "request_log_capture_mismatch",
                            "request_id": request_id,
                            "authorization_id": authorization.id,
                            "request_kind": request_kind.as_str(),
                            "pricing_band": pricing_decision.band.as_str(),
                            "pricing_rule_id": pricing_decision.matched_rule_id.map(|id| id.to_string()),
                            "previous_captured_microcredits": authorization.captured_microcredits,
                            "expected_captured_microcredits": expected_captured_microcredits,
                            "delta_microcredits": delta_microcredits,
                            "pricing_source": pricing.source,
                            "input_price_microcredits": pricing.input_price_microcredits,
                            "cached_input_price_microcredits": pricing.cached_input_price_microcredits,
                            "output_price_microcredits": pricing.output_price_microcredits,
                            "billable_input_tokens": billable_input_tokens,
                            "cached_input_tokens": normalized_cached_input_tokens,
                            "reasoning_tokens": normalized_reasoning_tokens,
                            "billable_output_tokens": billable_output_tokens,
                        })),
                        now,
                    },
                )
                .await?;
        }

        sqlx::query(
            r#"
            UPDATE tenant_credit_authorizations
            SET captured_microcredits = $2,
                meta_json = COALESCE(meta_json, '{}'::jsonb) || $3::jsonb,
                updated_at = $4
            WHERE id = $1
            "#,
        )
        .bind(authorization.id)
        .bind(expected_captured_microcredits)
        .bind(json!({
            "phase": "reconcile_adjust",
            "reason": "request_log_capture_mismatch",
            "request_id": request_id,
            "request_kind": request_kind.as_str(),
            "pricing_band": pricing_decision.band.as_str(),
            "pricing_rule_id": pricing_decision.matched_rule_id.map(|id| id.to_string()),
            "previous_captured_microcredits": authorization.captured_microcredits,
            "expected_captured_microcredits": expected_captured_microcredits,
            "pricing_source": pricing.source,
            "reconciled_at": now,
        }))
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to update authorization captured_microcredits in reconcile adjust")?;

        tx.commit()
            .await
            .context("failed to commit reconcile adjust transaction")?;
        Ok(true)
    }
}

fn authorization_reusable_for_authorize(status: &str) -> bool {
    status == "authorized"
}

fn authorization_capture_is_idempotent(status: &str) -> bool {
    status == "captured"
}

const PRICING_PER_MILLION_TOKENS_SCALE: i128 = 1_000_000;

fn charge_tokens_by_per_million_price(tokens: i64, price_per_million_microcredits: i64) -> i64 {
    let normalized_tokens = tokens.max(0) as i128;
    let normalized_price = price_per_million_microcredits.max(0) as i128;
    if normalized_tokens == 0 || normalized_price == 0 {
        return 0;
    }
    // Round to nearest microcredit to avoid systematic under/over charging on small requests.
    let numerator = normalized_tokens
        .saturating_mul(normalized_price)
        .saturating_add(PRICING_PER_MILLION_TOKENS_SCALE / 2);
    let charged = numerator / PRICING_PER_MILLION_TOKENS_SCALE;
    charged.clamp(0, i64::MAX as i128) as i64
}

fn calculate_charged_microcredits(
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    pricing: &BillingPricingResolved,
) -> i64 {
    let normalized_input_tokens = input_tokens.max(0);
    let normalized_cached_input_tokens = cached_input_tokens.max(0).min(normalized_input_tokens);
    let billable_input_tokens = normalized_input_tokens.saturating_sub(normalized_cached_input_tokens);
    let normalized_output_tokens = output_tokens.max(0);

    let input_charge =
        charge_tokens_by_per_million_price(billable_input_tokens, pricing.input_price_microcredits);
    let cached_input_charge = charge_tokens_by_per_million_price(
        normalized_cached_input_tokens,
        pricing.cached_input_price_microcredits,
    );
    let output_charge =
        charge_tokens_by_per_million_price(normalized_output_tokens, pricing.output_price_microcredits);
    input_charge
        .saturating_add(cached_input_charge)
        .saturating_add(output_charge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_authorize_reuses_only_authorized_status() {
        assert!(authorization_reusable_for_authorize("authorized"));
        assert!(!authorization_reusable_for_authorize("captured"));
        assert!(!authorization_reusable_for_authorize("released"));
    }

    #[test]
    fn billing_capture_is_idempotent_only_for_captured_status() {
        assert!(authorization_capture_is_idempotent("captured"));
        assert!(!authorization_capture_is_idempotent("authorized"));
        assert!(!authorization_capture_is_idempotent("released"));
    }
}
