use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use chrono::Utc;
use codex_pool_core::api::ProductEdition;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DataPlaneConfig {
    pub listen_addr: SocketAddr,
    pub routing_strategy: RoutingStrategy,
    pub upstream_accounts: Vec<UpstreamAccount>,
    pub account_ejection_ttl_sec: u64,
    pub enable_request_failover: bool,
    pub same_account_quick_retry_max: u32,
    pub request_failover_wait_ms: u64,
    pub retry_poll_interval_ms: u64,
    pub sticky_prefer_non_conflicting: bool,
    pub shared_routing_cache_enabled: bool,
    pub enable_metered_stream_billing: bool,
    pub billing_authorize_required_for_stream: bool,
    pub stream_billing_reserve_microcredits: i64,
    pub billing_dynamic_preauth_enabled: bool,
    pub billing_preauth_expected_output_tokens: i64,
    pub billing_preauth_safety_factor: f64,
    pub billing_preauth_min_microcredits: i64,
    pub billing_preauth_max_microcredits: i64,
    pub billing_preauth_unit_price_microcredits: i64,
    pub stream_billing_drain_timeout_ms: u64,
    pub billing_capture_retry_max: u32,
    pub billing_capture_retry_backoff_ms: u64,
    pub redis_url: Option<String>,
    pub auth_validate_url: Option<String>,
    pub auth_validate_cache_ttl_sec: u64,
    pub auth_validate_negative_cache_ttl_sec: u64,
    pub auth_fail_open: bool,
    pub enable_internal_debug_routes: bool,
}

const DEFAULT_CONFIG_FILE_PATH: &str = "config.toml";
const GLOBAL_CONFIG_FILE_ENV: &str = "CODEX_POOL_CONFIG_FILE";
const DATA_PLANE_CONFIG_FILE_ENV: &str = "DATA_PLANE_CONFIG_FILE";

const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:8091";
const DEFAULT_REQUEST_LOG_STREAM: &str = "stream.request_log";
const DEFAULT_ACCOUNT_EJECTION_TTL_SEC: u64 = 30;
const MAX_ACCOUNT_EJECTION_TTL_SEC: u64 = 600;
const DEFAULT_ENABLE_REQUEST_FAILOVER: bool = true;
const DEFAULT_SAME_ACCOUNT_QUICK_RETRY_MAX: u32 = 1;
const MAX_SAME_ACCOUNT_QUICK_RETRY_MAX: u32 = 5;
const DEFAULT_REQUEST_FAILOVER_WAIT_MS: u64 = 2_000;
const MIN_REQUEST_FAILOVER_WAIT_MS: u64 = 100;
const MAX_REQUEST_FAILOVER_WAIT_MS: u64 = 30_000;
const DEFAULT_RETRY_POLL_INTERVAL_MS: u64 = 100;
const MIN_RETRY_POLL_INTERVAL_MS: u64 = 10;
const MAX_RETRY_POLL_INTERVAL_MS: u64 = 1_000;
const DEFAULT_STICKY_PREFER_NON_CONFLICTING: bool = true;
const DEFAULT_SHARED_ROUTING_CACHE_ENABLED: bool = true;
const DEFAULT_ENABLE_METERED_STREAM_BILLING: bool = true;
const DEFAULT_BILLING_AUTHORIZE_REQUIRED_FOR_STREAM: bool = true;
const DEFAULT_STREAM_BILLING_RESERVE_MICROCREDITS: i64 = 2_000_000;
const MIN_STREAM_BILLING_RESERVE_MICROCREDITS: i64 = 1_000;
const MAX_STREAM_BILLING_RESERVE_MICROCREDITS: i64 = 1_000_000_000_000;
const DEFAULT_BILLING_DYNAMIC_PREAUTH_ENABLED: bool = true;
const DEFAULT_BILLING_PREAUTH_EXPECTED_OUTPUT_TOKENS: i64 = 256;
const MIN_BILLING_PREAUTH_EXPECTED_OUTPUT_TOKENS: i64 = 0;
const MAX_BILLING_PREAUTH_EXPECTED_OUTPUT_TOKENS: i64 = 1_000_000;
const DEFAULT_BILLING_PREAUTH_SAFETY_FACTOR: f64 = 1.3;
const MIN_BILLING_PREAUTH_SAFETY_FACTOR: f64 = 1.0;
const MAX_BILLING_PREAUTH_SAFETY_FACTOR: f64 = 10.0;
const DEFAULT_BILLING_PREAUTH_MIN_MICROCREDITS: i64 = 1_000;
const MIN_BILLING_PREAUTH_MIN_MICROCREDITS: i64 = 1;
const MAX_BILLING_PREAUTH_MIN_MICROCREDITS: i64 = 1_000_000_000_000;
const DEFAULT_BILLING_PREAUTH_MAX_MICROCREDITS: i64 = 1_000_000_000_000;
const MIN_BILLING_PREAUTH_MAX_MICROCREDITS: i64 = 1_000;
const MAX_BILLING_PREAUTH_MAX_MICROCREDITS: i64 = 1_000_000_000_000;
const DEFAULT_BILLING_PREAUTH_UNIT_PRICE_MICROCREDITS: i64 = 10_000;
const MIN_BILLING_PREAUTH_UNIT_PRICE_MICROCREDITS: i64 = 1;
const MAX_BILLING_PREAUTH_UNIT_PRICE_MICROCREDITS: i64 = 1_000_000_000;
const DEFAULT_STREAM_BILLING_DRAIN_TIMEOUT_MS: u64 = 5_000;
const MIN_STREAM_BILLING_DRAIN_TIMEOUT_MS: u64 = 100;
const MAX_STREAM_BILLING_DRAIN_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_BILLING_CAPTURE_RETRY_MAX: u32 = 3;
const MIN_BILLING_CAPTURE_RETRY_MAX: u32 = 1;
const MAX_BILLING_CAPTURE_RETRY_MAX: u32 = 10;
const DEFAULT_BILLING_CAPTURE_RETRY_BACKOFF_MS: u64 = 200;
const MIN_BILLING_CAPTURE_RETRY_BACKOFF_MS: u64 = 10;
const MAX_BILLING_CAPTURE_RETRY_BACKOFF_MS: u64 = 5_000;
const DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC: u64 = 30;
const DEFAULT_AUTH_VALIDATE_NEGATIVE_CACHE_TTL_SEC: u64 = 5;
const CODEX_POOL_EDITION_ENV: &str = "CODEX_POOL_EDITION";

