use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::{
    AccountRoutingTraits, AiErrorLearningSettings, BuiltinErrorTemplateRecord,
    CompiledRoutingPlan, OutboundProxyNode, OutboundProxyPoolSettings, UpstreamAccount,
    UpstreamErrorTemplateRecord,
};

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
    #[serde(default)]
    pub ai_error_learning_settings: AiErrorLearningSettings,
    #[serde(default)]
    pub approved_upstream_error_templates: Vec<UpstreamErrorTemplateRecord>,
    #[serde(default)]
    pub builtin_error_templates: Vec<BuiltinErrorTemplateRecord>,
    #[serde(default)]
    pub outbound_proxy_pool_settings: OutboundProxyPoolSettings,
    #[serde(default)]
    pub outbound_proxy_nodes: Vec<OutboundProxyNode>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_error_learning_settings: Option<AiErrorLearningSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_upstream_error_templates: Option<Vec<UpstreamErrorTemplateRecord>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin_error_templates: Option<Vec<BuiltinErrorTemplateRecord>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_proxy_pool_settings: Option<OutboundProxyPoolSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_proxy_nodes: Option<Vec<OutboundProxyNode>>,
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
