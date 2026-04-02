use crate::model::{ClaudeCodeRoutingSettings, UpstreamErrorTemplateRecord};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::edition::{
    BillingMode, EditionFeatures, ProductEdition, SystemCapabilitiesResponse,
};
pub use crate::error::{ErrorBody, ErrorEnvelope};
pub use crate::runtime_contract::{
    ApiKeyGroupStatus, ApiKeyPolicy, ValidateApiKeyRequest, ValidateApiKeyResponse,
};
pub use crate::snapshot::{
    DataPlaneSnapshot, DataPlaneSnapshotEvent, DataPlaneSnapshotEventType,
    DataPlaneSnapshotEventsResponse,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub account_total: usize,
    pub active_account_total: usize,
    pub window_limit_tokens: u64,
    pub window_used_tokens: u64,
    pub window_reset_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolveUpstreamErrorTemplateRequest {
    pub fingerprint: String,
    pub provider: String,
    pub normalized_status_code: u16,
    pub normalized_upstream_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sanitized_upstream_raw: Option<String>,
    pub target_locale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveUpstreamErrorTemplateResponse {
    pub template: UpstreamErrorTemplateRecord,
    #[serde(default)]
    pub created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCodeRoutingSettingsResponse {
    #[serde(flatten)]
    pub settings: ClaudeCodeRoutingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateClaudeCodeRoutingSettingsRequest {
    pub enabled: bool,
    pub opus_target_model: Option<String>,
    pub sonnet_target_model: Option<String>,
    pub haiku_target_model: Option<String>,
}
