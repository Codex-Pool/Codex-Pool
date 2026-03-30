static OAUTH_LOGIN_ENV_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

fn set_env(key: &str, value: Option<&str>) -> Option<String> {
    let previous = std::env::var(key).ok();
    match value {
        Some(raw) => std::env::set_var(key, raw),
        None => std::env::remove_var(key),
    }
    previous
}

fn mock_id_token(email: &str, account_id: &str, plan_type: &str) -> String {
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        json!({
            "email": email,
            "chatgpt_account_id": account_id,
            "chatgpt_plan_type": plan_type
        })
        .to_string(),
    );
    format!("{header}.{payload}.signature")
}

async fn spawn_codex_oauth_token_server(
    fail_auth_code_exchange: bool,
) -> (String, tokio::task::JoinHandle<()>) {
    let app = axum::Router::new()
        .route(
            "/oauth/token",
            axum::routing::post(
                |axum::extract::State(fail_auth_code_exchange): axum::extract::State<bool>,
                 axum::extract::Form(form): axum::extract::Form<
                    std::collections::HashMap<String, String>,
                >| async move {
                    let grant_type = form
                        .get("grant_type")
                        .map(|value| value.as_str())
                        .unwrap_or_default();
                    match grant_type {
                        "authorization_code" => {
                            if fail_auth_code_exchange {
                                return (
                                    StatusCode::SERVICE_UNAVAILABLE,
                                    axum::Json(json!({
                                        "error": "server_error",
                                        "error_description": "temporary unavailable",
                                    })),
                                );
                            }
                            (
                                StatusCode::OK,
                                axum::Json(json!({
                                    "access_token": "at-from-code",
                                    "refresh_token": "rt-from-code",
                                    "expires_in": 3600,
                                    "token_type": "Bearer",
                                    "scope": "offline_access openid profile email",
                                    "chatgpt_account_id": "acct-from-code",
                                    "id_token": mock_id_token(
                                        "oauth-user@example.com",
                                        "acct-from-id-token",
                                        "pro",
                                    ),
                                })),
                            )
                        }
                        "refresh_token" => (
                            StatusCode::OK,
                            axum::Json(json!({
                                "access_token": "at-from-refresh",
                                "refresh_token": form.get("refresh_token").cloned().unwrap_or_else(|| "rt-fallback".to_string()),
                                "expires_in": 3600,
                                "token_type": "Bearer",
                                "scope": "offline_access",
                                "chatgpt_account_id": "acct-from-refresh",
                            })),
                        ),
                        _ => (
                            StatusCode::BAD_REQUEST,
                            axum::Json(json!({
                                "error": "invalid_request",
                                "error_description": "unsupported grant type",
                            })),
                        ),
                    }
                },
            ),
        )
        .with_state(fail_auth_code_exchange);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind oauth mock");
    let addr = listener.local_addr().expect("read oauth mock addr");
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}/oauth/token"), handle)
}

#[derive(Clone)]
struct FixedAccountIdOAuthTokenClient {
    account_id: &'static str,
    account_user_id: &'static str,
}

#[derive(Clone)]
struct SharedAccountIdOAuthTokenClient;

#[derive(Clone)]
struct InvalidFallbackAdmissionOAuthTokenClient;

