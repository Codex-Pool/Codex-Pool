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
                    "/internal/v1/upstream-accounts/{account_id}/disable"
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
        .cloned()
        .expect("snapshot account should exist");
    assert_eq!(snapshot_account["enabled"], false);
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
