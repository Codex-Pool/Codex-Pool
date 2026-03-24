use chrono::{DateTime, Utc};
use codex_pool_core::model::{
    AiErrorLearningSettings, ApiKey, BuiltinErrorTemplateRecord, LocalizedErrorTemplates,
    ModelRoutingPolicy, ModelRoutingSettings, ModelRoutingTriggerMode, OutboundProxyPoolSettings,
    ProxyFailMode, RoutingPlanVersion, RoutingPolicy, RoutingProfile, RoutingProfileSelector,
    RoutingStrategy, UpstreamAuthProvider, UpstreamErrorAction, UpstreamErrorRetryScope,
    UpstreamErrorTemplateRecord, UpstreamMode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub tenant_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUpstreamAccountRequest {
    pub label: String,
    pub mode: UpstreamMode,
    pub base_url: String,
    pub bearer_token: String,
    pub chatgpt_account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_provider: Option<UpstreamAuthProvider>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOutboundProxyNodeRequest {
    pub label: String,
    pub proxy_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateOutboundProxyNodeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOutboundProxyPoolSettingsRequest {
    pub enabled: bool,
    pub fail_mode: ProxyFailMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminOutboundProxyNodeView {
    pub id: Uuid,
    pub label: String,
    pub proxy_url_masked: String,
    pub scheme: String,
    pub has_auth: bool,
    pub enabled: bool,
    pub weight: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_test_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_tested_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminOutboundProxyPoolResponse {
    pub settings: OutboundProxyPoolSettings,
    #[serde(default)]
    pub nodes: Vec<AdminOutboundProxyNodeView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminOutboundProxyNodeMutationResponse {
    pub node: AdminOutboundProxyNodeView,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminOutboundProxyPoolSettingsResponse {
    pub settings: OutboundProxyPoolSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminOutboundProxyTestResponse {
    pub tested: usize,
    pub results: Vec<AdminOutboundProxyNodeView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateOAuthRefreshTokenRequest {
    pub refresh_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateOAuthRefreshTokenResponse {
    pub expires_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_account_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOAuthRefreshTokenRequest {
    pub label: String,
    pub base_url: String,
    pub refresh_token: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "access_token",
        alias = "bearer_token"
    )]
    pub fallback_access_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_token_expires_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chatgpt_account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<UpstreamMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chatgpt_plan_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "type")]
    pub source_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthRefreshStatus {
    Never,
    Ok,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthAccountPoolState {
    Active,
    Quarantine,
    PendingPurge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OAuthVaultRecordStatus {
    #[default]
    Queued,
    Ready,
    NeedsRefresh,
    NoQuota,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionCredentialKind {
    RefreshRotatable,
    OneTimeAccessToken,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefreshCredentialState {
    Healthy,
    TransientFailed,
    TerminalInvalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthLiveResultStatus {
    Ok,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthLiveResultSource {
    Active,
    Passive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthInventoryFailureStage {
    AdmissionProbe,
    ActivationRefresh,
    ActivationRateLimits,
    RuntimeRefresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAccountStatusResponse {
    pub account_id: Uuid,
    pub auth_provider: UpstreamAuthProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_kind: Option<SessionCredentialKind>,
    #[serde(default)]
    pub has_refresh_credential: bool,
    #[serde(default)]
    pub has_access_token_fallback: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_credential_state: Option<RefreshCredentialState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_identity_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_plan_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_subscription_active_start: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_subscription_active_until: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_subscription_last_checked: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_account_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_compute_residency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizations: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_family_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_version: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh_at: Option<DateTime<Utc>>,
    pub last_refresh_status: OAuthRefreshStatus,
    #[serde(default)]
    pub refresh_reused_detected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh_error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh_error: Option<String>,
    pub effective_enabled: bool,
    pub pool_state: OAuthAccountPoolState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_until: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_purge_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_purge_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_live_result_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_live_result_status: Option<OAuthLiveResultStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_live_result_source: Option<OAuthLiveResultSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_live_result_status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_live_error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_live_error_message_preview: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_models: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rate_limits: Vec<OAuthRateLimitSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits_fetched_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits_expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits_last_error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits_last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_refresh_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthInventorySummaryResponse {
    pub total: u64,
    pub queued: u64,
    pub ready: u64,
    pub needs_refresh: u64,
    pub no_quota: u64,
    pub failed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthRuntimePoolSummaryResponse {
    pub total: u64,
    pub active: u64,
    pub quarantine: u64,
    pub pending_purge: u64,
    pub oauth_refresh_token: u64,
    pub legacy_bearer: u64,
    pub rate_limits_ready: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthHealthSignalsSummaryResponse {
    pub total: u64,
    pub live_result_ok: u64,
    pub live_result_failed: u64,
    pub pending_purge_signals: u64,
    pub quarantine_signals: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthInventoryRecord {
    pub id: Uuid,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_plan_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    pub vault_status: OAuthVaultRecordStatus,
    pub has_refresh_token: bool,
    pub has_access_token_fallback: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_checked_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_retry_after: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub admission_rate_limits: Vec<OAuthRateLimitSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_rate_limits_expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_stage: Option<OAuthInventoryFailureStage>,
    #[serde(default)]
    pub attempt_count: u32,
    #[serde(default)]
    pub transient_retry_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_retry_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthRateLimitSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<OAuthRateLimitWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary: Option<OAuthRateLimitWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthRateLimitWindow {
    pub used_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_minutes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFamilyActionResponse {
    pub account_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_family_id: Option<String>,
    pub enabled: bool,
    pub affected_accounts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub record: ApiKey,
    pub plaintext_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRoutingPolicyRequest {
    pub tenant_id: Uuid,
    pub strategy: RoutingStrategy,
    pub max_retries: u32,
    pub stream_max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRetryPolicyRequest {
    pub tenant_id: Uuid,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertStreamRetryPolicyRequest {
    pub tenant_id: Uuid,
    pub stream_max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRoutingProfileRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    #[serde(default)]
    pub selector: RoutingProfileSelector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertModelRoutingPolicyRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    pub name: String,
    pub family: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exact_models: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_prefixes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallback_profile_ids: Vec<Uuid>,
    pub enabled: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordAccountModelSupportRequest {
    pub account_id: Uuid,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_models: Vec<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingProfilesResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<RoutingProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingPoliciesResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policies: Vec<ModelRoutingPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingSettingsResponse {
    pub settings: ModelRoutingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateModelRoutingSettingsRequest {
    pub enabled: bool,
    pub auto_publish: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planner_model_chain: Vec<String>,
    pub trigger_mode: ModelRoutingTriggerMode,
    pub kill_switch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPlanVersionsResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<RoutingPlanVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiErrorLearningSettingsResponse {
    pub settings: AiErrorLearningSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAiErrorLearningSettingsRequest {
    pub enabled: bool,
    pub first_seen_timeout_ms: u64,
    pub review_hit_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamErrorTemplatesResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<UpstreamErrorTemplateRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinErrorTemplatesResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<BuiltinErrorTemplateRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamErrorTemplateResponse {
    pub template: UpstreamErrorTemplateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinErrorTemplateResponse {
    pub template: BuiltinErrorTemplateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateUpstreamErrorTemplateRequest {
    pub semantic_error_code: String,
    pub action: UpstreamErrorAction,
    pub retry_scope: UpstreamErrorRetryScope,
    #[serde(default)]
    pub templates: LocalizedErrorTemplates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateBuiltinErrorTemplateRequest {
    #[serde(default)]
    pub templates: LocalizedErrorTemplates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResponse {
    pub policy: RoutingPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HourlyUsageTotalPoint {
    pub hour_start: i64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HourlyTenantUsageTotalPoint {
    pub tenant_id: Uuid,
    pub hour_start: i64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageDashboardTokenBreakdown {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageDashboardTokenTrendPoint {
    pub hour_start: i64,
    pub request_count: u64,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_microusd: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageDashboardModelDistributionItem {
    pub model: String,
    pub request_count: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageDashboardMetrics {
    pub total_requests: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_microusd: Option<i64>,
    pub token_breakdown: UsageDashboardTokenBreakdown,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_first_token_latency_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub token_trends: Vec<UsageDashboardTokenTrendPoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_request_distribution: Vec<UsageDashboardModelDistributionItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_token_distribution: Vec<UsageDashboardModelDistributionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageHourlyTrendsResponse {
    pub start_ts: i64,
    pub end_ts: i64,
    pub account_totals: Vec<HourlyUsageTotalPoint>,
    pub tenant_api_key_totals: Vec<HourlyUsageTotalPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard_metrics: Option<UsageDashboardMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageHourlyTenantTrendsResponse {
    pub start_ts: i64,
    pub end_ts: i64,
    pub items: Vec<HourlyTenantUsageTotalPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HourlyAccountUsagePoint {
    pub account_id: Uuid,
    pub hour_start: i64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HourlyTenantApiKeyUsagePoint {
    pub tenant_id: Uuid,
    pub api_key_id: Uuid,
    pub hour_start: i64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageQueryResponse<T> {
    pub items: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageSummaryQueryResponse {
    pub start_ts: i64,
    pub end_ts: i64,
    pub account_total_requests: u64,
    pub tenant_api_key_total_requests: u64,
    pub unique_account_count: u64,
    pub unique_tenant_api_key_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_microusd: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard_metrics: Option<UsageDashboardMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TenantUsageLeaderboardItem {
    pub tenant_id: Uuid,
    pub total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountUsageLeaderboardItem {
    pub account_id: Uuid,
    pub total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKeyUsageLeaderboardItem {
    pub tenant_id: Uuid,
    pub api_key_id: Uuid,
    pub total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TenantUsageLeaderboardResponse {
    pub start_ts: i64,
    pub end_ts: i64,
    pub items: Vec<TenantUsageLeaderboardItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountUsageLeaderboardResponse {
    pub start_ts: i64,
    pub end_ts: i64,
    pub items: Vec<AccountUsageLeaderboardItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKeyUsageLeaderboardResponse {
    pub start_ts: i64,
    pub end_ts: i64,
    pub items: Vec<ApiKeyUsageLeaderboardItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageLeaderboardOverviewResponse {
    pub start_ts: i64,
    pub end_ts: i64,
    pub tenants: Vec<TenantUsageLeaderboardItem>,
    pub accounts: Vec<AccountUsageLeaderboardItem>,
    pub api_keys: Vec<ApiKeyUsageLeaderboardItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<UsageSummaryQueryResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminLoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminMeResponse {
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthImportJobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthImportItemStatus {
    Pending,
    Processing,
    Created,
    Updated,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthImportJobSummary {
    pub job_id: Uuid,
    pub status: OAuthImportJobStatus,
    pub total: u64,
    pub processed: u64,
    pub created_count: u64,
    pub updated_count: u64,
    pub failed_count: u64,
    pub skipped_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput_per_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub error_summary: Vec<OAuthImportErrorSummary>,
    #[serde(default)]
    pub admission_counts: OAuthImportAdmissionCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthImportErrorSummary {
    pub error_code: String,
    pub count: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthImportAdmissionCounts {
    pub ready: u64,
    pub needs_refresh: u64,
    pub no_quota: u64,
    pub failed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthImportJobItem {
    pub item_id: u64,
    pub source_file: String,
    pub line_no: u64,
    pub status: OAuthImportItemStatus,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_status: Option<OAuthVaultRecordStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_stage: Option<OAuthInventoryFailureStage>,
    #[serde(default)]
    pub attempt_count: u32,
    #[serde(default)]
    pub transient_retry_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_retry_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthImportJobItemsResponse {
    pub items: Vec<OAuthImportJobItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthImportJobActionResponse {
    pub job_id: Uuid,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthRateLimitRefreshJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthRateLimitRefreshErrorSummary {
    pub error_code: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthRateLimitRefreshJobSummary {
    pub job_id: Uuid,
    pub status: OAuthRateLimitRefreshJobStatus,
    pub total: u64,
    pub processed: u64,
    pub success_count: u64,
    pub failed_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput_per_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub error_summary: Vec<OAuthRateLimitRefreshErrorSummary>,
}