#[async_trait::async_trait]
impl control_plane::oauth::OAuthTokenClient for SharedAccountIdOAuthTokenClient {
    async fn refresh_token(
        &self,
        refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<
        control_plane::oauth::OAuthTokenInfo,
        control_plane::oauth::OAuthTokenClientError,
    > {
        let (email, account_user_id) = if refresh_token.contains("workspace-a") {
            (
                "oauth-workspace-a@example.com",
                "acct_user_shared_workspace_a",
            )
        } else {
            (
                "oauth-workspace-b@example.com",
                "acct_user_shared_workspace_b",
            )
        };
        Ok(control_plane::oauth::OAuthTokenInfo {
            access_token: format!("access-{refresh_token}"),
            refresh_token: refresh_token.to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
            token_type: Some("Bearer".to_string()),
            scope: Some("offline_access".to_string()),
            email: Some(email.to_string()),
            oauth_subject: Some("auth0|oauth-user".to_string()),
            oauth_identity_provider: Some("google-oauth2".to_string()),
            email_verified: Some(true),
            chatgpt_account_id: Some("acct-from-code".to_string()),
            chatgpt_user_id: Some("user-shared".to_string()),
            chatgpt_plan_type: Some("team".to_string()),
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: Some(account_user_id.to_string()),
            chatgpt_compute_residency: Some("us".to_string()),
            workspace_name: Some(
                if refresh_token.contains("workspace-a") {
                    "OAI-03.09"
                } else {
                    "OAI-07.11"
                }
                .to_string(),
            ),
            organizations: Some(vec![json!({
                "id": "org_shared",
                "title": "Personal",
            })]),
            groups: Some(vec![]),
        })
    }
}

#[async_trait::async_trait]
impl control_plane::oauth::OAuthTokenClient for InvalidFallbackAdmissionOAuthTokenClient {
    async fn refresh_token(
        &self,
        refresh_token: &str,
        base_url: Option<&str>,
    ) -> Result<
        control_plane::oauth::OAuthTokenInfo,
        control_plane::oauth::OAuthTokenClientError,
    > {
        StaticOAuthTokenClient
            .refresh_token(refresh_token, base_url)
            .await
    }