const ALLOWED_API_KEYS_ENV: &str = "DATA_PLANE_ALLOWED_API_KEYS";

impl DataPlaneConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let config_path = resolve_config_path();
        let file_config = load_file_config(&config_path)?;
        apply_process_env_defaults(&file_config);
        let edition = ProductEdition::from_env_var(CODEX_POOL_EDITION_ENV);

        let listen_addr = parse_listen_addr(
            std::env::var("DATA_PLANE_LISTEN")
                .ok()
                .as_deref()
                .or(file_config.listen.as_deref()),
        )?;
        let upstream_accounts = parse_upstream_accounts(
            std::env::var("UPSTREAM_ACCOUNTS_JSON").ok().as_deref(),
            file_config.upstream_accounts,
        )?;
        let stream_billing_reserve_raw = std::env::var("BILLING_PREAUTH_FALLBACK_MICROCREDITS")
            .ok()
            .or_else(|| std::env::var("STREAM_BILLING_RESERVE_MICROCREDITS").ok());
        let stream_billing_reserve_fallback = file_config
            .billing_preauth_fallback_microcredits
            .or(file_config.stream_billing_reserve_microcredits);

        let mut config = Self {
            listen_addr,
            routing_strategy: parse_routing_strategy(
                std::env::var("ROUTING_STRATEGY")
                    .ok()
                    .as_deref()
                    .or(file_config.routing_strategy.as_deref()),
            )?,
            upstream_accounts,
            account_ejection_ttl_sec: parse_account_ejection_ttl_sec(
                std::env::var("ACCOUNT_EJECTION_TTL_SEC").ok().as_deref(),
                file_config.account_ejection_ttl_sec,
            ),
            enable_request_failover: env_flag_override("ENABLE_REQUEST_FAILOVER")
                .or(file_config.enable_request_failover)
                .unwrap_or(DEFAULT_ENABLE_REQUEST_FAILOVER),
            same_account_quick_retry_max: parse_same_account_quick_retry_max(
                std::env::var("SAME_ACCOUNT_QUICK_RETRY_MAX")
                    .ok()
                    .as_deref(),
                file_config.same_account_quick_retry_max,
            ),
            request_failover_wait_ms: parse_request_failover_wait_ms(
                std::env::var("REQUEST_FAILOVER_WAIT_MS").ok().as_deref(),
                file_config.request_failover_wait_ms,
            ),
            retry_poll_interval_ms: parse_retry_poll_interval_ms(
                std::env::var("RETRY_POLL_INTERVAL_MS").ok().as_deref(),
                file_config.retry_poll_interval_ms,
            ),
            sticky_prefer_non_conflicting: env_flag_override("STICKY_PREFER_NON_CONFLICTING")
                .or(file_config.sticky_prefer_non_conflicting)
                .unwrap_or(DEFAULT_STICKY_PREFER_NON_CONFLICTING),
            shared_routing_cache_enabled: env_flag_override("SHARED_ROUTING_CACHE_ENABLED")
                .or(file_config.shared_routing_cache_enabled)
                .unwrap_or(DEFAULT_SHARED_ROUTING_CACHE_ENABLED),
            enable_metered_stream_billing: env_flag_override("ENABLE_METERED_STREAM_BILLING")
                .or(file_config.enable_metered_stream_billing)
                .unwrap_or(DEFAULT_ENABLE_METERED_STREAM_BILLING),
            billing_authorize_required_for_stream: env_flag_override(
                "BILLING_AUTHORIZE_REQUIRED_FOR_STREAM",
            )
            .or(file_config.billing_authorize_required_for_stream)
            .unwrap_or(DEFAULT_BILLING_AUTHORIZE_REQUIRED_FOR_STREAM),
            stream_billing_reserve_microcredits: parse_stream_billing_reserve_microcredits(
                stream_billing_reserve_raw.as_deref(),
                stream_billing_reserve_fallback,
            ),
            billing_dynamic_preauth_enabled: env_flag_override("BILLING_DYNAMIC_PREAUTH_ENABLED")
                .or(file_config.billing_dynamic_preauth_enabled)
                .unwrap_or(DEFAULT_BILLING_DYNAMIC_PREAUTH_ENABLED),
            billing_preauth_expected_output_tokens: parse_billing_preauth_expected_output_tokens(
                std::env::var("BILLING_PREAUTH_EXPECTED_OUTPUT_TOKENS")
                    .ok()
                    .as_deref(),
                file_config.billing_preauth_expected_output_tokens,
            ),
            billing_preauth_safety_factor: parse_billing_preauth_safety_factor(
                std::env::var("BILLING_PREAUTH_SAFETY_FACTOR")
                    .ok()
                    .as_deref(),
                file_config.billing_preauth_safety_factor,
            ),
            billing_preauth_min_microcredits: parse_billing_preauth_min_microcredits(
                std::env::var("BILLING_PREAUTH_MIN_MICROCREDITS")
                    .ok()
                    .as_deref(),
                file_config.billing_preauth_min_microcredits,
            ),
            billing_preauth_max_microcredits: parse_billing_preauth_max_microcredits(
                std::env::var("BILLING_PREAUTH_MAX_MICROCREDITS")
                    .ok()
                    .as_deref(),
                file_config.billing_preauth_max_microcredits,
            ),
            billing_preauth_unit_price_microcredits: parse_billing_preauth_unit_price_microcredits(
                std::env::var("BILLING_PREAUTH_UNIT_PRICE_MICROCREDITS")
                    .ok()
                    .as_deref(),
                file_config.billing_preauth_unit_price_microcredits,
            ),
            stream_billing_drain_timeout_ms: parse_stream_billing_drain_timeout_ms(
                std::env::var("STREAM_BILLING_DRAIN_TIMEOUT_MS")
                    .ok()
                    .as_deref(),
                file_config.stream_billing_drain_timeout_ms,
            ),
            billing_capture_retry_max: parse_billing_capture_retry_max(
                std::env::var("BILLING_CAPTURE_RETRY_MAX").ok().as_deref(),
                file_config.billing_capture_retry_max,
            ),
            billing_capture_retry_backoff_ms: parse_billing_capture_retry_backoff_ms(
                std::env::var("BILLING_CAPTURE_RETRY_BACKOFF_MS")
                    .ok()
                    .as_deref(),
                file_config.billing_capture_retry_backoff_ms,
            ),
            redis_url: std::env::var("REDIS_URL").ok().or(file_config.redis_url),
            auth_validate_url: std::env::var("AUTH_VALIDATE_URL")
                .ok()
                .or(file_config.auth_validate_url),
            auth_validate_cache_ttl_sec: parse_u64_env_with_fallback(
                "AUTH_VALIDATE_CACHE_TTL_SEC",
                file_config.auth_validate_cache_ttl_sec,
                DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
            ),
            auth_validate_negative_cache_ttl_sec: parse_auth_validate_negative_cache_ttl_sec(
                std::env::var("AUTH_VALIDATE_NEGATIVE_CACHE_TTL_SEC")
                    .ok()
                    .as_deref(),
                file_config.auth_validate_negative_cache_ttl_sec,
            ),
            auth_fail_open: env_flag_override("AUTH_FAIL_OPEN")
                .or(file_config.auth_fail_open)
                .unwrap_or(false),
            enable_internal_debug_routes: env_flag_override("ENABLE_INTERNAL_DEBUG_ROUTES")
                .or(file_config.enable_internal_debug_routes)
                .unwrap_or(false),
        };

        apply_edition_overrides(&mut config, edition);

        Ok(config)
    }

    pub fn request_log_stream(&self) -> String {
        std::env::var("REQUEST_LOG_STREAM")
            .unwrap_or_else(|_| DEFAULT_REQUEST_LOG_STREAM.to_string())
    }
}

