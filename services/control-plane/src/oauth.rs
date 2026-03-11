use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use codex_pool_core::api::{OAuthRateLimitSnapshot, OAuthRateLimitWindow};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

const DEFAULT_OPENAI_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const DEFAULT_OPENAI_OAUTH_AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const DEFAULT_OPENAI_CHATGPT_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
const DEFAULT_OPENAI_OAUTH_TIMEOUT_SEC: u64 = 15;
const DEFAULT_OAUTH_EXPIRES_IN_SEC: i64 = 3600;
const CODEX_DEFAULT_USER_AGENT: &str = "codex-cli";
const CHATGPT_ACCOUNT_ID_HEADER: &str = "ChatGPT-Account-Id";
const OPENAI_AUTH_NESTED_CLAIM_KEY: &str = "https://api.openai.com/auth";
const OPENAI_PROFILE_NESTED_CLAIM_KEY: &str = "https://api.openai.com/profile";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthRefreshErrorCode {
    InvalidRefreshToken,
    RefreshTokenReused,
    RefreshTokenRevoked,
    MissingClientId,
    UnauthorizedClient,
    RateLimited,
    UpstreamUnavailable,
    Unknown,
}

impl OAuthRefreshErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRefreshToken => "invalid_refresh_token",
            Self::RefreshTokenReused => "refresh_token_reused",
            Self::RefreshTokenRevoked => "refresh_token_revoked",
            Self::MissingClientId => "missing_client_id",
            Self::UnauthorizedClient => "unauthorized_client",
            Self::RateLimited => "rate_limited",
            Self::UpstreamUnavailable => "upstream_unavailable",
            Self::Unknown => "unknown_oauth_error",
        }
    }
}

impl std::fmt::Display for OAuthRefreshErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct OAuthTokenInfo {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub email: Option<String>,
    pub oauth_subject: Option<String>,
    pub oauth_identity_provider: Option<String>,
    pub email_verified: Option<bool>,
    pub chatgpt_account_id: Option<String>,
    pub chatgpt_user_id: Option<String>,
    pub chatgpt_plan_type: Option<String>,
    pub chatgpt_subscription_active_start: Option<DateTime<Utc>>,
    pub chatgpt_subscription_active_until: Option<DateTime<Utc>>,
    pub chatgpt_subscription_last_checked: Option<DateTime<Utc>>,
    pub chatgpt_account_user_id: Option<String>,
    pub chatgpt_compute_residency: Option<String>,
    pub organizations: Option<Vec<Value>>,
    pub groups: Option<Vec<Value>>,
}

#[derive(Debug, Error)]
pub enum OAuthTokenClientError {
    #[error("oauth token endpoint is not configured")]
    NotConfigured,
    #[error("invalid refresh token ({code}): {message}")]
    InvalidRefreshToken {
        code: OAuthRefreshErrorCode,
        message: String,
    },
    #[error("oauth token endpoint error ({code}): {message}")]
    Upstream {
        code: OAuthRefreshErrorCode,
        message: String,
    },
    #[error("oauth token response parse error")]
    Parse,
}

impl OAuthTokenClientError {
    pub fn code(&self) -> OAuthRefreshErrorCode {
        match self {
            Self::NotConfigured => OAuthRefreshErrorCode::Unknown,
            Self::InvalidRefreshToken { code, .. } => *code,
            Self::Upstream { code, .. } => *code,
            Self::Parse => OAuthRefreshErrorCode::Unknown,
        }
    }
}

