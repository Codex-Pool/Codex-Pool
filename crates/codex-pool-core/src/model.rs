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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProxyFailMode {
    #[default]
    StrictProxy,
    AllowDirectFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutboundProxyPoolSettings {
    pub enabled: bool,
    pub fail_mode: ProxyFailMode,
    pub updated_at: DateTime<Utc>,
}

impl Default for OutboundProxyPoolSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            fail_mode: ProxyFailMode::StrictProxy,
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutboundProxyNode {
    pub id: Uuid,
    pub label: String,
    pub proxy_url: String,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
pub struct AiErrorLearningSettings {
    pub enabled: bool,
    pub first_seen_timeout_ms: u64,
    pub review_hit_threshold: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Default for AiErrorLearningSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            first_seen_timeout_ms: 2_000,
            review_hit_threshold: 10,
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LocalizedErrorTemplates {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub en: Option<String>,
    #[serde(default, rename = "zh-CN", skip_serializing_if = "Option::is_none")]
    pub zh_cn: Option<String>,
    #[serde(default, rename = "zh-TW", skip_serializing_if = "Option::is_none")]
    pub zh_tw: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ja: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ru: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpstreamErrorAction {
    ReturnFailure,
    RetrySameAccount,
    RetryCrossAccount,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpstreamErrorRetryScope {
    None,
    SameAccount,
    CrossAccount,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinErrorTemplateKind {
    GatewayError,
    HeuristicUpstream,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpstreamErrorTemplateStatus {
    ProvisionalLive,
    ReviewPending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpstreamErrorTemplateRecord {
    pub id: Uuid,
    pub fingerprint: String,
    pub provider: String,
    pub normalized_status_code: u16,
    pub semantic_error_code: String,
    pub action: UpstreamErrorAction,
    pub retry_scope: UpstreamErrorRetryScope,
    pub status: UpstreamErrorTemplateStatus,
    #[serde(default)]
    pub templates: LocalizedErrorTemplates,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub representative_samples: Vec<String>,
    #[serde(default)]
    pub hit_count: u64,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuiltinErrorTemplateOverrideRecord {
    pub kind: BuiltinErrorTemplateKind,
    pub code: String,
    #[serde(default)]
    pub templates: LocalizedErrorTemplates,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuiltinErrorTemplateRecord {
    pub kind: BuiltinErrorTemplateKind,
    pub code: String,
    #[serde(default)]
    pub templates: LocalizedErrorTemplates,
    #[serde(default)]
    pub default_templates: LocalizedErrorTemplates,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<UpstreamErrorAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_scope: Option<UpstreamErrorRetryScope>,
    #[serde(default)]
    pub is_overridden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

fn localized_templates(
    en: &str,
    zh_cn: &str,
    zh_tw: &str,
    ja: &str,
    ru: &str,
) -> LocalizedErrorTemplates {
    LocalizedErrorTemplates {
        en: Some(en.to_string()),
        zh_cn: Some(zh_cn.to_string()),
        zh_tw: Some(zh_tw.to_string()),
        ja: Some(ja.to_string()),
        ru: Some(ru.to_string()),
    }
}

fn builtin_template_record(
    kind: BuiltinErrorTemplateKind,
    code: &str,
    templates: LocalizedErrorTemplates,
    action: Option<UpstreamErrorAction>,
    retry_scope: Option<UpstreamErrorRetryScope>,
) -> BuiltinErrorTemplateRecord {
    BuiltinErrorTemplateRecord {
        kind,
        code: code.to_string(),
        templates: templates.clone(),
        default_templates: templates,
        action,
        retry_scope,
        is_overridden: false,
        updated_at: None,
    }
}

pub fn default_builtin_error_templates() -> Vec<BuiltinErrorTemplateRecord> {
    let mut templates = vec![
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "invalid_request_body",
            localized_templates(
                "Could not parse the request body.",
                "请求体解析失败。",
                "請求體解析失敗。",
                "リクエスト本文を解析できませんでした。",
                "Не удалось разобрать тело запроса.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "invalid_request_rate_limited",
            localized_templates(
                "Too many invalid requests. Please retry later.",
                "无效请求过多，请稍后再试。",
                "無效請求過多，請稍後再試。",
                "無効なリクエストが多すぎます。しばらくしてから再試行してください。",
                "Слишком много некорректных запросов, попробуйте позже.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "invalid_upstream_url",
            localized_templates(
                "The upstream URL is invalid.",
                "上游地址无效。",
                "上游位址無效。",
                "上流 URL が無効です。",
                "Некорректный upstream URL.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "invalid_websocket_upgrade",
            localized_templates(
                "Invalid WebSocket upgrade request.",
                "无效的 WebSocket 升级请求。",
                "無效的 WebSocket 升級請求。",
                "無効な WebSocket アップグレード要求です。",
                "Некорректный запрос на обновление WebSocket.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "no_upstream_account",
            localized_templates(
                "No upstream accounts are currently available.",
                "当前没有可用的上游账号。",
                "目前沒有可用的上游帳號。",
                "利用可能な上流アカウントがありません。",
                "Сейчас нет доступных upstream-аккаунтов.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "payload_too_large",
            localized_templates(
                "The request body exceeds the server limit.",
                "请求体超过了服务端限制。",
                "請求體超過了伺服器限制。",
                "リクエスト本文がサーバー制限を超えています。",
                "Тело запроса превышает лимит сервера.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "upstream_transport_error",
            localized_templates(
                "The upstream request failed.",
                "上游请求失败。",
                "上游請求失敗。",
                "上流リクエストに失敗しました。",
                "Ошибка запроса к upstream.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "upstream_websocket_connect_error",
            localized_templates(
                "Failed to connect to the upstream WebSocket.",
                "连接上游 WebSocket 失败。",
                "連接上游 WebSocket 失敗。",
                "上流 WebSocket への接続に失敗しました。",
                "Не удалось подключиться к upstream WebSocket.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "websocket_handshake_error",
            localized_templates(
                "The upstream WebSocket handshake failed.",
                "上游 WebSocket 握手失败。",
                "上游 WebSocket 握手失敗。",
                "上流 WebSocket ハンドシェイクに失敗しました。",
                "Ошибка рукопожатия upstream WebSocket.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::GatewayError,
            "websocket_upgrade_required",
            localized_templates(
                "The upstream requires a WebSocket upgrade.",
                "上游要求使用 WebSocket 协议升级。",
                "上游要求使用 WebSocket 協議升級。",
                "上流は WebSocket アップグレードを要求しています。",
                "Upstream требует обновления до WebSocket.",
            ),
            None,
            None,
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::HeuristicUpstream,
            "quota_exhausted",
            localized_templates(
                "The upstream quota is exhausted. Please retry shortly.",
                "上游额度已耗尽，请稍后重试。",
                "上游額度已耗盡，請稍後重試。",
                "上流のクォータが尽きています。しばらくしてから再試行してください。",
                "Квота на стороне апстрима исчерпана. Повторите попытку позже.",
            ),
            Some(UpstreamErrorAction::RetryCrossAccount),
            Some(UpstreamErrorRetryScope::CrossAccount),
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::HeuristicUpstream,
            "unsupported_model",
            localized_templates(
                "The requested model is not available.",
                "请求的模型当前不可用。",
                "請求的模型目前不可用。",
                "要求されたモデルは現在利用できません。",
                "Запрошенная модель сейчас недоступна.",
            ),
            Some(UpstreamErrorAction::ReturnFailure),
            Some(UpstreamErrorRetryScope::None),
        ),
        builtin_template_record(
            BuiltinErrorTemplateKind::HeuristicUpstream,
            "upstream_request_failed",
            localized_templates(
                "The upstream request failed. Please retry later.",
                "上游请求失败，请稍后重试。",
                "上游請求失敗，請稍後重試。",
                "上流リクエストに失敗しました。しばらくしてから再試行してください。",
                "Ошибка запроса к апстриму. Повторите попытку позже.",
            ),
            Some(UpstreamErrorAction::ReturnFailure),
            Some(UpstreamErrorRetryScope::None),
        ),
    ];
    templates.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.code.cmp(&right.code))
    });
    templates
}

pub fn default_builtin_error_template(
    kind: BuiltinErrorTemplateKind,
    code: &str,
) -> Option<BuiltinErrorTemplateRecord> {
    default_builtin_error_templates()
        .into_iter()
        .find(|template| template.kind == kind && template.code == code)
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
