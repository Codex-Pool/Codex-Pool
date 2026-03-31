#[tokio::test]
async fn oauth_import_and_status_routes_work() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([3_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-refresh-token")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "oauth-a",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "refresh_token": "rt-demo"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);
    let import_body = to_bytes(import_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let import_json: Value = serde_json::from_slice(&import_body).unwrap();
    let account_id = import_json["id"].as_str().unwrap().to_string();

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/{account_id}/oauth/status"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_body = to_bytes(status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_json: Value = serde_json::from_slice(&status_body).unwrap();
    assert_eq!(status_json["auth_provider"], "oauth_refresh_token");
    assert_eq!(status_json["last_refresh_status"], "ok");
    assert!(status_json["token_family_id"].is_string());
    assert_eq!(status_json["token_version"], 1);
    assert_eq!(status_json["refresh_reused_detected"], false);

    let snapshot_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/data-plane/snapshot")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(snapshot_response.status(), StatusCode::OK);
    let snapshot_body = to_bytes(snapshot_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let snapshot_json: Value = serde_json::from_slice(&snapshot_body).unwrap();
    assert_eq!(
        snapshot_json["accounts"][0]["bearer_token"],
        "access-from-oauth"
    );
}

#[tokio::test]
async fn oauth_statuses_route_returns_cached_rate_limit_metadata() {
    let account_id = Uuid::new_v4();
    let store = Arc::new(RateLimitApiTestStore::with_status(
        sample_cached_oauth_status(account_id),
    ));
    let app = build_app_with_store(store);
    let admin_token = login_admin_token(&app).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/statuses")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "account_ids": [account_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["items"][0]["account_id"], account_id.to_string());
    assert_eq!(
        value["items"][0]["rate_limits"][0]["limit_id"],
        "five_hours"
    );
    assert!(value["items"][0]["rate_limits_fetched_at"].is_string());
    assert!(value["items"][0]["rate_limits_expires_at"].is_string());
}

#[tokio::test]
async fn oauth_inventory_routes_expose_vault_admission_state() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([31_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(InventoryProbeOAuthTokenClient {
            rate_limits: vec![OAuthRateLimitSnapshot {
                limit_id: Some("five_hours".to_string()),
                limit_name: Some("5 hours".to_string()),
                primary: Some(OAuthRateLimitWindow {
                    used_percent: 28.0,
                    window_minutes: Some(300),
                    resets_at: Some(chrono::Utc::now() + chrono::Duration::minutes(25)),
                }),
                secondary: None,
            }],
        }),
        Some(cipher),
    );
    store
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "inventory-ready".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-inventory-ready".to_string(),
            fallback_access_token: Some("ak-inventory-ready".to_string()),
            fallback_token_expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(2)),
            chatgpt_account_id: Some("acct_inventory_ready".to_string()),
            mode: None,
            enabled: None,
            priority: None,
            chatgpt_plan_type: Some("free".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();
    store
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "inventory-needs-refresh".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-inventory-needs-refresh".to_string(),
            fallback_access_token: None,
            fallback_token_expires_at: None,
            chatgpt_account_id: Some("acct_inventory_needs_refresh".to_string()),
            mode: None,
            enabled: None,
            priority: None,
            chatgpt_plan_type: Some("free".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let summary_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts/oauth/inventory/summary")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(summary_response.status(), StatusCode::OK);
    let summary_body = to_bytes(summary_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let summary_json: Value = serde_json::from_slice(&summary_body).unwrap();
    assert_eq!(summary_json["total"], 2);
    assert_eq!(summary_json["ready"], 1);
    assert_eq!(summary_json["needs_refresh"], 1);
    assert_eq!(summary_json["no_quota"], 0);

    let records_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts/oauth/inventory/records")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(records_response.status(), StatusCode::OK);
    let records_body = to_bytes(records_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let mut records_json: Vec<Value> = serde_json::from_slice(&records_body).unwrap();
    records_json.sort_by(|left, right| {
        left["label"]
            .as_str()
            .cmp(&right["label"].as_str())
    });
    assert_eq!(records_json.len(), 2);
    assert_eq!(records_json[0]["label"], "inventory-needs-refresh");
    assert_eq!(records_json[0]["vault_status"], "needs_refresh");
    assert_eq!(
        records_json[0]["admission_error_code"],
        "missing_access_token_fallback"
    );
    assert_eq!(records_json[1]["label"], "inventory-ready");
    assert_eq!(records_json[1]["vault_status"], "ready");
    assert_eq!(records_json[1]["admission_source"], "fallback_access_token");
    assert!(records_json[1]["admission_checked_at"].is_string());
}

#[tokio::test]
async fn oauth_inventory_batch_action_marks_records_failed() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([7_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    store
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "inventory-mark-failed".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-inventory-mark-failed".to_string(),
            fallback_access_token: None,
            fallback_token_expires_at: None,
            chatgpt_account_id: Some("acct_inventory_mark_failed".to_string()),
            mode: None,
            enabled: None,
            priority: None,
            chatgpt_plan_type: Some("free".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let before_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts/oauth/inventory/records")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(before_response.status(), StatusCode::OK);
    let before_body = to_bytes(before_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let before_records: Vec<Value> = serde_json::from_slice(&before_body).unwrap();
    let target_record = before_records
        .iter()
        .find(|item| item["label"] == "inventory-mark-failed")
        .cloned()
        .expect("inventory target record should exist");
    let target_record_id = target_record["id"].as_str().unwrap();
    assert_eq!(target_record["vault_status"], "needs_refresh");

    let action_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/inventory/batch-actions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "mark_failed",
                        "record_ids": [target_record_id],
                        "reason": "operator_retired"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(action_response.status(), StatusCode::OK);
    let action_body = to_bytes(action_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let action_json: Value = serde_json::from_slice(&action_body).unwrap();
    assert_eq!(action_json["total"], 1);
    assert_eq!(action_json["success_count"], 1);
    assert_eq!(action_json["failed_count"], 0);

    let after_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts/oauth/inventory/records")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after_response.status(), StatusCode::OK);
    let after_body = to_bytes(after_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let after_records: Vec<Value> = serde_json::from_slice(&after_body).unwrap();
    let updated_record = after_records
        .iter()
        .find(|item| item["id"] == target_record_id)
        .cloned()
        .expect("updated inventory target record should exist");
    assert_eq!(updated_record["vault_status"], "failed");
    assert_eq!(updated_record["retryable"], false);
    assert_eq!(updated_record["terminal_reason"], "operator_retired");
}

#[tokio::test]
async fn oauth_runtime_and_health_summary_routes_reflect_pool_state() {
    let store = InMemoryStore::default();
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    for label in ["runtime-summary-a", "runtime-summary-b"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/upstream-accounts")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::from(
                        json!({
                            "label": label,
                            "mode": "chat_gpt_session",
                            "base_url": "https://chatgpt.com/backend-api/codex",
                            "bearer_token": format!("token-{label}"),
                            "enabled": true,
                            "priority": 100
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = to_bytes(list_response.into_body(), usize::MAX).await.unwrap();
    let list_json: Vec<Value> = serde_json::from_slice(&list_body).unwrap();
    let account_id = list_json[0]["id"].as_str().unwrap().to_string();

    let live_result = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/health/live-result"
                ))
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::from(
                    json!({
                        "status": "failed",
                        "source": "active",
                        "status_code": 429,
                        "error_code": "rate_limited",
                        "error_message": "too many requests"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(live_result.status(), StatusCode::OK);

    let runtime_summary = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts/runtime/summary")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(runtime_summary.status(), StatusCode::OK);
    let runtime_body = to_bytes(runtime_summary.into_body(), usize::MAX)
        .await
        .unwrap();
    let runtime_json: Value = serde_json::from_slice(&runtime_body).unwrap();
    assert_eq!(runtime_json["total"], 2);
    assert_eq!(runtime_json["quarantine"], 1);
    assert_eq!(runtime_json["active"], 1);
    assert_eq!(runtime_json["legacy_bearer"], 2);

    let health_summary = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts/health/signals/summary")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(health_summary.status(), StatusCode::OK);
    let health_body = to_bytes(health_summary.into_body(), usize::MAX)
        .await
        .unwrap();
    let health_json: Value = serde_json::from_slice(&health_body).unwrap();
    assert_eq!(health_json["total"], 2);
    assert_eq!(health_json["live_result_failed"], 1);
    assert_eq!(health_json["quarantine_signals"], 1);
}

#[tokio::test]
async fn account_pool_routes_unify_runtime_and_inventory_records() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([41_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(InventoryProbeOAuthTokenClient {
            rate_limits: vec![OAuthRateLimitSnapshot {
                limit_id: Some("five_hours".to_string()),
                limit_name: Some("5 hours".to_string()),
                primary: Some(OAuthRateLimitWindow {
                    used_percent: 28.0,
                    window_minutes: Some(300),
                    resets_at: Some(chrono::Utc::now() + chrono::Duration::minutes(25)),
                }),
                secondary: None,
            }],
        }),
        Some(cipher),
    );
    store
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "pool-inventory-ready".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-pool-inventory-ready".to_string(),
            fallback_access_token: Some("ak-pool-inventory-ready".to_string()),
            fallback_token_expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(2)),
            chatgpt_account_id: Some("acct_pool_inventory_ready".to_string()),
            mode: None,
            enabled: None,
            priority: None,
            chatgpt_plan_type: Some("pro".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();
    store
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "pool-inventory-needs-refresh".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-pool-inventory-needs-refresh".to_string(),
            fallback_access_token: None,
            fallback_token_expires_at: None,
            chatgpt_account_id: Some("acct_pool_inventory_needs_refresh".to_string()),
            mode: None,
            enabled: None,
            priority: None,
            chatgpt_plan_type: Some("free".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let routable_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "pool-runtime-routable",
                        "mode": "open_ai_api_key",
                        "base_url": "https://api.openai.com/v1",
                        "bearer_token": "token-pool-runtime-routable",
                        "enabled": true,
                        "priority": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(routable_response.status(), StatusCode::OK);

    let cooling_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "pool-runtime-cooling",
                        "mode": "chat_gpt_session",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "bearer_token": "token-pool-runtime-cooling",
                        "enabled": true,
                        "priority": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cooling_response.status(), StatusCode::OK);

    let runtime_accounts_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let runtime_accounts_body = to_bytes(runtime_accounts_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let runtime_accounts_json: Vec<Value> = serde_json::from_slice(&runtime_accounts_body).unwrap();
    let cooling_account_id = runtime_accounts_json
        .iter()
        .find(|item| item["label"] == "pool-runtime-cooling")
        .and_then(|item| item["id"].as_str())
        .unwrap()
        .to_string();

    let live_result = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{cooling_account_id}/health/live-result"
                ))
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::from(
                    json!({
                        "status": "failed",
                        "source": "passive",
                        "status_code": 429,
                        "error_code": "rate_limited",
                        "error_message": "too many requests"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(live_result.status(), StatusCode::OK);

    let inventory_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts/oauth/inventory/records")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let inventory_body = to_bytes(inventory_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let inventory_records: Vec<Value> = serde_json::from_slice(&inventory_body).unwrap();
    let inventory_failed_id = inventory_records
        .iter()
        .find(|item| item["label"] == "pool-inventory-needs-refresh")
        .and_then(|item| item["id"].as_str())
        .unwrap()
        .to_string();

    let mark_failed_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/inventory/batch-actions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "mark_failed",
                        "record_ids": [inventory_failed_id],
                        "reason": "operator_retired_invalid_refresh_token"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mark_failed_response.status(), StatusCode::OK);

    let summary_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/account-pool/summary")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(summary_response.status(), StatusCode::OK);
    let summary_body = to_bytes(summary_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let summary_json: Value = serde_json::from_slice(&summary_body).unwrap();
    assert_eq!(summary_json["total"], 4);
    assert_eq!(summary_json["routable"], 1);
    assert_eq!(summary_json["cooling"], 1);
    assert_eq!(summary_json["inventory"], 1);
    assert_eq!(summary_json["pending_delete"], 1);
    assert_eq!(summary_json["quota"], 1);
    assert_eq!(summary_json["admin"], 1);

    let records_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/account-pool/accounts")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(records_response.status(), StatusCode::OK);
    let records_body = to_bytes(records_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let account_pool_records: Vec<Value> = serde_json::from_slice(&records_body).unwrap();

    let routable_record = account_pool_records
        .iter()
        .find(|item| item["label"] == "pool-runtime-routable")
        .unwrap();
    assert_eq!(routable_record["record_scope"], "runtime");
    assert_eq!(routable_record["operator_state"], "routable");
    assert_eq!(routable_record["reason_class"], "healthy");
    assert_eq!(routable_record["route_eligible"], true);
    assert_eq!(routable_record["health_freshness"], "unknown");
    assert!(routable_record["last_probe_at"].is_null());
    assert!(routable_record["last_probe_outcome"].is_null());

    let cooling_record = account_pool_records
        .iter()
        .find(|item| item["label"] == "pool-runtime-cooling")
        .unwrap();
    assert_eq!(cooling_record["record_scope"], "runtime");
    assert_eq!(cooling_record["operator_state"], "cooling");
    assert_eq!(cooling_record["reason_class"], "quota");
    assert_eq!(cooling_record["reason_code"], "rate_limited");
    assert_eq!(cooling_record["health_freshness"], "stale");
    assert!(cooling_record["last_probe_at"].is_null());
    assert!(cooling_record["last_probe_outcome"].is_null());

    let inventory_ready_record = account_pool_records
        .iter()
        .find(|item| item["label"] == "pool-inventory-ready")
        .unwrap();
    assert_eq!(inventory_ready_record["record_scope"], "inventory");
    assert_eq!(inventory_ready_record["operator_state"], "inventory");
    assert_eq!(inventory_ready_record["reason_class"], "healthy");
    assert_eq!(inventory_ready_record["health_freshness"], "unknown");

    let pending_delete_record = account_pool_records
        .iter()
        .find(|item| item["label"] == "pool-inventory-needs-refresh")
        .unwrap();
    assert_eq!(pending_delete_record["record_scope"], "inventory");
    assert_eq!(pending_delete_record["operator_state"], "pending_delete");
    assert_eq!(pending_delete_record["reason_class"], "admin");
    assert_eq!(
        pending_delete_record["reason_code"],
        "operator_retired_invalid_refresh_token"
    );
}

#[tokio::test]
async fn account_pool_actions_support_restore_reprobe_and_delete() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([42_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(InventoryProbeOAuthTokenClient {
            rate_limits: vec![OAuthRateLimitSnapshot {
                limit_id: Some("five_hours".to_string()),
                limit_name: Some("5 hours".to_string()),
                primary: Some(OAuthRateLimitWindow {
                    used_percent: 33.0,
                    window_minutes: Some(300),
                    resets_at: Some(chrono::Utc::now() + chrono::Duration::minutes(45)),
                }),
                secondary: None,
            }],
        }),
        Some(cipher),
    );
    store
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "pool-action-target".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-pool-action-target".to_string(),
            fallback_access_token: Some("ak-pool-action-target".to_string()),
            fallback_token_expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(2)),
            chatgpt_account_id: Some("acct_pool_action_target".to_string()),
            mode: None,
            enabled: None,
            priority: None,
            chatgpt_plan_type: Some("pro".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let before_records_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/account-pool/accounts")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let before_records_body = to_bytes(before_records_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let before_records: Vec<Value> = serde_json::from_slice(&before_records_body).unwrap();
    let target_id = before_records
        .iter()
        .find(|item| item["label"] == "pool-action-target")
        .and_then(|item| item["id"].as_str())
        .unwrap()
        .to_string();

    let mark_failed_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/inventory/batch-actions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "mark_failed",
                        "record_ids": [target_id],
                        "reason": "operator_retired_invalid_refresh_token"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mark_failed_response.status(), StatusCode::OK);

    let restore_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/account-pool/actions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "restore",
                        "record_ids": [target_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(restore_response.status(), StatusCode::OK);

    let restored_record_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/account-pool/accounts/{target_id}"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let restored_record_body = to_bytes(restored_record_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let restored_record: Value = serde_json::from_slice(&restored_record_body).unwrap();
    assert_eq!(restored_record["operator_state"], "inventory");
    assert!(restored_record["reason_code"].is_null());

    let reprobe_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/account-pool/actions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "reprobe",
                        "record_ids": [target_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reprobe_response.status(), StatusCode::OK);

    let reprobed_record_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/account-pool/accounts/{target_id}"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let reprobed_record_body = to_bytes(reprobed_record_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let reprobed_record: Value = serde_json::from_slice(&reprobed_record_body).unwrap();
    assert_eq!(reprobed_record["operator_state"], "inventory");
    assert_eq!(reprobed_record["reason_class"], "healthy");

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/account-pool/actions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "delete",
                        "record_ids": [target_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::OK);

    let after_delete_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/account-pool/accounts")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let after_delete_body = to_bytes(after_delete_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let after_delete_records: Vec<Value> = serde_json::from_slice(&after_delete_body).unwrap();
    assert!(after_delete_records
        .iter()
        .all(|item| item["label"] != "pool-action-target"));
}

#[tokio::test]
async fn account_pool_routes_expose_runtime_health_freshness() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([43_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(InventoryProbeOAuthTokenClient {
            rate_limits: vec![OAuthRateLimitSnapshot {
                limit_id: Some("five_hours".to_string()),
                limit_name: Some("5 hours".to_string()),
                primary: Some(OAuthRateLimitWindow {
                    used_percent: 18.0,
                    window_minutes: Some(300),
                    resets_at: Some(chrono::Utc::now() + chrono::Duration::minutes(35)),
                }),
                secondary: None,
            }],
        }),
        Some(cipher),
    );
    store
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "pool-runtime-health-fresh".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-pool-runtime-health-fresh".to_string(),
            fallback_access_token: Some("ak-pool-runtime-health-fresh".to_string()),
            fallback_token_expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(2)),
            chatgpt_account_id: Some("acct_pool_runtime_health_fresh".to_string()),
            mode: Some(codex_pool_core::model::UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: Some("pro".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();
    let activated = store.activate_oauth_refresh_token_vault().await.unwrap();
    assert_eq!(activated, 1);
    let account_id = store
        .list_upstream_accounts()
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("runtime account")
        .id;

    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/account-pool/accounts/{account_id}"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["operator_state"], "routable");
    assert_eq!(payload["health_freshness"], "fresh");
    assert_eq!(payload["last_probe_outcome"], "ok");
    assert!(payload["last_probe_at"].is_string());
}

#[tokio::test]
async fn oauth_status_route_returns_not_found_for_unknown_account() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/{}/oauth/status",
                    Uuid::new_v4()
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["error"]["code"], "not_found");
}

#[tokio::test]
async fn oauth_rate_limit_refresh_job_routes_work() {
    let store = Arc::new(RateLimitApiTestStore::default());
    let app = build_app_with_store(store.clone());
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/rate-limits/refresh-jobs")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let job_id = create_json["job_id"].as_str().unwrap().to_string();
    assert_eq!(create_json["status"], "queued");

    let mut latest = Value::Null;
    for _ in 0..20 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/upstream-accounts/oauth/rate-limits/refresh-jobs/{job_id}"
                    ))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        latest = serde_json::from_slice(&body).unwrap();
        if latest["status"] == "completed" {
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }

    assert_eq!(latest["status"], "completed");
    assert_eq!(latest["processed"], latest["total"]);
    assert_eq!(latest["failed_count"], 0);
    assert!(store.run_call_count() > 0);
}

#[tokio::test]
async fn oauth_rate_limit_refresh_job_summary_returns_not_found_for_unknown_job_id() {
    let store = Arc::new(RateLimitApiTestStore::default());
    let app = build_app_with_store(store);
    let admin_token = login_admin_token(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/rate-limits/refresh-jobs/{}",
                    Uuid::new_v4()
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["error"]["code"], "not_found");
}

#[tokio::test]
async fn internal_oauth_refresh_and_disable_routes_work() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([9_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-refresh-token")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "oauth-internal-a",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "refresh_token": "rt-internal-demo"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);
    let import_body = to_bytes(import_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let import_json: Value = serde_json::from_slice(&import_body).unwrap();
    let account_id = import_json["id"].as_str().unwrap().to_string();

    let unauthorized_refresh_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/oauth/refresh"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        unauthorized_refresh_response.status(),
        StatusCode::UNAUTHORIZED
    );

    let refresh_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/oauth/refresh"
                ))
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_body = to_bytes(refresh_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let refresh_json: Value = serde_json::from_slice(&refresh_body).unwrap();
    assert_eq!(refresh_json["account_id"], account_id);
    assert_eq!(refresh_json["last_refresh_status"], "ok");

    let disable_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/disable?reason=account_deactivated"
                ))
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(disable_response.status(), StatusCode::OK);
    let disable_body = to_bytes(disable_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let disable_json: Value = serde_json::from_slice(&disable_body).unwrap();
    assert_eq!(disable_json["id"], account_id);
    assert_eq!(disable_json["enabled"], false);

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/upstream-accounts/{account_id}/oauth/status"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_body = to_bytes(status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_json: Value = serde_json::from_slice(&status_body).unwrap();
    assert_eq!(status_json["pool_state"], "pending_purge");
    assert_eq!(status_json["pending_purge_reason"], "account_deactivated");
    assert!(status_json["pending_purge_at"].is_string());

    let snapshot_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/data-plane/snapshot")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(snapshot_response.status(), StatusCode::OK);
    let snapshot_body = to_bytes(snapshot_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let snapshot_json: Value = serde_json::from_slice(&snapshot_body).unwrap();
    let snapshot_account = snapshot_json["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == account_id)
        .cloned();
    assert!(
        snapshot_account.is_none(),
        "pending purge account should be removed from runtime snapshot"
    );
}

#[tokio::test]
async fn internal_oauth_refresh_route_returns_status_for_legacy_bearer_account() {
    let store = Arc::new(InMemoryStore::default());
    let account = store
        .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
            label: "legacy-bearer-internal-refresh".to_string(),
            mode: codex_pool_core::model::UpstreamMode::CodexOauth,
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            access_token: "legacy-bearer-refresh-noop".to_string(),
            chatgpt_account_id: Some("acct_legacy_internal_refresh".to_string()),
            enabled: Some(true),
            priority: Some(100),
            token_expires_at: Some(
                chrono::Utc::now() + chrono::Duration::hours(2),
            ),
            chatgpt_plan_type: Some("free".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .expect("create legacy bearer account")
        .account;
    let app = build_app_with_store(store);

    let refresh_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{}/oauth/refresh",
                    account.id
                ))
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_body = to_bytes(refresh_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let refresh_json: Value = serde_json::from_slice(&refresh_body).unwrap();
    assert_eq!(refresh_json["account_id"], account.id.to_string());
    assert_eq!(refresh_json["auth_provider"], "legacy_bearer");
    assert_eq!(refresh_json["credential_kind"], "one_time_access_token");
    assert_eq!(refresh_json["has_refresh_credential"], false);
    assert_eq!(refresh_json["last_refresh_status"], "never");
}

#[tokio::test]
async fn account_pool_reprobe_supports_runtime_legacy_bearer_account() {
    let store = InMemoryStore::new_with_oauth(
        Arc::new(InventoryProbeOAuthTokenClient {
            rate_limits: vec![OAuthRateLimitSnapshot {
                limit_id: Some("five_hours".to_string()),
                limit_name: Some("5 hours".to_string()),
                primary: Some(OAuthRateLimitWindow {
                    used_percent: 18.0,
                    window_minutes: Some(300),
                    resets_at: Some(chrono::Utc::now() + chrono::Duration::minutes(35)),
                }),
                secondary: None,
            }],
        }),
        None,
    );
    store
        .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
            label: "legacy-bearer-account-pool-reprobe".to_string(),
            mode: codex_pool_core::model::UpstreamMode::CodexOauth,
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            access_token: "legacy-bearer-account-pool-reprobe-token".to_string(),
            chatgpt_account_id: Some("acct_legacy_account_pool_reprobe".to_string()),
            enabled: Some(true),
            priority: Some(100),
            token_expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(2)),
            chatgpt_plan_type: Some("free".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .expect("create legacy bearer account");

    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let records_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/account-pool/accounts")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(records_response.status(), StatusCode::OK);
    let records_body = to_bytes(records_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let records: Vec<Value> = serde_json::from_slice(&records_body).unwrap();
    let record_id = records
        .iter()
        .find(|item| item["label"] == "legacy-bearer-account-pool-reprobe")
        .and_then(|item| item["id"].as_str())
        .expect("legacy bearer account pool record")
        .to_string();
    let initial_record = records
        .iter()
        .find(|item| item["id"] == record_id)
        .expect("initial legacy bearer record");
    assert!(initial_record["last_probe_at"].is_null());
    assert!(initial_record["rate_limits_fetched_at"].is_null());

    let reprobe_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/account-pool/actions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "reprobe",
                        "record_ids": [record_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reprobe_response.status(), StatusCode::OK);
    let reprobe_body = to_bytes(reprobe_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let reprobe_json: Value = serde_json::from_slice(&reprobe_body).unwrap();
    assert_eq!(reprobe_json["success_count"], 1);
    assert_eq!(reprobe_json["failed_count"], 0);

    let reprobed_record_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/account-pool/accounts/{record_id}"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reprobed_record_response.status(), StatusCode::OK);
    let reprobed_record_body = to_bytes(reprobed_record_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let reprobed_record: Value = serde_json::from_slice(&reprobed_record_body).unwrap();
    assert!(reprobed_record["last_probe_at"].is_string());
    assert_eq!(reprobed_record["health_freshness"], "fresh");
    assert_eq!(reprobed_record["last_probe_outcome"], "ok");
    assert!(reprobed_record["rate_limits_fetched_at"].is_string());
    assert_eq!(reprobed_record["rate_limits"][0]["limit_id"], "five_hours");
}

#[tokio::test]
async fn oauth_family_disable_and_enable_routes_work() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([9_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-refresh-token")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "oauth-family",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "refresh_token": "rt-family"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);
    let import_body = to_bytes(import_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let import_json: Value = serde_json::from_slice(&import_body).unwrap();
    let account_id = import_json["id"].as_str().unwrap().to_string();

    let disable_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/upstream-accounts/{account_id}/oauth/family/disable"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(disable_response.status(), StatusCode::OK);
    let disable_body = to_bytes(disable_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let disable_json: Value = serde_json::from_slice(&disable_body).unwrap();
    assert_eq!(disable_json["enabled"], false);
    assert_eq!(disable_json["affected_accounts"], 1);

    let disabled_status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/{account_id}/oauth/status"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(disabled_status_response.status(), StatusCode::OK);
    let disabled_status_body = to_bytes(disabled_status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let disabled_status_json: Value = serde_json::from_slice(&disabled_status_body).unwrap();
    assert_eq!(disabled_status_json["effective_enabled"], false);

    let enable_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/upstream-accounts/{account_id}/oauth/family/enable"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(enable_response.status(), StatusCode::OK);
    let enable_body = to_bytes(enable_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let enable_json: Value = serde_json::from_slice(&enable_body).unwrap();
    assert_eq!(enable_json["enabled"], true);
}

#[tokio::test]
async fn internal_seen_ok_route_requires_internal_token_and_is_idempotent() {
    let store = Arc::new(RateLimitApiTestStore::default());
    let app = build_app_with_store(store.clone());
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "seen-ok-account",
                        "mode": "chat_gpt_session",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "bearer_token": "test-token",
                        "enabled": true,
                        "priority": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let account_id = create_json["id"].as_str().unwrap().to_string();

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/health/seen-ok"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/health/seen-ok"
                ))
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = to_bytes(first.into_body(), usize::MAX).await.unwrap();
    let first_json: Value = serde_json::from_slice(&first_body).unwrap();
    assert_eq!(first_json["ok"], true);
    assert_eq!(first_json["accepted"], true);
    sleep(Duration::from_millis(50)).await;
    assert_eq!(store.seen_ok_refresh_call_count(), 1);

    let second = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/health/seen-ok"
                ))
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    let second_body = to_bytes(second.into_body(), usize::MAX).await.unwrap();
    let second_json: Value = serde_json::from_slice(&second_body).unwrap();
    assert_eq!(second_json["ok"], true);
    assert_eq!(second_json["accepted"], false);
    sleep(Duration::from_millis(50)).await;
    assert_eq!(store.seen_ok_refresh_call_count(), 1);
}

#[tokio::test]
async fn internal_live_result_route_requires_internal_token_and_accepts_payload() {
    let store = InMemoryStore::default();
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "live-result-account",
                        "mode": "chat_gpt_session",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "bearer_token": "test-token",
                        "enabled": true,
                        "priority": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let account_id = create_json["id"].as_str().unwrap().to_string();

    let payload = json!({
        "status": "failed",
        "source": "passive",
        "status_code": 401,
        "error_code": "account_deactivated",
        "error_message": "account is deactivated",
        "upstream_request_id": "req-live-route",
        "model": "gpt-5.4"
    });

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/health/live-result"
                ))
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let authorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/health/live-result"
                ))
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authorized.status(), StatusCode::OK);
    let authorized_body = to_bytes(authorized.into_body(), usize::MAX).await.unwrap();
    let authorized_json: Value = serde_json::from_slice(&authorized_body).unwrap();
    assert_eq!(authorized_json["ok"], true);
    assert_eq!(authorized_json["accepted"], true);
    assert_eq!(authorized_json["account_id"], account_id);
    assert_eq!(authorized_json["status"], "failed");
    assert_eq!(authorized_json["error_code"], "account_deactivated");

    let status_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/upstream-accounts/{account_id}/oauth/status"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_body = to_bytes(status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_json: Value = serde_json::from_slice(&status_body).unwrap();
    assert_eq!(status_json["pool_state"], "pending_purge");
    assert_eq!(status_json["pending_purge_reason"], "account_deactivated");
    assert_eq!(status_json["last_live_result_status"], "failed");
    assert_eq!(status_json["last_live_result_source"], "passive");
    assert_eq!(status_json["last_live_result_status_code"], 401);
    assert_eq!(status_json["last_live_error_code"], "account_deactivated");
}

#[tokio::test]
async fn internal_live_result_rate_limited_quarantines_runtime_account() {
    let store = InMemoryStore::default();
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "live-result-rate-limited",
                        "mode": "chat_gpt_session",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "bearer_token": "test-token-rate-limited",
                        "enabled": true,
                        "priority": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let account_id = create_json["id"].as_str().unwrap().to_string();

    let payload = json!({
        "status": "failed",
        "source": "active",
        "status_code": 429,
        "error_code": "rate_limited",
        "error_message": "too many requests",
        "upstream_request_id": "req-live-rate-limited",
        "model": "gpt-5.4"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/health/live-result"
                ))
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["accepted"], true);

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/upstream-accounts/{account_id}/oauth/status"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_body = to_bytes(status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_json: Value = serde_json::from_slice(&status_body).unwrap();
    assert_eq!(status_json["pool_state"], "quarantine");
    assert_eq!(status_json["quarantine_reason"], "rate_limited");
    assert!(status_json["quarantine_until"].is_string());
    assert_eq!(status_json["last_live_result_status"], "failed");
    assert_eq!(status_json["last_live_result_source"], "active");
    assert_eq!(status_json["last_live_result_status_code"], 429);
    assert_eq!(status_json["last_live_error_code"], "rate_limited");

    let snapshot_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/data-plane/snapshot")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(snapshot_response.status(), StatusCode::OK);
    let snapshot_body = to_bytes(snapshot_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let snapshot_json: Value = serde_json::from_slice(&snapshot_body).unwrap();
    let snapshot_account = snapshot_json["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == account_id)
        .cloned();
    assert!(
        snapshot_account.is_none(),
        "quarantined account should be removed from runtime snapshot"
    );
}

#[tokio::test]
async fn internal_live_result_token_invalidated_quarantines_runtime_account() {
    let store = InMemoryStore::default();
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "live-result-token-invalidated",
                        "mode": "chat_gpt_session",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "bearer_token": "test-token-token-invalidated",
                        "enabled": true,
                        "priority": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let account_id = create_json["id"].as_str().unwrap().to_string();

    let payload = json!({
        "status": "failed",
        "source": "passive",
        "status_code": 401,
        "error_code": "token_invalidated",
        "error_message": "token invalidated",
        "upstream_request_id": "req-live-token-invalidated",
        "model": "gpt-5.4"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/health/live-result"
                ))
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["accepted"], true);

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/upstream-accounts/{account_id}/oauth/status"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_body = to_bytes(status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_json: Value = serde_json::from_slice(&status_body).unwrap();
    assert_eq!(status_json["pool_state"], "quarantine");
    assert_eq!(status_json["quarantine_reason"], "token_invalidated");
    assert!(status_json["quarantine_until"].is_string());
    assert_eq!(status_json["last_live_result_status"], "failed");
    assert_eq!(status_json["last_live_result_source"], "passive");
    assert_eq!(status_json["last_live_result_status_code"], 401);
    assert_eq!(status_json["last_live_error_code"], "token_invalidated");

    let snapshot_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/data-plane/snapshot")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(snapshot_response.status(), StatusCode::OK);
    let snapshot_body = to_bytes(snapshot_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let snapshot_json: Value = serde_json::from_slice(&snapshot_body).unwrap();
    let snapshot_account = snapshot_json["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == account_id)
        .cloned();
    assert!(
        snapshot_account.is_none(),
        "token_invalidated account should be removed from runtime snapshot"
    );
}

#[tokio::test]
async fn internal_live_result_token_invalidated_escalates_legacy_account_to_pending_purge() {
    let store = InMemoryStore::default();
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "live-result-token-invalidated-escalate",
                        "mode": "chat_gpt_session",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "bearer_token": "test-token-token-invalidated-escalate",
                        "enabled": true,
                        "priority": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let account_id = create_json["id"].as_str().unwrap().to_string();

    let payload = json!({
        "status": "failed",
        "source": "passive",
        "status_code": 401,
        "error_code": "token_invalidated",
        "error_message": "token invalidated",
        "upstream_request_id": "req-live-token-invalidated-escalate",
        "model": "gpt-5.4"
    });

    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/internal/v1/upstream-accounts/{account_id}/health/live-result"
                    ))
                    .header("content-type", "application/json")
                    .header(
                        "authorization",
                        format!("Bearer {}", internal_service_token()),
                    )
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/upstream-accounts/{account_id}/oauth/status"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_body = to_bytes(status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_json: Value = serde_json::from_slice(&status_body).unwrap();
    assert_eq!(status_json["pool_state"], "pending_purge");
    assert_eq!(status_json["pending_purge_reason"], "token_invalidated");
    assert!(status_json["pending_purge_at"].is_string());
}

#[tokio::test]
async fn internal_model_seen_ok_route_requires_internal_token_and_accepts_model() {
    let store = InMemoryStore::default();
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "model-seen-ok-account",
                        "mode": "chat_gpt_session",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "bearer_token": "test-token",
                        "enabled": true,
                        "priority": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let account_id = create_json["id"].as_str().unwrap().to_string();

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/models/seen-ok"
                ))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "model": "gpt-5.3-codex" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let authorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/internal/v1/upstream-accounts/{account_id}/models/seen-ok"
                ))
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .header("content-type", "application/json")
                .body(Body::from(json!({ "model": "gpt-5.3-codex" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authorized.status(), StatusCode::OK);
    let authorized_body = to_bytes(authorized.into_body(), usize::MAX).await.unwrap();
    let authorized_json: Value = serde_json::from_slice(&authorized_body).unwrap();
    assert_eq!(authorized_json["ok"], true);
    assert_eq!(authorized_json["accepted"], true);
    assert_eq!(authorized_json["account_id"], account_id);
    assert_eq!(authorized_json["model"], "gpt-5.3-codex");
}

#[tokio::test]
async fn upstream_account_batch_actions_route_handles_partial_failures_and_refresh_jobs() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "batch-enable-delete",
                        "mode": "open_ai_api_key",
                        "base_url": "https://api.openai.com/v1",
                        "bearer_token": "tok-batch-1",
                        "enabled": false,
                        "priority": 10
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let account_id = create_json["id"].as_str().unwrap().to_string();
    let missing_id = Uuid::new_v4().to_string();

    let enable_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/batch-actions")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "action": "enable",
                        "account_ids": [account_id, missing_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(enable_response.status(), StatusCode::OK);
    let enable_body = to_bytes(enable_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let enable_json: Value = serde_json::from_slice(&enable_body).unwrap();
    assert_eq!(enable_json["action"], "enable");
    assert_eq!(enable_json["total"], 2);
    assert_eq!(enable_json["success_count"], 1);
    assert_eq!(enable_json["failed_count"], 1);
    assert_eq!(enable_json["items"][0]["ok"], true);
    assert_eq!(enable_json["items"][1]["ok"], false);
    assert_eq!(enable_json["items"][1]["error"]["code"], "not_found");

    let refresh_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/batch-actions")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "action": "refresh_login",
                        "account_ids": [account_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_body = to_bytes(refresh_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let refresh_json: Value = serde_json::from_slice(&refresh_body).unwrap();
    assert_eq!(refresh_json["success_count"], 1);
    assert_eq!(refresh_json["failed_count"], 0);
    assert!(refresh_json["items"][0]["job_id"].is_string());

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/batch-actions")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "action": "delete",
                        "account_ids": [account_id, missing_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::OK);
    let delete_body = to_bytes(delete_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let delete_json: Value = serde_json::from_slice(&delete_body).unwrap();
    assert_eq!(delete_json["success_count"], 1);
    assert_eq!(delete_json["failed_count"], 1);

    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_json: Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn upstream_account_batch_actions_route_supports_oauth_family_actions() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([10_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-refresh-token")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "oauth-family-batch",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "refresh_token": "rt-family-batch"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);
    let import_body = to_bytes(import_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let import_json: Value = serde_json::from_slice(&import_body).unwrap();
    let account_id = import_json["id"].as_str().unwrap().to_string();

    let pause_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/batch-actions")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "action": "pause_family",
                        "account_ids": [account_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pause_response.status(), StatusCode::OK);
    let pause_body = to_bytes(pause_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let pause_json: Value = serde_json::from_slice(&pause_body).unwrap();
    assert_eq!(pause_json["success_count"], 1);
    assert_eq!(pause_json["failed_count"], 0);
    assert_eq!(pause_json["items"][0]["ok"], true);

    let paused_status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/{account_id}/oauth/status"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(paused_status_response.status(), StatusCode::OK);
    let paused_status_body = to_bytes(paused_status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let paused_status_json: Value = serde_json::from_slice(&paused_status_body).unwrap();
    assert_eq!(paused_status_json["effective_enabled"], false);

    let resume_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/batch-actions")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "action": "resume_family",
                        "account_ids": [account_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resume_response.status(), StatusCode::OK);
    let resume_body = to_bytes(resume_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let resume_json: Value = serde_json::from_slice(&resume_body).unwrap();
    assert_eq!(resume_json["success_count"], 1);
    assert_eq!(resume_json["failed_count"], 0);
    assert_eq!(resume_json["items"][0]["ok"], true);
}

#[tokio::test]
async fn oauth_import_returns_refresh_token_reused_error() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([8_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(ReusedOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-refresh-token")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "oauth-b",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "refresh_token": "rt-reused"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "refresh_token_reused");
}

#[tokio::test]
async fn account_pool_runtime_responses_test_route_forwards_to_data_plane_with_internal_token() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([61_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(InventoryProbeOAuthTokenClient {
            rate_limits: vec![OAuthRateLimitSnapshot {
                limit_id: Some("five_hours".to_string()),
                limit_name: Some("5 hours".to_string()),
                primary: Some(OAuthRateLimitWindow {
                    used_percent: 12.0,
                    window_minutes: Some(300),
                    resets_at: Some(chrono::Utc::now() + chrono::Duration::minutes(40)),
                }),
                secondary: None,
            }],
        }),
        Some(cipher),
    );
    store
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "pool-runtime-responses-test".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-pool-runtime-responses-test".to_string(),
            fallback_access_token: Some("ak-pool-runtime-responses-test".to_string()),
            fallback_token_expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(2)),
            chatgpt_account_id: Some("acct_pool_runtime_responses_test".to_string()),
            mode: Some(codex_pool_core::model::UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: Some("pro".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(store.activate_oauth_refresh_token_vault().await.unwrap(), 1);
    let account_id = store
        .list_upstream_accounts()
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("runtime account")
        .id;

    let forwarded = Arc::new(Mutex::new(Vec::<Value>::new()));
    let data_plane_app = axum::Router::new().route(
        "/internal/v1/upstream-accounts/{account_id}/responses/test",
        axum::routing::post({
            let forwarded = forwarded.clone();
            move |axum::extract::Path(account_id): axum::extract::Path<String>,
                  headers: axum::http::HeaderMap,
                  body: String| {
                let forwarded = forwarded.clone();
                async move {
                    let payload: Value = serde_json::from_str(&body).unwrap();
                    forwarded.lock().unwrap().push(json!({
                        "account_id": account_id,
                        "authorization": headers
                            .get("authorization")
                            .and_then(|value| value.to_str().ok()),
                        "request_id": headers
                            .get("x-request-id")
                            .and_then(|value| value.to_str().ok()),
                        "payload": payload,
                    }));

                    axum::Json(json!({
                        "response_id": "resp_test_runtime",
                        "assistant_text": "pong",
                        "request_id": "req_test_runtime",
                        "model": "gpt-5.4",
                        "status_code": 200,
                        "latency_ms": 42,
                        "usage": {
                            "input_tokens": 11,
                            "cached_input_tokens": 0,
                            "output_tokens": 7,
                            "reasoning_tokens": 0
                        },
                        "previous_response_id": "resp_prev_runtime"
                    }))
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, data_plane_app).await.unwrap();
    });

    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let config_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/admin/config")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "data_plane_base_url": format!("http://{addr}")
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(config_response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/account-pool/accounts/{account_id}/responses/test"
                ))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "input": "ping",
                        "previous_response_id": "resp_prev_runtime"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["response_id"], "resp_test_runtime");
    assert_eq!(payload["assistant_text"], "pong");
    assert_eq!(payload["request_id"], "req_test_runtime");
    assert_eq!(payload["previous_response_id"], "resp_prev_runtime");

    let forwarded = forwarded.lock().unwrap();
    assert_eq!(forwarded.len(), 1);
    assert_eq!(forwarded[0]["account_id"], account_id.to_string());
    assert_eq!(
        forwarded[0]["authorization"],
        format!("Bearer {}", internal_service_token())
    );
    assert_eq!(forwarded[0]["payload"]["model"], "gpt-5.4");
    assert_eq!(forwarded[0]["payload"]["input"], "ping");
    assert_eq!(
        forwarded[0]["payload"]["previous_response_id"],
        "resp_prev_runtime"
    );
    assert!(forwarded[0]["request_id"].is_string());
}

#[tokio::test]
async fn account_pool_inventory_responses_test_route_rejects_without_forwarding() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([62_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(InventoryProbeOAuthTokenClient { rate_limits: Vec::new() }),
        Some(cipher),
    );
    store
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "pool-inventory-responses-test".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-pool-inventory-responses-test".to_string(),
            fallback_access_token: Some("ak-pool-inventory-responses-test".to_string()),
            fallback_token_expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(2)),
            chatgpt_account_id: Some("acct_pool_inventory_responses_test".to_string()),
            mode: Some(codex_pool_core::model::UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: Some("pro".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    let forwarded = Arc::new(Mutex::new(Vec::<Value>::new()));
    let data_plane_app = axum::Router::new().route(
        "/internal/v1/upstream-accounts/{account_id}/responses/test",
        axum::routing::post({
            let forwarded = forwarded.clone();
            move |axum::extract::Path(account_id): axum::extract::Path<String>, body: String| {
                let forwarded = forwarded.clone();
                async move {
                    forwarded.lock().unwrap().push(json!({
                        "account_id": account_id,
                        "body": body,
                    }));
                    axum::Json(json!({
                        "response_id": "unexpected"
                    }))
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, data_plane_app).await.unwrap();
    });

    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let config_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/admin/config")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "data_plane_base_url": format!("http://{addr}")
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(config_response.status(), StatusCode::OK);

    let records_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/account-pool/accounts")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(records_response.status(), StatusCode::OK);
    let records_body = to_bytes(records_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let records: Vec<Value> = serde_json::from_slice(&records_body).unwrap();
    let record_id = records
        .iter()
        .find(|item| item["label"] == "pool-inventory-responses-test")
        .and_then(|item| item["id"].as_str())
        .expect("inventory record id")
        .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/account-pool/accounts/{record_id}/responses/test"
                ))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "input": "ping"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "invalid_request");
    assert!(forwarded.lock().unwrap().is_empty());
}