#[async_trait]
pub trait OAuthTokenClient: Send + Sync {
    async fn refresh_token(
        &self,
        refresh_token: &str,
        base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, OAuthTokenClientError>;

    async fn fetch_rate_limits(
        &self,
        _access_token: &str,
        _base_url: Option<&str>,
        _chatgpt_account_id: Option<&str>,
    ) -> Result<Vec<OAuthRateLimitSnapshot>, OAuthTokenClientError> {
        Ok(Vec::new())
    }
}

#[derive(Clone)]
pub struct OpenAiOAuthClient {
    http_client: reqwest::Client,
    token_url: String,
    authorize_url: String,
    client_id: Option<String>,
}

impl OpenAiOAuthClient {
    pub fn from_env() -> Self {
        let token_url = std::env::var("OPENAI_OAUTH_TOKEN_URL")
            .unwrap_or_else(|_| DEFAULT_OPENAI_OAUTH_TOKEN_URL.to_string());
        let authorize_url = std::env::var("OPENAI_OAUTH_AUTHORIZE_URL")
            .unwrap_or_else(|_| DEFAULT_OPENAI_OAUTH_AUTHORIZE_URL.to_string());
        let timeout_sec = std::env::var("OPENAI_OAUTH_TIMEOUT_SEC")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_OPENAI_OAUTH_TIMEOUT_SEC);
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_sec))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http_client,
            token_url,
            authorize_url,
            client_id: std::env::var("OPENAI_OAUTH_CLIENT_ID").ok(),
        }
    }

    pub fn from_parts(
        token_url: impl Into<String>,
        authorize_url: impl Into<String>,
        client_id: Option<String>,
        timeout_sec: Option<u64>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                timeout_sec.unwrap_or(DEFAULT_OPENAI_OAUTH_TIMEOUT_SEC),
            ))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http_client,
            token_url: token_url.into(),
            authorize_url: authorize_url.into(),
            client_id,
        }
    }

    pub fn build_authorize_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: &str,
        scope: Option<&str>,
    ) -> Result<String, OAuthTokenClientError> {
        if self.authorize_url.trim().is_empty() {
            return Err(OAuthTokenClientError::NotConfigured);
        }
        let client_id = self
            .client_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(OAuthTokenClientError::NotConfigured)?;
        let mut url = reqwest::Url::parse(&self.authorize_url)
            .map_err(|_| OAuthTokenClientError::NotConfigured)?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("response_type", "code");
            query.append_pair("client_id", client_id);
            query.append_pair("redirect_uri", redirect_uri.trim());
            query.append_pair("state", state.trim());
            query.append_pair("code_challenge", code_challenge.trim());
            query.append_pair("code_challenge_method", "S256");
            // Align with Codex OAuth web/CLI flow requirements used by CLIProxyAPI.
            query.append_pair("prompt", "login");
            query.append_pair("id_token_add_organizations", "true");
            query.append_pair("codex_cli_simplified_flow", "true");
            if let Some(scope) = scope.map(str::trim).filter(|value| !value.is_empty()) {
                query.append_pair("scope", scope);
            }
        }
        Ok(url.into())
    }

    pub async fn exchange_authorization_code(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<OAuthCodeExchangeInfo, OAuthTokenClientError> {
        if self.token_url.trim().is_empty() {
            return Err(OAuthTokenClientError::NotConfigured);
        }
        let client_id = self
            .client_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(OAuthTokenClientError::NotConfigured)?;

        let form = vec![
            ("grant_type", "authorization_code".to_string()),
            ("client_id", client_id.to_string()),
            ("code", code.trim().to_string()),
            ("redirect_uri", redirect_uri.trim().to_string()),
            ("code_verifier", code_verifier.trim().to_string()),
        ];

        let response = self
            .http_client
            .post(&self.token_url)
            .form(&form)
            .send()
            .await
            .map_err(|err| OAuthTokenClientError::Upstream {
                code: OAuthRefreshErrorCode::UpstreamUnavailable,
                message: err.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let raw_body = response.text().await.unwrap_or_default();
            let parsed_error = serde_json::from_str::<OAuthTokenEndpointError>(&raw_body).ok();
            let error_code = classify_oauth_error_code(status, parsed_error.as_ref(), &raw_body);
            let message = parsed_error
                .as_ref()
                .and_then(|item| {
                    item.error_description
                        .clone()
                        .or_else(|| item.message.clone())
                })
                .or_else(|| parsed_error.and_then(|item| item.error))
                .unwrap_or_else(|| truncate_message(&raw_body, 256));
            return Err(OAuthTokenClientError::Upstream {
                code: error_code,
                message: format!("status={status} message={message}"),
            });
        }

        let payload = response
            .json::<OAuthTokenEndpointResponse>()
            .await
            .map_err(|_| OAuthTokenClientError::Parse)?;
        let raw_payload = serde_json::to_value(&payload).unwrap_or(Value::Null);
        let expires_in = payload
            .expires_in
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_OAUTH_EXPIRES_IN_SEC);
        let id_token_claims = payload.id_token.as_deref().and_then(parse_id_token_claims);
        let chatgpt_account_id = payload.chatgpt_account_id.clone().or_else(|| {
            id_token_claims
                .as_ref()
                .and_then(|claims| claims.chatgpt_account_id.clone())
        });

        Ok(OAuthCodeExchangeInfo {
            access_token: payload.access_token,
            refresh_token: payload.refresh_token,
            expires_at: Utc::now() + Duration::seconds(expires_in),
            token_type: payload.token_type,
            scope: payload.scope,
            chatgpt_account_id,
            id_token_claims,
            id_token_raw: payload.id_token,
            raw_payload,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OAuthIdTokenClaims {
    pub email: Option<String>,
    pub oauth_subject: Option<String>,
    pub oauth_identity_provider: Option<String>,
    pub email_verified: Option<bool>,
    pub chatgpt_account_id: Option<String>,
    pub chatgpt_user_id: Option<String>,
    pub chatgpt_plan_type: Option<String>,
    pub chatgpt_subscription_active_start: Option<DateTime<Utc>>,
    pub chatgpt_subscription_active_until: Option<DateTime<Utc>>,
    pub chatgpt_subscription_last_checked: Option<DateTime<Utc>>,
    pub organizations: Option<Vec<Value>>,
    pub groups: Option<Vec<Value>>,
    pub raw_claims: Value,
}

#[derive(Debug, Clone)]
struct OAuthAccessTokenClaims {
    oauth_subject: Option<String>,
    email: Option<String>,
    email_verified: Option<bool>,
    chatgpt_account_id: Option<String>,
    chatgpt_account_user_id: Option<String>,
    chatgpt_compute_residency: Option<String>,
    chatgpt_plan_type: Option<String>,
    chatgpt_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthCodeExchangeInfo {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub chatgpt_account_id: Option<String>,
    pub id_token_claims: Option<OAuthIdTokenClaims>,
    pub id_token_raw: Option<String>,
    pub raw_payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthTokenEndpointResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenEndpointError {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RateLimitStatusPayload {
    #[serde(default)]
    rate_limit: Option<RateLimitStatusDetails>,
    #[serde(default)]
    additional_rate_limits: Option<Vec<AdditionalRateLimitDetails>>,
}

#[derive(Debug, Deserialize)]
struct RateLimitStatusDetails {
    #[serde(default)]
    primary_window: Option<RateLimitWindowSnapshot>,
    #[serde(default)]
    secondary_window: Option<RateLimitWindowSnapshot>,
}

#[derive(Debug, Deserialize)]
struct AdditionalRateLimitDetails {
    #[serde(default)]
    limit_name: Option<String>,
    #[serde(default)]
    metered_feature: Option<String>,
    #[serde(default)]
    rate_limit: Option<RateLimitStatusDetails>,
}

#[derive(Debug, Deserialize)]
struct RateLimitWindowSnapshot {
    #[serde(default)]
    used_percent: Option<f64>,
    #[serde(default)]
    limit_window_seconds: Option<i64>,
    #[serde(default)]
    reset_at: Option<i64>,
}

#[async_trait]
impl OAuthTokenClient for OpenAiOAuthClient {
    async fn refresh_token(
        &self,
        refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, OAuthTokenClientError> {
        if self.token_url.trim().is_empty() {
            return Err(OAuthTokenClientError::NotConfigured);
        }

        let mut form = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token.to_string()),
        ];
        if let Some(client_id) = self.client_id.as_deref() {
            form.push(("client_id", client_id.to_string()));
        }

        let response = self
            .http_client
            .post(&self.token_url)
            .form(&form)
            .send()
            .await
            .map_err(|err| OAuthTokenClientError::Upstream {
                code: OAuthRefreshErrorCode::UpstreamUnavailable,
                message: err.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let raw_body = response.text().await.unwrap_or_default();
            let parsed_error = serde_json::from_str::<OAuthTokenEndpointError>(&raw_body).ok();
            let error_code = classify_oauth_error_code(status, parsed_error.as_ref(), &raw_body);
            let message = parsed_error
                .as_ref()
                .and_then(|item| {
                    item.error_description
                        .clone()
                        .or_else(|| item.message.clone())
                })
                .or_else(|| parsed_error.and_then(|item| item.error))
                .unwrap_or_else(|| truncate_message(&raw_body, 256));

            return if matches!(status, StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED)
                && !matches!(
                    error_code,
                    OAuthRefreshErrorCode::MissingClientId
                        | OAuthRefreshErrorCode::UnauthorizedClient
                ) {
                Err(OAuthTokenClientError::InvalidRefreshToken {
                    code: error_code,
                    message,
                })
            } else {
                Err(OAuthTokenClientError::Upstream {
                    code: error_code,
                    message: format!("status={status} message={message}"),
                })
            };
        }

        let payload = response
            .json::<OAuthTokenEndpointResponse>()
            .await
            .map_err(|_| OAuthTokenClientError::Parse)?;
        let expires_in = payload
            .expires_in
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_OAUTH_EXPIRES_IN_SEC);
        let id_token_claims = payload.id_token.as_deref().and_then(parse_id_token_claims);
        let access_token_claims = parse_access_token_claims(&payload.access_token);
        let chatgpt_account_id = payload
            .chatgpt_account_id
            .clone()
            .or_else(|| {
                id_token_claims
                    .as_ref()
                    .and_then(|claims| claims.chatgpt_account_id.clone())
            })
            .or_else(|| {
                access_token_claims
                    .as_ref()
                    .and_then(|claims| claims.chatgpt_account_id.clone())
            });
        let chatgpt_plan_type = id_token_claims
            .as_ref()
            .and_then(|claims| claims.chatgpt_plan_type.clone())
            .or_else(|| {
                access_token_claims
                    .as_ref()
                    .and_then(|claims| claims.chatgpt_plan_type.clone())
            });
        let email = id_token_claims
            .as_ref()
            .and_then(|claims| claims.email.clone())
            .or_else(|| {
                access_token_claims
                    .as_ref()
                    .and_then(|claims| claims.email.clone())
            });

        Ok(OAuthTokenInfo {
            access_token: payload.access_token,
            refresh_token: payload
                .refresh_token
                .unwrap_or_else(|| refresh_token.to_string()),
            expires_at: Utc::now() + Duration::seconds(expires_in),
            token_type: payload.token_type,
            scope: payload.scope,
            email,
            oauth_subject: id_token_claims
                .as_ref()
                .and_then(|claims| claims.oauth_subject.clone())
                .or_else(|| {
                    access_token_claims
                        .as_ref()
                        .and_then(|claims| claims.oauth_subject.clone())
                }),
            oauth_identity_provider: id_token_claims
                .as_ref()
                .and_then(|claims| claims.oauth_identity_provider.clone()),
            email_verified: id_token_claims
                .as_ref()
                .and_then(|claims| claims.email_verified)
                .or_else(|| {
                    access_token_claims
                        .as_ref()
                        .and_then(|claims| claims.email_verified)
                }),
            chatgpt_account_id,
            chatgpt_user_id: id_token_claims
                .as_ref()
                .and_then(|claims| claims.chatgpt_user_id.clone())
                .or_else(|| {
                    access_token_claims
                        .as_ref()
                        .and_then(|claims| claims.chatgpt_user_id.clone())
                }),
            chatgpt_plan_type,
            chatgpt_subscription_active_start: id_token_claims
                .as_ref()
                .and_then(|claims| claims.chatgpt_subscription_active_start),
            chatgpt_subscription_active_until: id_token_claims
                .as_ref()
                .and_then(|claims| claims.chatgpt_subscription_active_until),
            chatgpt_subscription_last_checked: id_token_claims
                .as_ref()
                .and_then(|claims| claims.chatgpt_subscription_last_checked),
            chatgpt_account_user_id: access_token_claims
                .as_ref()
                .and_then(|claims| claims.chatgpt_account_user_id.clone()),
            chatgpt_compute_residency: access_token_claims
                .as_ref()
                .and_then(|claims| claims.chatgpt_compute_residency.clone()),
            organizations: id_token_claims
                .as_ref()
                .and_then(|claims| claims.organizations.clone()),
            groups: id_token_claims
                .as_ref()
                .and_then(|claims| claims.groups.clone()),
        })
    }

    async fn fetch_rate_limits(
        &self,
        access_token: &str,
        base_url: Option<&str>,
        chatgpt_account_id: Option<&str>,
    ) -> Result<Vec<OAuthRateLimitSnapshot>, OAuthTokenClientError> {
        let trimmed_access_token = access_token.trim();
        if trimmed_access_token.is_empty() {
            return Ok(Vec::new());
        }

        let usage_url = resolve_usage_endpoint(base_url);
        let mut request = self
            .http_client
            .get(&usage_url)
            .bearer_auth(trimmed_access_token)
            .header(reqwest::header::USER_AGENT, CODEX_DEFAULT_USER_AGENT);
        if let Some(account_id) = chatgpt_account_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            request = request.header(CHATGPT_ACCOUNT_ID_HEADER, account_id);
        }

        let response = request
            .send()
            .await
            .map_err(|err| OAuthTokenClientError::Upstream {
                code: OAuthRefreshErrorCode::UpstreamUnavailable,
                message: err.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let raw_body = response.text().await.unwrap_or_default();
            let parsed_error = serde_json::from_str::<OAuthTokenEndpointError>(&raw_body).ok();
            let error_code = classify_oauth_error_code(status, parsed_error.as_ref(), &raw_body);
            let message = parsed_error
                .as_ref()
                .and_then(|item| {
                    item.error_description
                        .clone()
                        .or_else(|| item.message.clone())
                })
                .or_else(|| parsed_error.and_then(|item| item.error))
                .unwrap_or_else(|| truncate_message(&raw_body, 256));
            return Err(OAuthTokenClientError::Upstream {
                code: error_code,
                message: format!("status={status} message={message}"),
            });
        }

        let payload = response
            .json::<RateLimitStatusPayload>()
            .await
            .map_err(|_| OAuthTokenClientError::Parse)?;
        Ok(rate_limit_snapshots_from_payload(payload))
    }
}

fn resolve_usage_endpoint(base_url: Option<&str>) -> String {
    let normalized = normalize_chatgpt_base_url(base_url);
    if let Some((prefix, _)) = normalized.split_once("/backend-api") {
        format!("{prefix}/backend-api/wham/usage")
    } else {
        format!("{normalized}/api/codex/usage")
    }
}

fn normalize_chatgpt_base_url(base_url: Option<&str>) -> String {
    let mut normalized = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_OPENAI_CHATGPT_BASE_URL)
        .to_string();
    while normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

fn rate_limit_snapshots_from_payload(
    payload: RateLimitStatusPayload,
) -> Vec<OAuthRateLimitSnapshot> {
    let mut snapshots = Vec::new();
    if let Some(rate_limit) = payload.rate_limit {
        if let Some(snapshot) =
            to_rate_limit_snapshot(Some("codex".to_string()), None, Some(rate_limit))
        {
            snapshots.push(snapshot);
        }
    }

    if let Some(additional_limits) = payload.additional_rate_limits {
        for limit in additional_limits {
            if let Some(snapshot) =
                to_rate_limit_snapshot(limit.metered_feature, limit.limit_name, limit.rate_limit)
            {
                snapshots.push(snapshot);
            }
        }
    }

    snapshots
}

fn to_rate_limit_snapshot(
    limit_id: Option<String>,
    limit_name: Option<String>,
    details: Option<RateLimitStatusDetails>,
) -> Option<OAuthRateLimitSnapshot> {
    let details = details?;
    let primary = to_rate_limit_window(details.primary_window);
    let secondary = to_rate_limit_window(details.secondary_window);
    if primary.is_none() && secondary.is_none() {
        return None;
    }

    Some(OAuthRateLimitSnapshot {
        limit_id,
        limit_name,
        primary,
        secondary,
    })
}

fn to_rate_limit_window(window: Option<RateLimitWindowSnapshot>) -> Option<OAuthRateLimitWindow> {
    let window = window?;
    Some(OAuthRateLimitWindow {
        used_percent: window.used_percent.unwrap_or(0.0),
        window_minutes: window
            .limit_window_seconds
            .filter(|seconds| *seconds > 0)
            .map(|seconds| (seconds + 59) / 60),
        resets_at: window
            .reset_at
            .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0)),
    })
}

fn truncate_message(message: &str, max_len: usize) -> String {
    if message.len() <= max_len {
        return message.to_string();
    }

    message.chars().take(max_len).collect()
}

fn parse_id_token_claims(id_token: &str) -> Option<OAuthIdTokenClaims> {
    let mut segments = id_token.split('.');
    let _header = segments.next()?;
    let payload = segments.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let claims: Value = serde_json::from_slice(&decoded).ok()?;
    let nested_auth_claims = claims
        .get(OPENAI_AUTH_NESTED_CLAIM_KEY)
        .filter(|value| value.is_object());

    Some(OAuthIdTokenClaims {
        email: claim_string(&claims, &["email"]),
        oauth_subject: claim_string(&claims, &["sub"]),
        oauth_identity_provider: claim_string(&claims, &["auth_provider"]),
        email_verified: claim_bool(&claims, &["email_verified"]),
        chatgpt_account_id: nested_auth_claims
            .and_then(|nested| {
                claim_string(
                    nested,
                    &[
                        "chatgpt_account_id",
                        "chatgptAccountId",
                        "account_id",
                        "accountId",
                    ],
                )
            })
            .or_else(|| {
                claim_string(
                    &claims,
                    &[
                        "chatgpt_account_id",
                        "chatgptAccountId",
                        "account_id",
                        "accountId",
                    ],
                )
            }),
        chatgpt_user_id: nested_auth_claims
            .and_then(|nested| {
                claim_string(nested, &["chatgpt_user_id", "chatgptUserId", "user_id", "userId"])
            })
            .or_else(|| {
                claim_string(
                    &claims,
                    &["chatgpt_user_id", "chatgptUserId", "user_id", "userId"],
                )
            }),
        chatgpt_plan_type: nested_auth_claims
            .and_then(|nested| {
                claim_string(
                    nested,
                    &[
                        "chatgpt_plan_type",
                        "chatgptPlanType",
                        "plan_type",
                        "planType",
                        "plan",
                    ],
                )
            })
            .or_else(|| {
                claim_string(
                    &claims,
                    &[
                        "chatgpt_plan_type",
                        "chatgptPlanType",
                        "plan_type",
                        "planType",
                        "plan",
                    ],
                )
            }),
        chatgpt_subscription_active_start: nested_auth_claims.and_then(|nested| {
            claim_datetime(
                nested,
                &[
                    "chatgpt_subscription_active_start",
                    "chatgptSubscriptionActiveStart",
                ],
            )
        }),
        chatgpt_subscription_active_until: nested_auth_claims.and_then(|nested| {
            claim_datetime(
                nested,
                &[
                    "chatgpt_subscription_active_until",
                    "chatgptSubscriptionActiveUntil",
                ],
            )
        }),
        chatgpt_subscription_last_checked: nested_auth_claims.and_then(|nested| {
            claim_datetime(
                nested,
                &[
                    "chatgpt_subscription_last_checked",
                    "chatgptSubscriptionLastChecked",
                ],
            )
        }),
        organizations: nested_auth_claims
            .and_then(|nested| claim_array(nested, &["organizations"])),
        groups: nested_auth_claims.and_then(|nested| claim_array(nested, &["groups"])),
        raw_claims: claims,
    })
}

fn parse_access_token_claims(access_token: &str) -> Option<OAuthAccessTokenClaims> {
    let mut segments = access_token.split('.');
    let _header = segments.next()?;
    let payload = segments.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let claims: Value = serde_json::from_slice(&decoded).ok()?;
    let nested_auth_claims = claims
        .get(OPENAI_AUTH_NESTED_CLAIM_KEY)
        .filter(|value| value.is_object());
    let nested_profile_claims = claims
        .get(OPENAI_PROFILE_NESTED_CLAIM_KEY)
        .filter(|value| value.is_object());

    Some(OAuthAccessTokenClaims {
        oauth_subject: claim_string(&claims, &["sub"]),
        email: nested_profile_claims
            .and_then(|nested| claim_string(nested, &["email"]))
            .or_else(|| claim_string(&claims, &["email"])),
        email_verified: nested_profile_claims
            .and_then(|nested| claim_bool(nested, &["email_verified"]))
            .or_else(|| claim_bool(&claims, &["email_verified"])),
        chatgpt_account_id: nested_auth_claims.and_then(|nested| {
            claim_string(
                nested,
                &[
                    "chatgpt_account_id",
                    "chatgptAccountId",
                    "account_id",
                    "accountId",
                ],
            )
        }),
        chatgpt_account_user_id: nested_auth_claims.and_then(|nested| {
            claim_string(
                nested,
                &["chatgpt_account_user_id", "chatgptAccountUserId"],
            )
        }),
        chatgpt_compute_residency: nested_auth_claims.and_then(|nested| {
            claim_string(
                nested,
                &["chatgpt_compute_residency", "chatgptComputeResidency"],
            )
        }),
        chatgpt_plan_type: nested_auth_claims.and_then(|nested| {
            claim_string(
                nested,
                &[
                    "chatgpt_plan_type",
                    "chatgptPlanType",
                    "plan_type",
                    "planType",
                    "plan",
                ],
            )
        }),
        chatgpt_user_id: nested_auth_claims.and_then(|nested| {
            claim_string(nested, &["chatgpt_user_id", "chatgptUserId", "user_id", "userId"])
        }),
    })
}

fn claim_string(claims: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let value = claims
            .get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(value) = value {
            return Some(value.to_string());
        }
    }
    None
}

fn claim_bool(claims: &Value, keys: &[&str]) -> Option<bool> {
    for key in keys {
        if let Some(value) = claims.get(*key).and_then(|value| value.as_bool()) {
            return Some(value);
        }
    }
    None
}

fn claim_datetime(claims: &Value, keys: &[&str]) -> Option<DateTime<Utc>> {
    for key in keys {
        let value = claims
            .get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(value) = value {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
                return Some(parsed.with_timezone(&Utc));
            }
        }
    }
    None
}

fn claim_array(claims: &Value, keys: &[&str]) -> Option<Vec<Value>> {
    for key in keys {
        if let Some(values) = claims.get(*key).and_then(|value| value.as_array()) {
            return Some(values.clone());
        }
    }
    None
}

fn classify_oauth_error_code(
    status: StatusCode,
    parsed_error: Option<&OAuthTokenEndpointError>,
    raw_body: &str,
) -> OAuthRefreshErrorCode {
    if status == StatusCode::TOO_MANY_REQUESTS {
        return OAuthRefreshErrorCode::RateLimited;
    }
    if status.is_server_error() {
        return OAuthRefreshErrorCode::UpstreamUnavailable;
    }

    let mut search_space = String::new();
    if let Some(parsed_error) = parsed_error {
        if let Some(error) = parsed_error.error.as_deref() {
            search_space.push_str(error);
            search_space.push('\n');
        }
        if let Some(description) = parsed_error.error_description.as_deref() {
            search_space.push_str(description);
            search_space.push('\n');
        }
        if let Some(message) = parsed_error.message.as_deref() {
            search_space.push_str(message);
        }
    }
    if search_space.is_empty() {
        search_space.push_str(raw_body);
    }
    let lowered = search_space.to_ascii_lowercase();

    if lowered.contains("refresh_token_reused")
        || lowered.contains("token reused")
        || lowered.contains("token has already been used")
    {
        return OAuthRefreshErrorCode::RefreshTokenReused;
    }
    if lowered.contains("refresh_token_revoked")
        || lowered.contains("token revoked")
        || lowered.contains("invalid_grant") && lowered.contains("revoked")
    {
        return OAuthRefreshErrorCode::RefreshTokenRevoked;
    }
    if lowered.contains("missing_client_id")
        || lowered.contains("client_id is required")
        || lowered.contains("client id is required")
    {
        return OAuthRefreshErrorCode::MissingClientId;
    }
    if lowered.contains("unauthorized_client") {
        return OAuthRefreshErrorCode::UnauthorizedClient;
    }
    if lowered.contains("rate limit") || lowered.contains("too many requests") {
        return OAuthRefreshErrorCode::RateLimited;
    }
    if matches!(status, StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED) {
        return OAuthRefreshErrorCode::InvalidRefreshToken;
    }

    OAuthRefreshErrorCode::Unknown
}

#[cfg(test)]
mod tests {
    use super::{
        classify_oauth_error_code, parse_access_token_claims, parse_id_token_claims,
        resolve_usage_endpoint,
        OAuthRefreshErrorCode, OAuthTokenEndpointError,
    };
    use base64::Engine;
    use chrono::{DateTime, Utc};
    use reqwest::StatusCode;
    use serde_json::json;

    #[test]
    fn classify_reused_refresh_token_error() {
        let parsed = OAuthTokenEndpointError {
            error: Some("invalid_grant".to_string()),
            error_description: Some("refresh_token_reused".to_string()),
            message: None,
        };
        let code = classify_oauth_error_code(StatusCode::BAD_REQUEST, Some(&parsed), "");
        assert_eq!(code, OAuthRefreshErrorCode::RefreshTokenReused);
    }

    #[test]
    fn classify_missing_client_id_error() {
        let parsed = OAuthTokenEndpointError {
            error: Some("invalid_request".to_string()),
            error_description: Some("client_id is required".to_string()),
            message: None,
        };
        let code = classify_oauth_error_code(StatusCode::BAD_REQUEST, Some(&parsed), "");
        assert_eq!(code, OAuthRefreshErrorCode::MissingClientId);
    }

    #[test]
    fn classify_rate_limited_by_status_code() {
        let parsed = OAuthTokenEndpointError {
            error: None,
            error_description: None,
            message: None,
        };
        let code = classify_oauth_error_code(StatusCode::TOO_MANY_REQUESTS, Some(&parsed), "");
        assert_eq!(code, OAuthRefreshErrorCode::RateLimited);
    }

    #[test]
    fn resolve_usage_endpoint_uses_backend_api_root_for_codex_base() {
        let endpoint = resolve_usage_endpoint(Some("https://chatgpt.com/backend-api/codex"));
        assert_eq!(endpoint, "https://chatgpt.com/backend-api/wham/usage");
    }

    #[test]
    fn resolve_usage_endpoint_uses_backend_api_root_for_backend_api_base() {
        let endpoint = resolve_usage_endpoint(Some("https://chatgpt.com/backend-api/"));
        assert_eq!(endpoint, "https://chatgpt.com/backend-api/wham/usage");
    }

    #[test]
    fn resolve_usage_endpoint_falls_back_for_non_backend_api_base() {
        let endpoint = resolve_usage_endpoint(Some("https://example.com/codex"));
        assert_eq!(endpoint, "https://example.com/codex/api/codex/usage");
    }

    #[test]
    fn parse_id_token_claims_reads_nested_openai_auth_claims() {
        let payload = json!({
            "email": "demo@example.com",
            "https://api.openai.com/auth": {
                "chatgpt_account_id": "acct_nested",
                "chatgpt_plan_type": "team"
            }
        });
        let payload_bytes = serde_json::to_vec(&payload).expect("serialize payload");
        let payload_segment =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_bytes);
        let token = format!("header.{payload_segment}.signature");

        let claims = parse_id_token_claims(&token).expect("claims should parse");
        assert_eq!(claims.email.as_deref(), Some("demo@example.com"));
        assert_eq!(claims.chatgpt_account_id.as_deref(), Some("acct_nested"));
        assert_eq!(claims.chatgpt_plan_type.as_deref(), Some("team"));
    }

    #[test]
    fn parse_id_token_claims_falls_back_to_top_level_fields() {
        let payload = json!({
            "email": "demo@example.com",
            "chatgpt_account_id": "acct_top",
            "chatgpt_plan_type": "pro"
        });
        let payload_bytes = serde_json::to_vec(&payload).expect("serialize payload");
        let payload_segment =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_bytes);
        let token = format!("header.{payload_segment}.signature");

        let claims = parse_id_token_claims(&token).expect("claims should parse");
        assert_eq!(claims.chatgpt_account_id.as_deref(), Some("acct_top"));
        assert_eq!(claims.chatgpt_plan_type.as_deref(), Some("pro"));
    }

    #[test]
    fn parse_id_token_claims_reads_structured_identity_and_subscription_fields() {
        let payload = json!({
            "sub": "google-oauth2|1234567890",
            "auth_provider": "google",
            "email": "demo@example.com",
            "email_verified": true,
            "https://api.openai.com/auth": {
                "chatgpt_account_id": "acct_nested",
                "chatgpt_plan_type": "team",
                "chatgpt_user_id": "user_shared",
                "chatgpt_subscription_active_start": "2026-03-07T07:34:14+00:00",
                "chatgpt_subscription_active_until": "2026-04-07T07:34:14+00:00",
                "chatgpt_subscription_last_checked": "2026-03-11T03:58:04.173746+00:00",
                "organizations": [
                    {
                        "id": "org-123",
                        "title": "Personal",
                        "role": "owner",
                        "is_default": true
                    }
                ],
                "groups": [
                    {
                        "id": "grp-1",
                        "title": "Workspace A"
                    }
                ]
            }
        });
        let payload_bytes = serde_json::to_vec(&payload).expect("serialize payload");
        let payload_segment =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_bytes);
        let token = format!("header.{payload_segment}.signature");

        let claims = parse_id_token_claims(&token).expect("claims should parse");
        assert_eq!(claims.oauth_subject.as_deref(), Some("google-oauth2|1234567890"));
        assert_eq!(claims.oauth_identity_provider.as_deref(), Some("google"));
        assert_eq!(claims.email.as_deref(), Some("demo@example.com"));
        assert_eq!(claims.email_verified, Some(true));
        assert_eq!(claims.chatgpt_user_id.as_deref(), Some("user_shared"));
        assert_eq!(
            claims.chatgpt_subscription_active_start,
            Some(DateTime::parse_from_rfc3339("2026-03-07T07:34:14+00:00").unwrap().with_timezone(&Utc))
        );
        assert_eq!(
            claims.chatgpt_subscription_active_until,
            Some(DateTime::parse_from_rfc3339("2026-04-07T07:34:14+00:00").unwrap().with_timezone(&Utc))
        );
        assert_eq!(
            claims.chatgpt_subscription_last_checked,
            Some(DateTime::parse_from_rfc3339("2026-03-11T03:58:04.173746+00:00").unwrap().with_timezone(&Utc))
        );
        assert_eq!(claims.organizations.as_ref().map(Vec::len), Some(1));
        assert_eq!(claims.groups.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn parse_access_token_claims_reads_account_instance_fields() {
        let payload = json!({
            "sub": "google-oauth2|1234567890",
            "https://api.openai.com/auth": {
                "chatgpt_account_id": "acct_nested",
                "chatgpt_account_user_id": "user_shared__acct_nested",
                "chatgpt_compute_residency": "no_constraint",
                "chatgpt_plan_type": "team",
                "chatgpt_user_id": "user_shared"
            },
            "https://api.openai.com/profile": {
                "email": "demo@example.com",
                "email_verified": true
            }
        });
        let payload_bytes = serde_json::to_vec(&payload).expect("serialize payload");
        let payload_segment =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_bytes);
        let token = format!("header.{payload_segment}.signature");

        let claims = parse_access_token_claims(&token).expect("claims should parse");
        assert_eq!(claims.oauth_subject.as_deref(), Some("google-oauth2|1234567890"));
        assert_eq!(claims.email.as_deref(), Some("demo@example.com"));
        assert_eq!(claims.email_verified, Some(true));
        assert_eq!(claims.chatgpt_account_id.as_deref(), Some("acct_nested"));
        assert_eq!(
            claims.chatgpt_account_user_id.as_deref(),
            Some("user_shared__acct_nested")
        );
        assert_eq!(
            claims.chatgpt_compute_residency.as_deref(),
            Some("no_constraint")
        );
        assert_eq!(claims.chatgpt_plan_type.as_deref(), Some("team"));
        assert_eq!(claims.chatgpt_user_id.as_deref(), Some("user_shared"));
    }
}