fn apply_edition_overrides(config: &mut DataPlaneConfig, edition: ProductEdition) {
    if matches!(edition, ProductEdition::Business) {
        return;
    }

    config.enable_metered_stream_billing = false;
    config.billing_authorize_required_for_stream = false;
    config.billing_dynamic_preauth_enabled = false;
}

#[derive(Debug, Default, Deserialize)]
struct DataPlaneTomlRoot {
    #[serde(default)]
    data_plane: DataPlaneTomlConfig,
}

#[derive(Debug, Default, Deserialize)]
struct DataPlaneTomlConfig {
    #[serde(default)]
    listen: Option<String>,
    #[serde(default)]
    routing_strategy: Option<String>,
    #[serde(default)]
    upstream_accounts: Option<Vec<UpstreamAccountSeed>>,
    #[serde(default)]
    account_ejection_ttl_sec: Option<u64>,
    #[serde(default)]
    enable_request_failover: Option<bool>,
    #[serde(default)]
    same_account_quick_retry_max: Option<u32>,
    #[serde(default)]
    request_failover_wait_ms: Option<u64>,
    #[serde(default)]
    retry_poll_interval_ms: Option<u64>,
    #[serde(default)]
    sticky_prefer_non_conflicting: Option<bool>,
    #[serde(default)]
    shared_routing_cache_enabled: Option<bool>,
    #[serde(default)]
    enable_metered_stream_billing: Option<bool>,
    #[serde(default)]
    billing_authorize_required_for_stream: Option<bool>,
    #[serde(default)]
    stream_billing_reserve_microcredits: Option<i64>,
    #[serde(default)]
    billing_preauth_fallback_microcredits: Option<i64>,
    #[serde(default)]
    billing_dynamic_preauth_enabled: Option<bool>,
    #[serde(default)]
    billing_preauth_expected_output_tokens: Option<i64>,
    #[serde(default)]
    billing_preauth_safety_factor: Option<f64>,
    #[serde(default)]
    billing_preauth_min_microcredits: Option<i64>,
    #[serde(default)]
    billing_preauth_max_microcredits: Option<i64>,
    #[serde(default)]
    billing_preauth_unit_price_microcredits: Option<i64>,
    #[serde(default)]
    stream_billing_drain_timeout_ms: Option<u64>,
    #[serde(default)]
    billing_capture_retry_max: Option<u32>,
    #[serde(default)]
    billing_capture_retry_backoff_ms: Option<u64>,
    #[serde(default)]
    redis_url: Option<String>,
    #[serde(default)]
    auth_validate_url: Option<String>,
    #[serde(default)]
    auth_validate_cache_ttl_sec: Option<u64>,
    #[serde(default)]
    auth_validate_negative_cache_ttl_sec: Option<u64>,
    #[serde(default)]
    max_request_body_bytes: Option<u64>,
    #[serde(default)]
    auth_fail_open: Option<bool>,
    #[serde(default)]
    enable_internal_debug_routes: Option<bool>,
    #[serde(default)]
    request_log_stream: Option<String>,
    #[serde(default)]
    control_plane_base_url: Option<String>,
    #[serde(default)]
    snapshot_poll_interval_ms: Option<u64>,
    #[serde(default)]
    allowed_api_keys: Option<Vec<String>>,
}

