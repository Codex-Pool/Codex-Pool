use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, NaiveDate, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Row, Transaction};
use sqlx_postgres::{PgPool, Postgres};
use uuid::Uuid;

use codex_pool_core::api::CreateApiKeyResponse;
use codex_pool_core::model::ApiKey;

const DEFAULT_TENANT_JWT_TTL_SEC: u64 = 8 * 60 * 60;
const DEFAULT_TENANT_SESSION_COOKIE_NAME: &str = "cp_tenant_session";
const DEFAULT_LOGIN_RATE_LIMIT_WINDOW_SEC: u64 = 300;
const DEFAULT_LOGIN_RATE_LIMIT_MAX_ATTEMPTS: usize = 20;
const CODE_PURPOSE_EMAIL_VERIFY: &str = "email_verify";
const CODE_PURPOSE_PASSWORD_RESET: &str = "password_reset";
const CHECKIN_REWARD_MIN: i64 = 50_000_000;
const CHECKIN_REWARD_MAX: i64 = 150_000_000;
const DEFAULT_BILLING_AUTHORIZATION_TTL_SEC: u64 = 15 * 60;
const MIN_BILLING_AUTHORIZATION_TTL_SEC: u64 = 30;
const MAX_BILLING_AUTHORIZATION_TTL_SEC: u64 = 6 * 60 * 60;

struct InsertCodeParams<'a> {
    tenant_id: Uuid,
    tenant_user_id: Uuid,
    purpose: &'a str,
    code_hash: &'a str,
    expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
}

