use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::model::{
    AccountRoutingTraits, ApiKey, ModelRoutingSettings, ModelRoutingTriggerMode,
    CompiledRoutingPlan, ModelRoutingPolicy, RoutingPlanVersion, RoutingPolicy, RoutingProfile,
    RoutingProfileSelector, RoutingStrategy, UpstreamAccount, UpstreamAuthProvider,
    UpstreamMode,
};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionCredentialKind {
    RefreshRotatable,
    OneTimeAccessToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAccountStatusResponse {
    pub account_id: Uuid,
    pub auth_provider: UpstreamAuthProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_kind: Option<SessionCredentialKind>,
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
pub struct DataPlaneSnapshot {
    pub revision: u64,
    #[serde(default)]
    pub cursor: u64,
    pub accounts: Vec<UpstreamAccount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub account_traits: Vec<AccountRoutingTraits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiled_routing_plan: Option<CompiledRoutingPlan>,
    pub issued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataPlaneSnapshotEventType {
    AccountUpsert,
    AccountDelete,
    RoutingPlanRefresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPlaneSnapshotEvent {
    pub id: u64,
    pub event_type: DataPlaneSnapshotEventType,
    pub account_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<UpstreamAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiled_routing_plan: Option<CompiledRoutingPlan>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPlaneSnapshotEventsResponse {
    pub cursor: u64,
    #[serde(default)]
    pub high_watermark: u64,
    #[serde(default)]
    pub events: Vec<DataPlaneSnapshotEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub record: ApiKey,
    pub plaintext_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateApiKeyRequest {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateApiKeyResponse {
    pub tenant_id: Uuid,
    pub api_key_id: Uuid,
    pub enabled: bool,
    #[serde(default)]
    pub group: ApiKeyGroupStatus,
    #[serde(default)]
    pub policy: ApiKeyPolicy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_microcredits: Option<i64>,
    pub cache_ttl_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeyGroupStatus {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub invalid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ApiKeyPolicy {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ip_allowlist: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_allowlist: Vec<String>,
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
pub struct PolicyResponse {
    pub policy: RoutingPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub account_total: usize,
    pub active_account_total: usize,
    pub window_limit_tokens: u64,
    pub window_used_tokens: u64,
    pub window_reset_at: DateTime<Utc>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthImportErrorSummary {
    pub error_code: String,
    pub count: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub message: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

impl ErrorEnvelope {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code: code.into(),
                message: message.into(),
                r#type: None,
            },
        }
    }
}