fn resolve_config_path() -> PathBuf {
    std::env::var(DATA_PLANE_CONFIG_FILE_ENV)
        .ok()
        .or_else(|| std::env::var(GLOBAL_CONFIG_FILE_ENV).ok())
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE_PATH))
}

fn load_file_config(path: &Path) -> anyhow::Result<DataPlaneTomlConfig> {
    if !path.exists() {
        return Ok(DataPlaneTomlConfig::default());
    }

    let raw = std::fs::read_to_string(path)?;
    let parsed: DataPlaneTomlRoot = toml::from_str(&raw)?;
    Ok(parsed.data_plane)
}

fn apply_process_env_defaults(file_config: &DataPlaneTomlConfig) {
    set_env_if_absent("REQUEST_LOG_STREAM", file_config.request_log_stream.clone());
    set_env_if_absent(
        "CONTROL_PLANE_BASE_URL",
        file_config.control_plane_base_url.clone(),
    );
    set_env_if_absent(
        "SNAPSHOT_POLL_INTERVAL_MS",
        file_config
            .snapshot_poll_interval_ms
            .map(|value| value.to_string()),
    );
    set_env_if_absent(
        "DATA_PLANE_MAX_REQUEST_BODY_BYTES",
        file_config
            .max_request_body_bytes
            .map(|value| value.to_string()),
    );
    if std::env::var_os(ALLOWED_API_KEYS_ENV).is_none() {
        if let Some(keys) = file_config.allowed_api_keys.as_ref() {
            let serialized = keys
                .iter()
                .map(|key| key.trim())
                .filter(|key| !key.is_empty())
                .collect::<Vec<_>>()
                .join(",");
            if !serialized.is_empty() {
                std::env::set_var(ALLOWED_API_KEYS_ENV, serialized);
            }
        }
    }
}

fn parse_routing_strategy(raw: Option<&str>) -> anyhow::Result<RoutingStrategy> {
    match raw.map(|value| value.trim().to_ascii_lowercase()) {
        None => Ok(RoutingStrategy::RoundRobin),
        Some(value) if value.is_empty() => Ok(RoutingStrategy::RoundRobin),
        Some(value) if value == "round_robin" => Ok(RoutingStrategy::RoundRobin),
        Some(value) if value == "fill_first" => Ok(RoutingStrategy::FillFirst),
        Some(value) => anyhow::bail!("invalid routing strategy: {value}"),
    }
}

fn parse_account_ejection_ttl_sec(raw: Option<&str>, fallback: Option<u64>) -> u64 {
    raw.and_then(|value| value.parse::<u64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_ACCOUNT_EJECTION_TTL_SEC)
        .min(MAX_ACCOUNT_EJECTION_TTL_SEC)
}

fn parse_auth_validate_negative_cache_ttl_sec(raw: Option<&str>, fallback: Option<u64>) -> u64 {
    raw.and_then(|value| value.parse::<u64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_AUTH_VALIDATE_NEGATIVE_CACHE_TTL_SEC)
}

fn parse_same_account_quick_retry_max(raw: Option<&str>, fallback: Option<u32>) -> u32 {
    raw.and_then(|value| value.parse::<u32>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_SAME_ACCOUNT_QUICK_RETRY_MAX)
        .min(MAX_SAME_ACCOUNT_QUICK_RETRY_MAX)
}

fn parse_request_failover_wait_ms(raw: Option<&str>, fallback: Option<u64>) -> u64 {
    raw.and_then(|value| value.parse::<u64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_REQUEST_FAILOVER_WAIT_MS)
        .clamp(MIN_REQUEST_FAILOVER_WAIT_MS, MAX_REQUEST_FAILOVER_WAIT_MS)
}

fn parse_retry_poll_interval_ms(raw: Option<&str>, fallback: Option<u64>) -> u64 {
    raw.and_then(|value| value.parse::<u64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_RETRY_POLL_INTERVAL_MS)
        .clamp(MIN_RETRY_POLL_INTERVAL_MS, MAX_RETRY_POLL_INTERVAL_MS)
}