struct CreditDeltaParams<'a> {
    tenant_id: Uuid,
    api_key_id: Option<Uuid>,
    event_type: &'a str,
    delta_microcredits: i64,
    request_id: Option<String>,
    model: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    meta_json: Option<serde_json::Value>,
    now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct BillingAuthorizationRecord {
    id: Uuid,
    tenant_id: Uuid,
    request_id: String,
    api_key_id: Option<Uuid>,
    model: Option<String>,
    reserved_microcredits: i64,
    captured_microcredits: i64,
    status: String,
    expires_at: DateTime<Utc>,
    meta_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
struct BillingPricingResolved {
    input_price_microcredits: i64,
    cached_input_price_microcredits: i64,
    output_price_microcredits: i64,
    source: String,
}

#[derive(Debug, Clone)]
struct ApiKeyGroupRecord {
    id: Uuid,
    name: String,
    description: Option<String>,
    is_default: bool,
    enabled: bool,
    allow_all_models: bool,
    input_multiplier_ppm: i64,
    cached_input_multiplier_ppm: i64,
    output_multiplier_ppm: i64,
    deleted_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct ApiKeyGroupModelPolicyRecord {
    id: Uuid,
    _group_id: Uuid,
    model_id: String,
    enabled: bool,
    input_multiplier_ppm: i64,
    cached_input_multiplier_ppm: i64,
    output_multiplier_ppm: i64,
    input_price_microcredits: Option<i64>,
    cached_input_price_microcredits: Option<i64>,
    output_price_microcredits: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct ApiKeyGroupResolvedPricing {
    formula: BillingPricingResolved,
    final_pricing: BillingPricingResolved,
    uses_absolute_pricing: bool,
}

const BILLING_MULTIPLIER_PPM_ONE: i64 = 1_000_000;
const DEFAULT_BILLING_SESSION_TTL_SEC: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BillingRequestKind {
    Any,
    Response,
    Compact,
    Chat,
    Unknown,
}

impl BillingRequestKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Response => "response",
            Self::Compact => "compact",
            Self::Chat => "chat",
            Self::Unknown => "unknown",
        }
    }

    fn from_optional(raw: Option<&str>) -> Self {
        match raw.unwrap_or("unknown").trim().to_ascii_lowercase().as_str() {
            "any" => Self::Any,
            "response" => Self::Response,
            "compact" => Self::Compact,
            "chat" => Self::Chat,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BillingPricingRuleScope {
    Request,
    Session,
}

impl BillingPricingRuleScope {
    #[cfg(test)]
    fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Session => "session",
        }
    }

    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "session" => Self::Session,
            _ => Self::Request,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BillingPricingBand {
    Base,
    LongContext,
}

impl BillingPricingBand {
    fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::LongContext => "long_context",
        }
    }

    fn from_optional(raw: Option<&str>) -> Self {
        match raw.unwrap_or("base").trim().to_ascii_lowercase().as_str() {
            "long_context" => Self::LongContext,
            _ => Self::Base,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BillingResolutionPhase {
    Authorize,
    Capture,
}

#[derive(Debug, Clone)]
struct BillingPricingRuleRecord {
    id: Uuid,
    model_pattern: String,
    request_kind: String,
    scope: String,
    threshold_input_tokens: Option<i64>,
    input_multiplier_ppm: i64,
    cached_input_multiplier_ppm: i64,
    output_multiplier_ppm: i64,
}

#[derive(Debug, Clone)]
struct BillingSessionRecord {
    pricing_band: String,
}

#[derive(Debug, Clone)]
struct BillingPricingDecision {
    pricing: BillingPricingResolved,
    band: BillingPricingBand,
    matched_rule_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct TenantPrincipal {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub impersonated_admin_user_id: Option<Uuid>,
    pub impersonation_session_id: Option<Uuid>,
    pub impersonation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TenantClaims {
    sub: String,
    tenant_id: String,
    email: String,
    iat: u64,
    exp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    impersonated_admin_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    impersonation_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    impersonation_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TenantRegisterRequest {
    pub tenant_name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantRegisterResponse {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub requires_email_verification: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TenantVerifyEmailRequest {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TenantLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantLoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantMeResponse {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub impersonated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impersonation_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TenantForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TenantResetPasswordRequest {
    pub email: String,
    pub code: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TenantCreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub ip_allowlist: Vec<String>,
    #[serde(default)]
    pub model_allowlist: Vec<String>,
    #[serde(default)]
    pub group_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TenantPatchApiKeyRequest {
    pub enabled: Option<bool>,
    pub ip_allowlist: Option<Vec<String>>,
    pub model_allowlist: Option<Vec<String>>,
    pub group_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyGroupBindingItem {
    pub id: Uuid,
    pub name: String,
    pub is_default: bool,
    pub enabled: bool,
    pub allow_all_models: bool,
    pub deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantApiKeyRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub ip_allowlist: Vec<String>,
    pub model_allowlist: Vec<String>,
    pub group_id: Uuid,
    pub group: ApiKeyGroupBindingItem,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyGroupModelPolicyItem {
    pub id: Uuid,
    pub model: String,
    pub enabled: bool,
    pub input_multiplier_ppm: i64,
    pub cached_input_multiplier_ppm: i64,
    pub output_multiplier_ppm: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_price_microcredits: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyGroupModelPreviewItem {
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_cached_input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_output_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula_input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula_cached_input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula_output_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_cached_input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_output_price_microcredits: Option<i64>,
    pub uses_absolute_pricing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<ApiKeyGroupModelPolicyItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyGroupCatalogItem {
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_cached_input_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_output_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_price_source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyGroupItem {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_default: bool,
    pub enabled: bool,
    pub allow_all_models: bool,
    pub input_multiplier_ppm: i64,
    pub cached_input_multiplier_ppm: i64,
    pub output_multiplier_ppm: i64,
    pub api_key_count: i64,
    pub model_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    pub policies: Vec<ApiKeyGroupModelPolicyItem>,
    pub models: Vec<ApiKeyGroupModelPreviewItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyGroupAdminListResponse {
    pub groups: Vec<ApiKeyGroupItem>,
    pub catalog: Vec<ApiKeyGroupCatalogItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyGroupUpsertRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_enabled_true")]
    pub enabled: bool,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub allow_all_models: bool,
    pub input_multiplier_ppm: i64,
    pub cached_input_multiplier_ppm: i64,
    pub output_multiplier_ppm: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyGroupModelPolicyUpsertRequest {
    pub group_id: Uuid,
    pub model: String,
    #[serde(default = "default_enabled_true")]
    pub enabled: bool,
    pub input_multiplier_ppm: i64,
    pub cached_input_multiplier_ppm: i64,
    pub output_multiplier_ppm: i64,
    #[serde(default)]
    pub input_price_microcredits: Option<i64>,
    #[serde(default)]
    pub cached_input_price_microcredits: Option<i64>,
    #[serde(default)]
    pub output_price_microcredits: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantCreateApiKeyResponse {
    pub record: TenantApiKeyRecord,
    pub plaintext_key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantCreditBalanceResponse {
    pub tenant_id: Uuid,
    pub balance_microcredits: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantCreditSummaryResponse {
    pub tenant_id: Uuid,
    pub balance_microcredits: i64,
    pub today_consumed_microcredits: i64,
    pub month_consumed_microcredits: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantCreditLedgerItem {
    pub id: Uuid,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub delta_microcredits: i64,
    pub balance_after_microcredits: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_price_microcredits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantCreditLedgerResponse {
    pub items: Vec<TenantCreditLedgerItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantDailyCheckinResponse {
    pub tenant_id: Uuid,
    pub local_date: NaiveDate,
    pub reward_microcredits: i64,
    pub balance_microcredits: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminTenantCreateRequest {
    pub name: String,
    pub status: Option<String>,
    pub plan: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminTenantPatchRequest {
    pub status: Option<String>,
    pub plan: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminTenantItem {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub plan: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminRechargeRequest {
    pub amount_microcredits: i64,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminRechargeResponse {
    pub tenant_id: Uuid,
    pub amount_microcredits: i64,
    pub balance_microcredits: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelPricingUpsertRequest {
    pub model: String,
    pub input_price_microcredits: i64,
    #[serde(default)]
    pub cached_input_price_microcredits: Option<i64>,
    pub output_price_microcredits: i64,
    #[serde(default = "default_enabled_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPricingItem {
    pub id: Uuid,
    pub model: String,
    pub input_price_microcredits: i64,
    pub cached_input_price_microcredits: i64,
    pub output_price_microcredits: i64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[derive(Debug, Clone, Serialize)]
pub struct OpenAiModelCatalogItem {
    pub model_id: String,
    pub owned_by: String,
    pub title: String,
    pub description: Option<String>,
    pub context_window_tokens: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub knowledge_cutoff: Option<String>,
    pub reasoning_token_support: Option<bool>,
    pub input_price_microcredits: Option<i64>,
    pub cached_input_price_microcredits: Option<i64>,
    pub output_price_microcredits: Option<i64>,
    pub pricing_notes: Option<String>,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    pub endpoints: Vec<String>,
    pub source_url: String,
    pub raw_text: Option<String>,
    pub synced_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiModelsSyncResponse {
    pub models_total: usize,
    pub created_or_updated: usize,
    pub deleted_catalog_rows: usize,
    pub cleared_custom_entities: u64,
    pub cleared_billing_rules: u64,
    pub deleted_legacy_pricing_rows: u64,
    pub synced_at: DateTime<Utc>,
}


#[derive(Debug, Clone, Deserialize)]
pub struct BillingPricingRuleUpsertRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub model_pattern: String,
    #[serde(default = "default_request_kind_any")]
    pub request_kind: String,
    #[serde(default = "default_billing_rule_scope_request")]
    pub scope: String,
    #[serde(default)]
    pub threshold_input_tokens: Option<i64>,
    pub input_multiplier_ppm: i64,
    pub cached_input_multiplier_ppm: i64,
    pub output_multiplier_ppm: i64,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_enabled_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingPricingRuleItem {
    pub id: Uuid,
    pub model_pattern: String,
    pub request_kind: String,
    pub scope: String,
    pub threshold_input_tokens: Option<i64>,
    pub input_multiplier_ppm: i64,
    pub cached_input_multiplier_ppm: i64,
    pub output_multiplier_ppm: i64,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminModelEntityUpsertRequest {
    pub model: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminModelEntityItem {
    pub id: Uuid,
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminImpersonateRequest {
    pub tenant_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminImpersonateResponse {
    pub session_id: Uuid,
    pub access_token: String,
    pub expires_in: u64,
    pub tenant_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BillingAuthorizeRequest {
    pub tenant_id: Uuid,
    #[serde(default)]
    pub api_key_id: Option<Uuid>,
    pub request_id: String,
    #[serde(default)]
    pub trace_request_id: Option<String>,
    pub model: String,
    #[serde(default)]
    pub session_key: Option<String>,
    #[serde(default)]
    pub request_kind: Option<String>,
    pub reserved_microcredits: i64,
    #[serde(default)]
    pub ttl_sec: Option<u64>,
    #[serde(default)]
    pub is_stream: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingAuthorizeResponse {
    pub authorization_id: Uuid,
    pub tenant_id: Uuid,
    pub request_id: String,
    pub status: String,
    pub reserved_microcredits: i64,
    pub captured_microcredits: i64,
    pub balance_microcredits: i64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BillingCaptureRequest {
    pub tenant_id: Uuid,
    #[serde(default)]
    pub api_key_id: Option<Uuid>,
    pub request_id: String,
    pub model: String,
    #[serde(default)]
    pub session_key: Option<String>,
    #[serde(default)]
    pub request_kind: Option<String>,
    pub input_tokens: i64,
    #[serde(default)]
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    #[serde(default)]
    pub reasoning_tokens: i64,
    #[serde(default)]
    pub is_stream: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingCaptureResponse {
    pub authorization_id: Uuid,
    pub tenant_id: Uuid,
    pub request_id: String,
    pub status: String,
    pub reserved_microcredits: i64,
    pub captured_microcredits: i64,
    pub charged_microcredits: i64,
    pub balance_microcredits: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BillingPricingRequest {
    pub model: String,
    #[serde(default)]
    pub api_key_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingPricingResponse {
    pub model: String,
    pub input_price_microcredits: i64,
    pub cached_input_price_microcredits: i64,
    pub output_price_microcredits: i64,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BillingReleaseRequest {
    pub tenant_id: Uuid,
    pub request_id: String,
    #[serde(default)]
    pub is_stream: bool,
    #[serde(default)]
    pub release_reason: Option<String>,
    #[serde(default)]
    pub upstream_status_code: Option<u16>,
    #[serde(default)]
    pub upstream_error_code: Option<String>,
    #[serde(default)]
    pub failover_action: Option<String>,
    #[serde(default)]
    pub failover_reason_class: Option<String>,
    #[serde(default)]
    pub recovery_action: Option<String>,
    #[serde(default)]
    pub recovery_outcome: Option<String>,
    #[serde(default)]
    pub cross_account_failover_attempted: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingReleaseResponse {
    pub authorization_id: Uuid,
    pub tenant_id: Uuid,
    pub request_id: String,
    pub status: String,
    pub reserved_microcredits: i64,
    pub captured_microcredits: i64,
    pub released_microcredits: i64,
    pub balance_microcredits: i64,
}

#[derive(Debug, Clone)]
pub struct BillingReconcileRequest {
    pub stale_sec: u64,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Default)]
pub struct BillingReconcileStats {
    pub scanned: u64,
    pub adjusted: u64,
    pub released_authorizations: u64,
    pub adjusted_microcredits_total: i64,
}

#[derive(Debug, Clone)]
pub struct BillingReconcileFactRequest {
    pub tenant_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub request_id: String,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub cached_input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub reasoning_tokens: Option<i64>,
}

#[derive(Debug)]
struct BillingReconcileRuntimeMetrics {
    scanned_total: AtomicU64,
    adjust_total: AtomicU64,
    failed_total: AtomicU64,
    released_total: AtomicU64,
}

static BILLING_RECONCILE_RUNTIME_METRICS: LazyLock<BillingReconcileRuntimeMetrics> =
    LazyLock::new(|| BillingReconcileRuntimeMetrics {
        scanned_total: AtomicU64::new(0),
        adjust_total: AtomicU64::new(0),
        failed_total: AtomicU64::new(0),
        released_total: AtomicU64::new(0),
    });

#[derive(Debug, Clone, Serialize, Default)]
pub struct BillingReconcileRuntimeSnapshot {
    pub billing_reconcile_scanned_total: u64,
    pub billing_reconcile_adjust_total: u64,
    pub billing_reconcile_failed_total: u64,
    pub billing_reconcile_released_total: u64,
}

pub fn billing_reconcile_runtime_snapshot() -> BillingReconcileRuntimeSnapshot {
    BillingReconcileRuntimeSnapshot {
        billing_reconcile_scanned_total: BILLING_RECONCILE_RUNTIME_METRICS
            .scanned_total
            .load(Ordering::Relaxed),
        billing_reconcile_adjust_total: BILLING_RECONCILE_RUNTIME_METRICS
            .adjust_total
            .load(Ordering::Relaxed),
        billing_reconcile_failed_total: BILLING_RECONCILE_RUNTIME_METRICS
            .failed_total
            .load(Ordering::Relaxed),
        billing_reconcile_released_total: BILLING_RECONCILE_RUNTIME_METRICS
            .released_total
            .load(Ordering::Relaxed),
    }
}

pub fn record_billing_reconcile_runtime_stats(stats: &BillingReconcileStats) {
    if stats.scanned > 0 {
        BILLING_RECONCILE_RUNTIME_METRICS
            .scanned_total
            .fetch_add(stats.scanned, Ordering::Relaxed);
    }
    if stats.adjusted > 0 {
        BILLING_RECONCILE_RUNTIME_METRICS
            .adjust_total
            .fetch_add(stats.adjusted, Ordering::Relaxed);
    }
    if stats.released_authorizations > 0 {
        BILLING_RECONCILE_RUNTIME_METRICS
            .released_total
            .fetch_add(stats.released_authorizations, Ordering::Relaxed);
    }
}

pub fn record_billing_reconcile_runtime_failed() {
    BILLING_RECONCILE_RUNTIME_METRICS
        .failed_total
        .fetch_add(1, Ordering::Relaxed);
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingPrecheckResponse {
    pub tenant_id: Uuid,
    pub tenant_status: String,
    pub balance_microcredits: i64,
    pub ok: bool,
}

#[derive(Debug, Clone)]
pub struct AuditLogWriteRequest {
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub action: String,
    pub reason: Option<String>,
    pub request_ip: Option<String>,
    pub user_agent: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub payload_json: serde_json::Value,
    pub result_status: String,
}

#[derive(Debug, Clone)]
pub struct AuditLogListQuery {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub limit: usize,
    pub tenant_id: Option<Uuid>,
    pub actor_type: Option<String>,
    pub actor_id: Option<Uuid>,
    pub action: Option<String>,
    pub result_status: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogListItem {
    pub id: Uuid,
    pub actor_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<Uuid>,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    pub payload_json: serde_json::Value,
    pub result_status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogListResponse {
    pub items: Vec<AuditLogListItem>,
}

fn default_enabled_true() -> bool {
    true
}

fn default_request_kind_any() -> String {
    "any".to_string()
}

fn default_billing_rule_scope_request() -> String {
    "request".to_string()
}

pub struct TenantAuthService {
    pool: PgPool,
    token_ttl_sec: u64,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    session_cookie_name: String,
    session_cookie_secure: bool,
    expose_debug_code: bool,
    login_rate_limit_window: StdDuration,
    login_rate_limit_max_attempts: usize,
    login_attempts: Arc<tokio::sync::Mutex<HashMap<String, Vec<Instant>>>>,
}