    async fn fetch_rate_limits(
        &self,
        _access_token: &str,
        _base_url: Option<&str>,
        _chatgpt_account_id: Option<&str>,
    ) -> Result<
        Vec<control_plane::contracts::OAuthRateLimitSnapshot>,
        control_plane::oauth::OAuthTokenClientError,
    > {
        Err(control_plane::oauth::OAuthTokenClientError::InvalidRefreshToken {
            code: control_plane::oauth::OAuthRefreshErrorCode::InvalidRefreshToken,
            message: "invalid refresh token".to_string(),
        })
    }
}

#[async_trait::async_trait]
impl control_plane::oauth::OAuthTokenClient for FixedAccountIdOAuthTokenClient {
    async fn refresh_token(
        &self,
        refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<
        control_plane::oauth::OAuthTokenInfo,
        control_plane::oauth::OAuthTokenClientError,
    > {
        Ok(control_plane::oauth::OAuthTokenInfo {
            access_token: format!("access-{refresh_token}"),
            refresh_token: refresh_token.to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
            token_type: Some("Bearer".to_string()),
            scope: Some("offline_access".to_string()),
            email: Some("oauth-user@example.com".to_string()),
            oauth_subject: Some("auth0|oauth-user".to_string()),
            oauth_identity_provider: Some("google-oauth2".to_string()),
            email_verified: Some(true),
            chatgpt_account_id: Some(self.account_id.to_string()),
            chatgpt_user_id: Some("user-shared".to_string()),
            chatgpt_plan_type: Some("team".to_string()),
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: Some(self.account_user_id.to_string()),
            chatgpt_compute_residency: Some("us".to_string()),
            workspace_name: Some("OAI-03.09".to_string()),
            organizations: Some(vec![json!({
                "id": "org_shared",
                "title": "Personal",
            })]),
            groups: Some(vec![]),
        })
    }
}

#[tokio::test]
async fn codex_oauth_login_session_callback_imports_account() {
    let _guard = OAUTH_LOGIN_ENV_LOCK.lock().await;
    let (token_url, mock_handle) = spawn_codex_oauth_token_server(false).await;
    let old_token_url = set_env("OPENAI_OAUTH_TOKEN_URL", Some(&token_url));
    let old_client_id = set_env("OPENAI_OAUTH_CLIENT_ID", Some("client_test_codex"));
    let old_authorize_url = set_env(
        "OPENAI_OAUTH_AUTHORIZE_URL",
        Some("https://auth.example.com/oauth/authorize"),
    );
    let old_public_base_url = set_env("CONTROL_PLANE_PUBLIC_BASE_URL", Some("https://cp.example.com"));

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([11_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/codex/login-sessions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "label": "codex-oauth-demo",
                        "base_url": "https://chatgpt.com/backend-api/codex",
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
    let session_id = create_json["session_id"].as_str().unwrap().to_string();
    let authorize_url = create_json["authorize_url"].as_str().unwrap();
    let state = reqwest::Url::parse(authorize_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .unwrap();

    let callback_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/codex/callback?code=code-from-oauth&state={state}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(callback_response.status(), StatusCode::OK);

    let mut latest = Value::Null;
    for _ in 0..20 {
        let status_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/upstream-accounts/oauth/codex/login-sessions/{session_id}"
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
        latest = serde_json::from_slice(&status_body).unwrap();
        if matches!(
            latest["status"].as_str().unwrap_or_default(),
            "completed" | "failed" | "expired"
        ) {
            break;
        }
        sleep(Duration::from_millis(30)).await;
    }

    assert_eq!(latest["status"], "completed", "latest session={latest}");
    assert_eq!(latest["result"]["created"], true);
    assert_eq!(latest["result"]["email"], "oauth-user@example.com");
    assert_eq!(latest["result"]["chatgpt_plan_type"], "pro");
    assert_eq!(latest["result"]["account"]["mode"], "codex_oauth");
    assert_eq!(
        latest["result"]["account"]["base_url"],
        "https://chatgpt.com/backend-api/codex"
    );

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
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_json: Value = serde_json::from_slice(&list_body).unwrap();
    let imported_id = latest["result"]["account"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(list_json
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item["id"] == imported_id)));

    mock_handle.abort();
    set_env("OPENAI_OAUTH_TOKEN_URL", old_token_url.as_deref());
    set_env("OPENAI_OAUTH_CLIENT_ID", old_client_id.as_deref());
    set_env("OPENAI_OAUTH_AUTHORIZE_URL", old_authorize_url.as_deref());
    set_env("CONTROL_PLANE_PUBLIC_BASE_URL", old_public_base_url.as_deref());
}

#[tokio::test]
async fn codex_oauth_login_session_updates_existing_chatgpt_account_user_id() {
    let _guard = OAUTH_LOGIN_ENV_LOCK.lock().await;
    let (token_url, mock_handle) = spawn_codex_oauth_token_server(false).await;
    let old_token_url = set_env("OPENAI_OAUTH_TOKEN_URL", Some(&token_url));
    let old_client_id = set_env("OPENAI_OAUTH_CLIENT_ID", Some("client_test_codex"));
    let old_authorize_url = set_env(
        "OPENAI_OAUTH_AUTHORIZE_URL",
        Some("https://auth.example.com/oauth/authorize"),
    );
    let old_public_base_url = set_env("CONTROL_PLANE_PUBLIC_BASE_URL", Some("https://cp.example.com"));

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([21_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(FixedAccountIdOAuthTokenClient {
            account_id: "acct-from-code",
            account_user_id: "acct_user_shared",
        }),
        Some(cipher),
    );
    let seeded = store
        .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "seeded-oauth".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-seeded".to_string(),
            fallback_access_token: None,
            fallback_token_expires_at: None,
            chatgpt_account_id: Some("acct-from-code".to_string()),
            mode: Some(codex_pool_core::model::UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: Some("team".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/codex/login-sessions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "label": "codex-oauth-duplicate",
                        "base_url": "https://chatgpt.com/backend-api/codex",
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
    let session_id = create_json["session_id"].as_str().unwrap().to_string();
    let authorize_url = create_json["authorize_url"].as_str().unwrap();
    let state = reqwest::Url::parse(authorize_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .unwrap();

    let callback_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/codex/callback?code=code-from-oauth&state={state}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(callback_response.status(), StatusCode::OK);

    let mut latest = Value::Null;
    for _ in 0..20 {
        let status_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/upstream-accounts/oauth/codex/login-sessions/{session_id}"
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
        latest = serde_json::from_slice(&status_body).unwrap();
        if matches!(
            latest["status"].as_str().unwrap_or_default(),
            "completed" | "failed" | "expired"
        ) {
            break;
        }
        sleep(Duration::from_millis(30)).await;
    }

    assert_eq!(latest["status"], "completed", "latest session={latest}");
    assert_eq!(latest["result"]["created"], false);
    assert_eq!(latest["result"]["account"]["id"], seeded.id.to_string());

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
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_json: Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_json.as_array().map(Vec::len), Some(1));

    mock_handle.abort();
    set_env("OPENAI_OAUTH_TOKEN_URL", old_token_url.as_deref());
    set_env("OPENAI_OAUTH_CLIENT_ID", old_client_id.as_deref());
    set_env("OPENAI_OAUTH_AUTHORIZE_URL", old_authorize_url.as_deref());
    set_env("CONTROL_PLANE_PUBLIC_BASE_URL", old_public_base_url.as_deref());
}

#[tokio::test]
async fn codex_oauth_login_session_keeps_distinct_accounts_with_shared_chatgpt_account_id() {
    let _guard = OAUTH_LOGIN_ENV_LOCK.lock().await;
    let (token_url, mock_handle) = spawn_codex_oauth_token_server(false).await;
    let old_token_url = set_env("OPENAI_OAUTH_TOKEN_URL", Some(&token_url));
    let old_client_id = set_env("OPENAI_OAUTH_CLIENT_ID", Some("client_test_codex"));
    let old_authorize_url = set_env(
        "OPENAI_OAUTH_AUTHORIZE_URL",
        Some("https://auth.example.com/oauth/authorize"),
    );
    let old_public_base_url = set_env("CONTROL_PLANE_PUBLIC_BASE_URL", Some("https://cp.example.com"));

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([23_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(SharedAccountIdOAuthTokenClient),
        Some(cipher),
    );
    let seeded = store
        .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: "seeded-oauth".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: "rt-seeded-workspace-a".to_string(),
            fallback_access_token: None,
            fallback_token_expires_at: None,
            chatgpt_account_id: Some("acct-from-code".to_string()),
            mode: Some(codex_pool_core::model::UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: Some("team".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/codex/login-sessions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "label": "codex-oauth-workspace-b",
                        "base_url": "https://chatgpt.com/backend-api/codex",
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
    let session_id = create_json["session_id"].as_str().unwrap().to_string();
    let authorize_url = create_json["authorize_url"].as_str().unwrap();
    let state = reqwest::Url::parse(authorize_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .unwrap();

    let callback_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/auth/callback?code=test-code&state={state}&session_id={session_id}&scope=offline_access&refresh_token=rt-from-code-workspace-b"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(callback_response.status(), StatusCode::OK);

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/codex/login-sessions/{session_id}"
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

    assert_eq!(status_json["status"], "completed");
    assert_eq!(status_json["result"]["created"], true);
    assert_ne!(
        status_json["result"]["account"]["id"].as_str().unwrap(),
        seeded.id.to_string()
    );

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
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_json: Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_json.as_array().map(Vec::len), Some(2));

    mock_handle.abort();
    set_env("OPENAI_OAUTH_TOKEN_URL", old_token_url.as_deref());
    set_env("OPENAI_OAUTH_CLIENT_ID", old_client_id.as_deref());
    set_env("OPENAI_OAUTH_AUTHORIZE_URL", old_authorize_url.as_deref());
    set_env("CONTROL_PLANE_PUBLIC_BASE_URL", old_public_base_url.as_deref());
}

#[tokio::test]
async fn codex_oauth_login_session_supports_manual_callback_submit() {
    let _guard = OAUTH_LOGIN_ENV_LOCK.lock().await;
    let (token_url, mock_handle) = spawn_codex_oauth_token_server(false).await;
    let old_token_url = set_env("OPENAI_OAUTH_TOKEN_URL", Some(&token_url));
    let old_client_id = set_env("OPENAI_OAUTH_CLIENT_ID", Some("client_test_codex"));
    let old_authorize_url = set_env(
        "OPENAI_OAUTH_AUTHORIZE_URL",
        Some("https://auth.example.com/oauth/authorize"),
    );
    let old_public_base_url = set_env("CONTROL_PLANE_PUBLIC_BASE_URL", Some("https://cp.example.com"));

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([12_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/codex/login-sessions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"label":"manual-fallback"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let session_id = create_json["session_id"].as_str().unwrap().to_string();
    let callback_url = create_json["callback_url"].as_str().unwrap().to_string();
    let authorize_url = create_json["authorize_url"].as_str().unwrap();
    let state = reqwest::Url::parse(authorize_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .unwrap();
    let redirect_url = format!("{callback_url}?code=manual-code&state={state}");

    let submit_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/codex/login-sessions/{session_id}/callback"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "redirect_url": redirect_url
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(submit_response.status(), StatusCode::OK);
    let submit_body = to_bytes(submit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let submit_json: Value = serde_json::from_slice(&submit_body).unwrap();
    assert_eq!(
        submit_json["status"],
        "completed",
        "manual callback result={submit_json}"
    );
    assert_eq!(submit_json["result"]["account"]["mode"], "codex_oauth");

    mock_handle.abort();
    set_env("OPENAI_OAUTH_TOKEN_URL", old_token_url.as_deref());
    set_env("OPENAI_OAUTH_CLIENT_ID", old_client_id.as_deref());
    set_env("OPENAI_OAUTH_AUTHORIZE_URL", old_authorize_url.as_deref());
    set_env("CONTROL_PLANE_PUBLIC_BASE_URL", old_public_base_url.as_deref());
}

#[tokio::test]
async fn codex_oauth_login_session_marks_failed_when_exchange_endpoint_errors() {
    let _guard = OAUTH_LOGIN_ENV_LOCK.lock().await;
    let (token_url, mock_handle) = spawn_codex_oauth_token_server(true).await;
    let old_token_url = set_env("OPENAI_OAUTH_TOKEN_URL", Some(&token_url));
    let old_client_id = set_env("OPENAI_OAUTH_CLIENT_ID", Some("client_test_codex"));
    let old_authorize_url = set_env(
        "OPENAI_OAUTH_AUTHORIZE_URL",
        Some("https://auth.example.com/oauth/authorize"),
    );
    let old_public_base_url = set_env("CONTROL_PLANE_PUBLIC_BASE_URL", Some("https://cp.example.com"));

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([13_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/codex/login-sessions")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"label":"exchange-failure"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let session_id = create_json["session_id"].as_str().unwrap().to_string();
    let authorize_url = create_json["authorize_url"].as_str().unwrap();
    let state = reqwest::Url::parse(authorize_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .unwrap();

    let callback_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/codex/callback?code=code-failure&state={state}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(callback_response.status(), StatusCode::OK);

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/codex/login-sessions/{session_id}"
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
    assert_eq!(status_json["status"], "failed");
    assert_eq!(status_json["error"]["code"], "upstream_unavailable");

    mock_handle.abort();
    set_env("OPENAI_OAUTH_TOKEN_URL", old_token_url.as_deref());
    set_env("OPENAI_OAUTH_CLIENT_ID", old_client_id.as_deref());
    set_env("OPENAI_OAUTH_AUTHORIZE_URL", old_authorize_url.as_deref());
    set_env("CONTROL_PLANE_PUBLIC_BASE_URL", old_public_base_url.as_deref());
}

#[tokio::test]
async fn oauth_import_job_does_not_fail_reused_refresh_tokens_during_queue_only_import() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([7_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(ReusedOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let boundary = "----cp-boundary-refresh-reused";
    let content = r#"{"refresh_token":"rt-reused-job","email":"reused@example.com"}"#;
    let payload = build_multipart_payload(boundary, "reused.jsonl", content);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-jobs")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(payload))
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

    let mut latest_job = Value::Null;
    for _ in 0..20 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}"
                    ))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        latest_job = serde_json::from_slice(&body).unwrap();
        let status = latest_job["status"].as_str().unwrap_or_default();
        if !matches!(status, "queued" | "running") {
            break;
        }
        sleep(Duration::from_millis(30)).await;
    }

    assert_eq!(latest_job["status"], "completed");
    assert_eq!(latest_job["created_count"], 1);
    assert_eq!(latest_job["updated_count"], 0);
    assert_eq!(latest_job["failed_count"], 0);

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
    assert_eq!(inventory_response.status(), StatusCode::OK);
    let inventory_body = to_bytes(inventory_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let inventory_json: Value = serde_json::from_slice(&inventory_body).unwrap();
    assert_eq!(inventory_json.as_array().map(Vec::len), Some(1));
    assert_eq!(inventory_json[0]["vault_status"], "needs_refresh");
    assert_eq!(
        inventory_json[0]["admission_error_code"],
        "missing_access_token_fallback"
    );
}

#[tokio::test]
async fn oauth_import_job_accepts_large_multipart_payload() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([9_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let boundary = "----cp-boundary-large";
    let padding = "x".repeat(2_200_000);
    let content = format!(
        r#"{{"refresh_token":"rt-large","email":"large@example.com","padding":"{padding}"}}"#
    );
    let payload = build_multipart_payload(boundary, "large.jsonl", &content);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-jobs")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    assert_eq!(create_json["total"], 1);
}

#[tokio::test]
async fn oauth_import_job_rejects_terminal_invalid_refresh_token_inventory_admission() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([31_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(InvalidFallbackAdmissionOAuthTokenClient),
        Some(cipher),
    );
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let boundary = "----cp-boundary-strict-terminal";
    let content = r#"{
      "type":"codex",
      "email":"strict-terminal@example.com",
      "refresh_token":"rt-strict-terminal",
      "access_token":"ak-strict-terminal",
      "chatgpt_account_id":"acct_strict_terminal",
      "exp": 1893456000
    }"#;
    let payload = build_multipart_payload(boundary, "strict-terminal.json", content);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-jobs")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(payload))
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

    let mut latest_job = Value::Null;
    for _ in 0..30 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}"
                    ))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        latest_job = serde_json::from_slice(&body).unwrap();
        let status = latest_job["status"].as_str().unwrap_or_default();
        if !matches!(status, "queued" | "running") {
            break;
        }
        sleep(Duration::from_millis(30)).await;
    }

    assert_eq!(latest_job["status"], "failed");
    assert_eq!(latest_job["created_count"], 0);
    assert_eq!(latest_job["updated_count"], 0);
    assert_eq!(latest_job["failed_count"], 1);
    assert_eq!(latest_job["admission_counts"]["failed"], 1);

    let items_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/items"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(items_response.status(), StatusCode::OK);
    let items_body = to_bytes(items_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let items_json: Value = serde_json::from_slice(&items_body).unwrap();
    assert_eq!(items_json["items"][0]["status"], "failed");
    assert_eq!(items_json["items"][0]["error_code"], "invalid_refresh_token");
    assert_eq!(items_json["items"][0]["admission_status"], "failed");

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
    assert_eq!(inventory_response.status(), StatusCode::OK);
    let inventory_body = to_bytes(inventory_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let inventory_json: Value = serde_json::from_slice(&inventory_body).unwrap();
    assert_eq!(inventory_json.as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn oauth_import_job_defers_identity_merge_until_activation() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([22_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(FixedAccountIdOAuthTokenClient {
            account_id: "acct-batch-shared",
            account_user_id: "acct_user_shared",
        }),
        Some(cipher),
    );
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let boundary = "----cp-boundary-duplicate-existing";
    let content = concat!(
        "{\"refresh_token\":\"rt-batch-a\",\"email\":\"shared@example.com\"}\n",
        "{\"refresh_token\":\"rt-batch-b\",\"email\":\"shared@example.com\"}"
    );
    let payload = build_multipart_payload(boundary, "shared.jsonl", content);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-jobs")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(payload))
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

    let mut latest_job = Value::Null;
    for _ in 0..20 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}"
                    ))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        latest_job = serde_json::from_slice(&body).unwrap();
        let status = latest_job["status"].as_str().unwrap_or_default();
        if !matches!(status, "queued" | "running") {
            break;
        }
        sleep(Duration::from_millis(30)).await;
    }

    assert_eq!(latest_job["status"], "completed");
    assert_eq!(latest_job["created_count"], 2);
    assert_eq!(latest_job["updated_count"], 0);

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
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_json: Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_json.as_array().map(Vec::len), Some(0));

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
    assert_eq!(inventory_response.status(), StatusCode::OK);
    let inventory_body = to_bytes(inventory_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let inventory_json: Value = serde_json::from_slice(&inventory_body).unwrap();
    assert_eq!(inventory_json.as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn oauth_import_job_keeps_distinct_vault_records_with_shared_chatgpt_account_id() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([24_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(
        Arc::new(SharedAccountIdOAuthTokenClient),
        Some(cipher),
    );
    let app = build_app_with_store(Arc::new(store));
    let admin_token = login_admin_token(&app).await;

    let boundary = "----cp-boundary-shared-workspaces";
    let content = concat!(
        "{\"refresh_token\":\"rt-batch-workspace-a\",\"email\":\"shared@example.com\"}\n",
        "{\"refresh_token\":\"rt-batch-workspace-b\",\"email\":\"shared@example.com\"}"
    );
    let payload = build_multipart_payload(boundary, "shared.jsonl", content);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-jobs")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(payload))
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

    let mut latest_job = Value::Null;
    for _ in 0..20 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}"
                    ))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        latest_job = serde_json::from_slice(&body).unwrap();
        let status = latest_job["status"].as_str().unwrap_or_default();
        if !matches!(status, "queued" | "running") {
            break;
        }
        sleep(Duration::from_millis(30)).await;
    }

    assert_eq!(latest_job["status"], "completed");
    assert_eq!(latest_job["created_count"], 2);
    assert_eq!(latest_job["updated_count"], 0);

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
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_json: Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_json.as_array().map(Vec::len), Some(0));

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
    assert_eq!(inventory_response.status(), StatusCode::OK);
    let inventory_body = to_bytes(inventory_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let inventory_json: Value = serde_json::from_slice(&inventory_body).unwrap();
    assert_eq!(inventory_json.as_array().map(Vec::len), Some(2));
}

fn build_multipart_payload(boundary: &str, file_name: &str, content: &str) -> Vec<u8> {
    format!(
        "--{boundary}\r\n\
Content-Disposition: form-data; name=\"files[]\"; filename=\"{file_name}\"\r\n\
Content-Type: application/jsonl\r\n\r\n\
{content}\r\n\
--{boundary}\r\n\
Content-Disposition: form-data; name=\"mode\"\r\n\r\n\
chat_gpt_session\r\n\
--{boundary}--\r\n"
    )
    .into_bytes()
}

#[tokio::test]
async fn admin_import_job_routes_work_with_retry_semantics() {
    let cipher_key = base64::engine::general_purpose::STANDARD.encode([4_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
    let app = build_app_with_store(Arc::new(store));

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "username": "admin",
                        "password": "admin123456"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    let login_body = to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: Value = serde_json::from_slice(&login_body).unwrap();
    let admin_token = login_json["access_token"].as_str().unwrap().to_string();

    let boundary = "----cp-boundary-001";
    let content = r#"{"refresh_token":"rt-valid-a","email":"a@example.com"}
{not_json_line
{"email":"missing-refresh-token@example.com"}"#;
    let payload = build_multipart_payload(boundary, "batch.jsonl", content);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts/oauth/import-jobs")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(payload))
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

    let mut latest_job = Value::Null;
    for _ in 0..30 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}"
                    ))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        latest_job = serde_json::from_slice(&body).unwrap();
        let status = latest_job["status"].as_str().unwrap_or_default();
        if !matches!(status, "queued" | "running") {
            break;
        }
        sleep(Duration::from_millis(30)).await;
    }

    assert_eq!(latest_job["created_count"], 1);
    assert_eq!(latest_job["failed_count"], 2);

    let failed_items_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/items?status=failed"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(failed_items_response.status(), StatusCode::OK);
    let failed_items_body = to_bytes(failed_items_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let failed_items_json: Value = serde_json::from_slice(&failed_items_body).unwrap();
    assert!(failed_items_json["items"].as_array().unwrap().len() >= 2);

    let retry_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/retry-failed"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(retry_response.status(), StatusCode::OK);
    let retry_body = to_bytes(retry_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let retry_json: Value = serde_json::from_slice(&retry_body).unwrap();
    assert_eq!(retry_json["accepted"], true);

    let mut retried_job = Value::Null;
    for _ in 0..30 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}"
                    ))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        retried_job = serde_json::from_slice(&body).unwrap();
        let status = retried_job["status"].as_str().unwrap_or_default();
        if !matches!(status, "queued" | "running") {
            break;
        }
        sleep(Duration::from_millis(30)).await;
    }

    // retry-failed 只重试 failed 项，已成功项不会重复计入 created。
    assert_eq!(retried_job["created_count"], 1);
    assert_eq!(retried_job["failed_count"], 2);

    let pause_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/pause"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pause_response.status(), StatusCode::OK);
    let pause_body = to_bytes(pause_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let pause_json: Value = serde_json::from_slice(&pause_body).unwrap();
    assert_eq!(pause_json["accepted"], false);

    let resume_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/resume"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resume_response.status(), StatusCode::OK);
    let resume_body = to_bytes(resume_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let resume_json: Value = serde_json::from_slice(&resume_body).unwrap();
    assert_eq!(resume_json["accepted"], false);
}

#[tokio::test]
async fn admin_console_management_endpoints_work() {
    let app = build_app();

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "username": "admin",
                        "password": "admin123456"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    let login_body = to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: Value = serde_json::from_slice(&login_body).unwrap();
    let admin_token = login_json["access_token"].as_str().unwrap().to_string();

    let system_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/system/state")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(system_response.status(), StatusCode::OK);
    let system_body = to_bytes(system_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let system_json: Value = serde_json::from_slice(&system_body).unwrap();
    assert!(system_json["counts"]["total_accounts"].is_number());
    assert!(system_json["control_plane_debug"].is_object());
    assert!(system_json["control_plane_debug"]["billing_reconcile_adjust_total"].is_number());
    assert!(system_json["control_plane_debug"]["billing_reconcile_scanned_total"].is_number());
    assert!(system_json["control_plane_debug"]["billing_reconcile_failed_total"].is_number());

    let get_config_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/config")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_config_response.status(), StatusCode::OK);

    let update_config_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/admin/config")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "notes": "integration-test-notes"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_config_response.status(), StatusCode::OK);
    let update_config_body = to_bytes(update_config_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let update_config_json: Value = serde_json::from_slice(&update_config_body).unwrap();
    assert_eq!(update_config_json["notes"], "integration-test-notes");

    let logs_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/logs?limit=20")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logs_response.status(), StatusCode::OK);
    let logs_body = to_bytes(logs_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let logs_json: Value = serde_json::from_slice(&logs_body).unwrap();
    assert!(logs_json["items"].is_array());

    let proxies_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/proxies")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(proxies_response.status(), StatusCode::OK);
    let proxies_body = to_bytes(proxies_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let proxies_json: Value = serde_json::from_slice(&proxies_body).unwrap();
    assert!(proxies_json["settings"].is_object());
    let proxy_nodes = proxies_json["nodes"]
        .as_array()
        .expect("admin proxies response should include nodes array");

    let proxy_test_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/proxies/test")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(proxy_test_response.status(), StatusCode::OK);
    let proxy_test_body = to_bytes(proxy_test_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let proxy_test_json: Value = serde_json::from_slice(&proxy_test_body).unwrap();
    let tested = proxy_test_json["tested"]
        .as_u64()
        .expect("proxy test response should include tested count");
    let results = proxy_test_json["results"]
        .as_array()
        .expect("proxy test response should include results array");
    assert_eq!(tested, proxy_nodes.len() as u64);
    assert_eq!(results.len(), proxy_nodes.len());

    let create_key_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/keys")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "integration-admin-key",
                        "tenant_name": "integration-tenant"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_key_response.status(), StatusCode::OK);
    let create_key_body = to_bytes(create_key_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_key_json: Value = serde_json::from_slice(&create_key_body).unwrap();
    let key_id = create_key_json["record"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(create_key_json["plaintext_key"]
        .as_str()
        .unwrap()
        .starts_with("cp_"));

    let list_keys_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/keys")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_keys_response.status(), StatusCode::OK);
    let list_keys_body = to_bytes(list_keys_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_keys_json: Value = serde_json::from_slice(&list_keys_body).unwrap();
    assert!(list_keys_json
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item["id"] == key_id)));

    let patch_key_response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/keys/{key_id}"))
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(patch_key_response.status(), StatusCode::OK);
    let patch_key_body = to_bytes(patch_key_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let patch_key_json: Value = serde_json::from_slice(&patch_key_body).unwrap();
    assert_eq!(patch_key_json["enabled"], false);
}