fn parse_stream_billing_reserve_microcredits(raw: Option<&str>, fallback: Option<i64>) -> i64 {
    raw.and_then(|value| value.parse::<i64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_STREAM_BILLING_RESERVE_MICROCREDITS)
        .clamp(
            MIN_STREAM_BILLING_RESERVE_MICROCREDITS,
            MAX_STREAM_BILLING_RESERVE_MICROCREDITS,
        )
}

fn parse_billing_preauth_expected_output_tokens(raw: Option<&str>, fallback: Option<i64>) -> i64 {
    raw.and_then(|value| value.parse::<i64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_BILLING_PREAUTH_EXPECTED_OUTPUT_TOKENS)
        .clamp(
            MIN_BILLING_PREAUTH_EXPECTED_OUTPUT_TOKENS,
            MAX_BILLING_PREAUTH_EXPECTED_OUTPUT_TOKENS,
        )
}

fn parse_billing_preauth_safety_factor(raw: Option<&str>, fallback: Option<f64>) -> f64 {
    raw.and_then(|value| value.parse::<f64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_BILLING_PREAUTH_SAFETY_FACTOR)
        .clamp(
            MIN_BILLING_PREAUTH_SAFETY_FACTOR,
            MAX_BILLING_PREAUTH_SAFETY_FACTOR,
        )
}

fn parse_billing_preauth_min_microcredits(raw: Option<&str>, fallback: Option<i64>) -> i64 {
    raw.and_then(|value| value.parse::<i64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_BILLING_PREAUTH_MIN_MICROCREDITS)
        .clamp(
            MIN_BILLING_PREAUTH_MIN_MICROCREDITS,
            MAX_BILLING_PREAUTH_MIN_MICROCREDITS,
        )
}

fn parse_billing_preauth_max_microcredits(raw: Option<&str>, fallback: Option<i64>) -> i64 {
    raw.and_then(|value| value.parse::<i64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_BILLING_PREAUTH_MAX_MICROCREDITS)
        .clamp(
            MIN_BILLING_PREAUTH_MAX_MICROCREDITS,
            MAX_BILLING_PREAUTH_MAX_MICROCREDITS,
        )
}

fn parse_billing_preauth_unit_price_microcredits(raw: Option<&str>, fallback: Option<i64>) -> i64 {
    raw.and_then(|value| value.parse::<i64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_BILLING_PREAUTH_UNIT_PRICE_MICROCREDITS)
        .clamp(
            MIN_BILLING_PREAUTH_UNIT_PRICE_MICROCREDITS,
            MAX_BILLING_PREAUTH_UNIT_PRICE_MICROCREDITS,
        )
}

fn parse_stream_billing_drain_timeout_ms(raw: Option<&str>, fallback: Option<u64>) -> u64 {
    raw.and_then(|value| value.parse::<u64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_STREAM_BILLING_DRAIN_TIMEOUT_MS)
        .clamp(
            MIN_STREAM_BILLING_DRAIN_TIMEOUT_MS,
            MAX_STREAM_BILLING_DRAIN_TIMEOUT_MS,
        )
}

fn parse_billing_capture_retry_max(raw: Option<&str>, fallback: Option<u32>) -> u32 {
    raw.and_then(|value| value.parse::<u32>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_BILLING_CAPTURE_RETRY_MAX)
        .clamp(MIN_BILLING_CAPTURE_RETRY_MAX, MAX_BILLING_CAPTURE_RETRY_MAX)
}

fn parse_billing_capture_retry_backoff_ms(raw: Option<&str>, fallback: Option<u64>) -> u64 {
    raw.and_then(|value| value.parse::<u64>().ok())
        .or(fallback)
        .unwrap_or(DEFAULT_BILLING_CAPTURE_RETRY_BACKOFF_MS)
        .clamp(
            MIN_BILLING_CAPTURE_RETRY_BACKOFF_MS,
            MAX_BILLING_CAPTURE_RETRY_BACKOFF_MS,
        )
}

fn parse_u64_env_with_fallback(key: &str, fallback: Option<u64>, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .or(fallback)
        .unwrap_or(default)
}

fn parse_listen_addr(raw: Option<&str>) -> anyhow::Result<SocketAddr> {
    Ok(raw.unwrap_or(DEFAULT_LISTEN_ADDR).parse()?)
}

