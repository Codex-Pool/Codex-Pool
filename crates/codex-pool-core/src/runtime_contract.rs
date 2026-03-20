use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
