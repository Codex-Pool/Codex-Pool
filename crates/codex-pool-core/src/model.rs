use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKey {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpstreamMode {
    #[serde(alias = "openai", alias = "api_key")]
    OpenAiApiKey,
    #[serde(alias = "chat_gpt_oauth", alias = "chatgpt", alias = "chatgpt_oauth")]
    ChatGptSession,
    #[serde(alias = "codex_session", alias = "codex")]
    CodexOauth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpstreamAuthProvider {
    LegacyBearer,
    #[serde(rename = "oauth_refresh_token")]
    OAuthRefreshToken,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpstreamAccount {
    pub id: Uuid,
    pub label: String,
    pub mode: UpstreamMode,
    pub base_url: String,
    pub bearer_token: String,
    pub chatgpt_account_id: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RoutingProfileSelector {
    #[serde(default)]
    pub plan_types: Vec<String>,
    #[serde(default)]
    pub modes: Vec<UpstreamMode>,
    #[serde(default)]
    pub auth_providers: Vec<UpstreamAuthProvider>,
    #[serde(default)]
    pub include_account_ids: Vec<Uuid>,
    #[serde(default)]
    pub exclude_account_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingProfile {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub selector: RoutingProfileSelector,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRoutingPolicy {
    pub id: Uuid,
    pub name: String,
    pub family: String,
    #[serde(default)]
    pub exact_models: Vec<String>,
    #[serde(default)]
    pub model_prefixes: Vec<String>,
    #[serde(default)]
    pub fallback_profile_ids: Vec<Uuid>,
    pub enabled: bool,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AccountRoutingTraits {
    pub account_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_provider: Option<UpstreamAuthProvider>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_until: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hard_block_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledRoutingProfile {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub account_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledModelRoutingPolicy {
    pub id: Uuid,
    pub name: String,
    pub family: String,
    #[serde(default)]
    pub exact_models: Vec<String>,
    #[serde(default)]
    pub model_prefixes: Vec<String>,
    #[serde(default)]
    pub fallback_segments: Vec<CompiledRoutingProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledRoutingPlan {
    pub version_id: Uuid,
    pub published_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_reason: Option<String>,
    #[serde(default)]
    pub default_route: Vec<CompiledRoutingProfile>,
    #[serde(default)]
    pub policies: Vec<CompiledModelRoutingPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelRoutingTriggerMode {
    Hybrid,
    ScheduledOnly,
    EventOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRoutingSettings {
    pub enabled: bool,
    pub auto_publish: bool,
    #[serde(default)]
    pub planner_model_chain: Vec<String>,
    pub trigger_mode: ModelRoutingTriggerMode,
    pub kill_switch: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingPlanVersion {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub published_at: DateTime<Utc>,
    pub compiled_plan: CompiledRoutingPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    RoundRobin,
    FillFirst,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingPolicy {
    pub tenant_id: Uuid,
    pub strategy: RoutingStrategy,
    pub max_retries: u32,
    pub stream_max_retries: u32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountHealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountHealth {
    pub account_id: Uuid,
    pub status: AccountHealthStatus,
    pub reason: Option<String>,
    pub updated_at: DateTime<Utc>,
}