fn env_flag_override(key: &str) -> Option<bool> {
    std::env::var(key).ok().map(|raw| {
        matches!(
            raw.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn set_env_if_absent(key: &str, value: Option<String>) {
    if std::env::var_os(key).is_some() {
        return;
    }

    if let Some(value) = value.filter(|item| !item.trim().is_empty()) {
        std::env::set_var(key, value);
    }
}

#[derive(Debug, Deserialize)]
struct UpstreamAccountSeed {
    label: String,
    mode: codex_pool_core::model::UpstreamMode,
    base_url: String,
    bearer_token: String,
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default = "default_priority")]
    priority: i32,
}

fn default_enabled() -> bool {
    true
}

fn default_priority() -> i32 {
    100
}

fn parse_upstream_accounts(
    env_raw: Option<&str>,
    fallback: Option<Vec<UpstreamAccountSeed>>,
) -> anyhow::Result<Vec<UpstreamAccount>> {
    let seeds = if let Some(raw) = env_raw {
        serde_json::from_str::<Vec<UpstreamAccountSeed>>(raw)?
    } else {
        fallback.unwrap_or_default()
    };

    Ok(seeds
        .into_iter()
        .map(|seed| UpstreamAccount {
            id: Uuid::new_v4(),
            label: seed.label,
            mode: seed.mode,
            base_url: seed.base_url,
            bearer_token: seed.bearer_token,
            chatgpt_account_id: seed.chatgpt_account_id,
            enabled: seed.enabled,
            priority: seed.priority,
            created_at: Utc::now(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}.toml", std::process::id()))
    }

    #[test]
    fn parses_accounts_from_env_json() {
        let accounts = parse_upstream_accounts(
            Some(
                r#"[{"label":"a","mode":"open_ai_api_key","base_url":"https://api.openai.com/v1","bearer_token":"tok"}]"#,
            ),
            None,
        )
        .expect("must parse");

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].label, "a");
    }

    #[test]
    fn parses_internal_debug_routes_flag() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        std::env::set_var("ENABLE_INTERNAL_DEBUG_ROUTES", "true");
        assert_eq!(
            env_flag_override("ENABLE_INTERNAL_DEBUG_ROUTES"),
            Some(true)
        );

        std::env::set_var("ENABLE_INTERNAL_DEBUG_ROUTES", "0");
        assert_eq!(
            env_flag_override("ENABLE_INTERNAL_DEBUG_ROUTES"),
            Some(false)
        );

        std::env::remove_var("ENABLE_INTERNAL_DEBUG_ROUTES");
        assert_eq!(env_flag_override("ENABLE_INTERNAL_DEBUG_ROUTES"), None);
    }

    #[test]
    fn parses_auth_fail_open_flag() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        std::env::set_var("AUTH_FAIL_OPEN", "true");
        assert_eq!(env_flag_override("AUTH_FAIL_OPEN"), Some(true));

        std::env::set_var("AUTH_FAIL_OPEN", "0");
        assert_eq!(env_flag_override("AUTH_FAIL_OPEN"), Some(false));

        std::env::remove_var("AUTH_FAIL_OPEN");
        assert_eq!(env_flag_override("AUTH_FAIL_OPEN"), None);
    }

    #[test]
    fn account_ejection_ttl_defaults_when_missing() {
        assert_eq!(parse_account_ejection_ttl_sec(None, None), 30);
    }

    #[test]
    fn account_ejection_ttl_reads_env_value() {
        assert_eq!(parse_account_ejection_ttl_sec(Some("45"), None), 45);
    }

    #[test]
    fn account_ejection_ttl_is_clamped_to_max() {
        assert_eq!(parse_account_ejection_ttl_sec(Some("1200"), None), 600);
    }

    #[test]
    fn auth_validate_negative_cache_ttl_defaults_when_missing() {
        assert_eq!(parse_auth_validate_negative_cache_ttl_sec(None, None), 5);
    }

    #[test]
    fn auth_validate_negative_cache_ttl_reads_env_value() {
        assert_eq!(
            parse_auth_validate_negative_cache_ttl_sec(Some("12"), None),
            12
        );
    }

    #[test]
    fn auth_validate_negative_cache_ttl_accepts_zero_to_disable() {
        assert_eq!(
            parse_auth_validate_negative_cache_ttl_sec(Some("0"), None),
            0
        );
    }

    #[test]
    fn parses_config_from_toml_and_sets_runtime_env_defaults() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let path = unique_temp_path("data-plane-config");
        std::fs::write(
            &path,
            r#"
[data_plane]
listen = "127.0.0.1:18080"
routing_strategy = "fill_first"
request_log_stream = "stream.toml"
control_plane_base_url = "http://127.0.0.1:18090"
snapshot_poll_interval_ms = 2200
max_request_body_bytes = 8192
allowed_api_keys = ["tk-a", "tk-b"]
"#,
        )
        .expect("write toml");

        std::env::set_var("DATA_PLANE_CONFIG_FILE", path.display().to_string());
        std::env::remove_var("REQUEST_LOG_STREAM");
        std::env::remove_var("CONTROL_PLANE_BASE_URL");
        std::env::remove_var("SNAPSHOT_POLL_INTERVAL_MS");
        std::env::remove_var("DATA_PLANE_MAX_REQUEST_BODY_BYTES");
        std::env::remove_var(ALLOWED_API_KEYS_ENV);

        let cfg = DataPlaneConfig::from_env().expect("load config");

        std::env::remove_var("DATA_PLANE_CONFIG_FILE");
        std::fs::remove_file(path).expect("cleanup toml");

        assert_eq!(cfg.listen_addr, "127.0.0.1:18080".parse().unwrap());
        assert_eq!(cfg.routing_strategy, RoutingStrategy::FillFirst);
        assert_eq!(
            std::env::var("REQUEST_LOG_STREAM").as_deref(),
            Ok("stream.toml")
        );
        assert_eq!(
            std::env::var("CONTROL_PLANE_BASE_URL").as_deref(),
            Ok("http://127.0.0.1:18090")
        );
        assert_eq!(
            std::env::var("SNAPSHOT_POLL_INTERVAL_MS").as_deref(),
            Ok("2200")
        );
        assert_eq!(
            std::env::var("DATA_PLANE_MAX_REQUEST_BODY_BYTES").as_deref(),
            Ok("8192")
        );
        assert_eq!(
            std::env::var(ALLOWED_API_KEYS_ENV).as_deref(),
            Ok("tk-a,tk-b")
        );

        std::env::remove_var("REQUEST_LOG_STREAM");
        std::env::remove_var("CONTROL_PLANE_BASE_URL");
        std::env::remove_var("SNAPSHOT_POLL_INTERVAL_MS");
        std::env::remove_var("DATA_PLANE_MAX_REQUEST_BODY_BYTES");
        std::env::remove_var(ALLOWED_API_KEYS_ENV);
    }

    #[test]
    fn env_has_higher_priority_than_toml() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let path = unique_temp_path("data-plane-config-priority");
        std::fs::write(
            &path,
            r#"
[data_plane]
listen = "127.0.0.1:18080"
request_log_stream = "stream.toml"
"#,
        )
        .expect("write toml");

        std::env::set_var("DATA_PLANE_CONFIG_FILE", path.display().to_string());
        std::env::set_var("DATA_PLANE_LISTEN", "127.0.0.1:28080");

        let cfg = DataPlaneConfig::from_env().expect("load config");

        std::env::remove_var("DATA_PLANE_CONFIG_FILE");
        std::env::remove_var("DATA_PLANE_LISTEN");
        assert_eq!(cfg.listen_addr, "127.0.0.1:28080".parse().unwrap());
        std::fs::remove_file(path).expect("cleanup toml");
    }

    #[test]
    fn uses_default_snapshot_poll_interval_when_unset() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        std::env::remove_var("SNAPSHOT_POLL_INTERVAL_MS");
        assert_eq!(
            parse_u64_env_with_fallback("SNAPSHOT_POLL_INTERVAL_MS", None, 1_000),
            1_000
        );
    }

    #[test]
    fn failover_and_cache_flags_default_to_safe_values() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let path = unique_temp_path("data-plane-config-failover-defaults");
        std::fs::write(
            &path,
            r#"
[data_plane]
listen = "127.0.0.1:18080"
"#,
        )
        .expect("write toml");

        std::env::set_var("DATA_PLANE_CONFIG_FILE", path.display().to_string());

        let cfg = DataPlaneConfig::from_env().expect("load config");

        std::env::remove_var("DATA_PLANE_CONFIG_FILE");
        std::fs::remove_file(path).expect("cleanup toml");

        assert!(cfg.enable_request_failover);
        assert_eq!(cfg.same_account_quick_retry_max, 1);
        assert_eq!(cfg.request_failover_wait_ms, 2_000);
        assert_eq!(cfg.retry_poll_interval_ms, 100);
        assert!(cfg.sticky_prefer_non_conflicting);
        assert!(cfg.shared_routing_cache_enabled);
        assert!(cfg.enable_metered_stream_billing);
        assert!(cfg.billing_authorize_required_for_stream);
        assert_eq!(cfg.stream_billing_reserve_microcredits, 2_000_000);
        assert!(cfg.billing_dynamic_preauth_enabled);
        assert_eq!(cfg.billing_preauth_expected_output_tokens, 256);
        assert_eq!(cfg.billing_preauth_safety_factor, 1.3);
        assert_eq!(cfg.billing_preauth_min_microcredits, 1_000);
        assert_eq!(cfg.billing_preauth_max_microcredits, 1_000_000_000_000);
        assert_eq!(cfg.billing_preauth_unit_price_microcredits, 10_000);
        assert_eq!(cfg.stream_billing_drain_timeout_ms, 5_000);
        assert_eq!(cfg.billing_capture_retry_max, 3);
        assert_eq!(cfg.billing_capture_retry_backoff_ms, 200);
    }

    #[test]
    fn parses_failover_and_cache_flags_from_env() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        std::env::set_var("ENABLE_REQUEST_FAILOVER", "false");
        std::env::set_var("SAME_ACCOUNT_QUICK_RETRY_MAX", "2");
        std::env::set_var("REQUEST_FAILOVER_WAIT_MS", "3500");
        std::env::set_var("RETRY_POLL_INTERVAL_MS", "150");
        std::env::set_var("STICKY_PREFER_NON_CONFLICTING", "false");
        std::env::set_var("SHARED_ROUTING_CACHE_ENABLED", "false");
        std::env::set_var("ENABLE_METERED_STREAM_BILLING", "false");
        std::env::set_var("BILLING_AUTHORIZE_REQUIRED_FOR_STREAM", "false");
        std::env::set_var("STREAM_BILLING_RESERVE_MICROCREDITS", "4500000");
        std::env::set_var("BILLING_DYNAMIC_PREAUTH_ENABLED", "false");
        std::env::set_var("BILLING_PREAUTH_EXPECTED_OUTPUT_TOKENS", "512");
        std::env::set_var("BILLING_PREAUTH_SAFETY_FACTOR", "1.7");
        std::env::set_var("BILLING_PREAUTH_MIN_MICROCREDITS", "2000");
        std::env::set_var("BILLING_PREAUTH_MAX_MICROCREDITS", "9000000");
        std::env::set_var("BILLING_PREAUTH_UNIT_PRICE_MICROCREDITS", "8800");
        std::env::set_var("STREAM_BILLING_DRAIN_TIMEOUT_MS", "6500");
        std::env::set_var("BILLING_CAPTURE_RETRY_MAX", "4");
        std::env::set_var("BILLING_CAPTURE_RETRY_BACKOFF_MS", "350");

        let cfg = DataPlaneConfig::from_env().expect("load config");

        assert!(!cfg.enable_request_failover);
        assert_eq!(cfg.same_account_quick_retry_max, 2);
        assert_eq!(cfg.request_failover_wait_ms, 3_500);
        assert_eq!(cfg.retry_poll_interval_ms, 150);
        assert!(!cfg.sticky_prefer_non_conflicting);
        assert!(!cfg.shared_routing_cache_enabled);
        assert!(!cfg.enable_metered_stream_billing);
        assert!(!cfg.billing_authorize_required_for_stream);
        assert_eq!(cfg.stream_billing_reserve_microcredits, 4_500_000);
        assert!(!cfg.billing_dynamic_preauth_enabled);
        assert_eq!(cfg.billing_preauth_expected_output_tokens, 512);
        assert_eq!(cfg.billing_preauth_safety_factor, 1.7);
        assert_eq!(cfg.billing_preauth_min_microcredits, 2_000);
        assert_eq!(cfg.billing_preauth_max_microcredits, 9_000_000);
        assert_eq!(cfg.billing_preauth_unit_price_microcredits, 8_800);
        assert_eq!(cfg.stream_billing_drain_timeout_ms, 6_500);
        assert_eq!(cfg.billing_capture_retry_max, 4);
        assert_eq!(cfg.billing_capture_retry_backoff_ms, 350);

        std::env::remove_var("ENABLE_REQUEST_FAILOVER");
        std::env::remove_var("SAME_ACCOUNT_QUICK_RETRY_MAX");
        std::env::remove_var("REQUEST_FAILOVER_WAIT_MS");
        std::env::remove_var("RETRY_POLL_INTERVAL_MS");
        std::env::remove_var("STICKY_PREFER_NON_CONFLICTING");
        std::env::remove_var("SHARED_ROUTING_CACHE_ENABLED");
        std::env::remove_var("ENABLE_METERED_STREAM_BILLING");
        std::env::remove_var("BILLING_AUTHORIZE_REQUIRED_FOR_STREAM");
        std::env::remove_var("STREAM_BILLING_RESERVE_MICROCREDITS");
        std::env::remove_var("BILLING_DYNAMIC_PREAUTH_ENABLED");
        std::env::remove_var("BILLING_PREAUTH_EXPECTED_OUTPUT_TOKENS");
        std::env::remove_var("BILLING_PREAUTH_SAFETY_FACTOR");
        std::env::remove_var("BILLING_PREAUTH_MIN_MICROCREDITS");
        std::env::remove_var("BILLING_PREAUTH_MAX_MICROCREDITS");
        std::env::remove_var("BILLING_PREAUTH_UNIT_PRICE_MICROCREDITS");
        std::env::remove_var("STREAM_BILLING_DRAIN_TIMEOUT_MS");
        std::env::remove_var("BILLING_CAPTURE_RETRY_MAX");
        std::env::remove_var("BILLING_CAPTURE_RETRY_BACKOFF_MS");
    }

    #[test]
    fn non_business_editions_disable_credit_billing_defaults() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let path = unique_temp_path("data-plane-config-team-edition");
        std::fs::write(
            &path,
            r#"
[data_plane]
listen = "127.0.0.1:18080"
"#,
        )
        .expect("write toml");

        std::env::set_var("DATA_PLANE_CONFIG_FILE", path.display().to_string());
        std::env::set_var("CODEX_POOL_EDITION", "team");

        let cfg = DataPlaneConfig::from_env().expect("load config");

        std::env::remove_var("DATA_PLANE_CONFIG_FILE");
        std::env::remove_var("CODEX_POOL_EDITION");
        std::fs::remove_file(path).expect("cleanup toml");

        assert!(!cfg.enable_metered_stream_billing);
        assert!(!cfg.billing_authorize_required_for_stream);
        assert!(!cfg.billing_dynamic_preauth_enabled);
    }

    #[test]
    fn non_business_editions_ignore_credit_billing_env_overrides() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let path = unique_temp_path("data-plane-config-personal-edition");
        std::fs::write(
            &path,
            r#"
[data_plane]
listen = "127.0.0.1:18080"
"#,
        )
        .expect("write toml");

        std::env::set_var("DATA_PLANE_CONFIG_FILE", path.display().to_string());
        std::env::set_var("CODEX_POOL_EDITION", "personal");
        std::env::set_var("ENABLE_METERED_STREAM_BILLING", "true");
        std::env::set_var("BILLING_AUTHORIZE_REQUIRED_FOR_STREAM", "true");
        std::env::set_var("BILLING_DYNAMIC_PREAUTH_ENABLED", "true");

        let cfg = DataPlaneConfig::from_env().expect("load config");

        std::env::remove_var("DATA_PLANE_CONFIG_FILE");
        std::env::remove_var("CODEX_POOL_EDITION");
        std::env::remove_var("ENABLE_METERED_STREAM_BILLING");
        std::env::remove_var("BILLING_AUTHORIZE_REQUIRED_FOR_STREAM");
        std::env::remove_var("BILLING_DYNAMIC_PREAUTH_ENABLED");
        std::fs::remove_file(path).expect("cleanup toml");

        assert!(!cfg.enable_metered_stream_billing);
        assert!(!cfg.billing_authorize_required_for_stream);
        assert!(!cfg.billing_dynamic_preauth_enabled);
    }
}
