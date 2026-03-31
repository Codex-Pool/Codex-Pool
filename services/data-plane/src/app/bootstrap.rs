use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context};
use axum::body::Body;
use axum::extract::{rejection::JsonRejection, Extension, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Request, StatusCode};
use axum::middleware;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use bytes::Bytes;
use codex_pool_core::api::{ErrorEnvelope, ProductEdition, UsageSummary};
use codex_pool_core::model::{
    AiErrorLearningSettings, BuiltinErrorTemplateRecord, RoutingStrategy,
    UpstreamErrorTemplateRecord, UpstreamMode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Notify;
use uuid::Uuid;

use self::snapshot::SnapshotPoller;
use crate::auth::validator::{AuthCacheLookupResult, AuthCacheStatsSnapshot, AuthValidatorClient};
use crate::auth::{require_api_key, require_internal_service_token, ApiPrincipal};
use crate::config::DataPlaneConfig;
use crate::event::http_sink::ControlPlaneHttpEventSink;
#[cfg(feature = "redis-backend")]
use crate::event::redis_sink::RedisStreamEventSink;
use crate::event::{EventSink, NoopEventSink, SplitEventSink};
use crate::outbound_proxy_runtime::OutboundProxyRuntime;
use crate::proxy::{
    account_responses_test_handler, proxy_handler, proxy_websocket_handler, responses_cancel_handler,
    responses_input_items_handler, responses_input_tokens_handler, responses_retrieve_handler,
    BackgroundResponsesRuntime,
};
use crate::router::RoundRobinRouter;
#[cfg(feature = "redis-backend")]
use crate::routing_cache::HybridRoutingCache;
use crate::routing_cache::{InMemoryRoutingCache, RoutingCache};
#[cfg(feature = "redis-backend")]
use crate::routing_cache::RedisRoutingCache;
use crate::upstream_health::{
    seen_ok_report_config_from_env, SeenOkReporter,
};
#[cfg(feature = "redis-backend")]
use crate::upstream_health::alive_ring_config_from_env;
#[cfg(feature = "redis-backend")]
use crate::upstream_health::AliveRingRouter;

#[path = "../snapshot.rs"]
mod snapshot;

const ALLOWED_API_KEYS_ENV: &str = "DATA_PLANE_ALLOWED_API_KEYS";
const INTERNAL_AUTH_TOKEN_ENV: &str = "CONTROL_PLANE_INTERNAL_AUTH_TOKEN";
const MAX_REQUEST_BODY_BYTES_ENV: &str = "DATA_PLANE_MAX_REQUEST_BODY_BYTES";
const INVALID_REQUEST_GUARD_ENABLED_ENV: &str = "DATA_PLANE_INVALID_REQUEST_GUARD_ENABLED";
const INVALID_REQUEST_GUARD_WINDOW_SEC_ENV: &str = "DATA_PLANE_INVALID_REQUEST_GUARD_WINDOW_SEC";
const INVALID_REQUEST_GUARD_THRESHOLD_ENV: &str = "DATA_PLANE_INVALID_REQUEST_GUARD_THRESHOLD";
const INVALID_REQUEST_GUARD_BLOCK_SEC_ENV: &str = "DATA_PLANE_INVALID_REQUEST_GUARD_BLOCK_SEC";
const DEFAULT_MARK_UNHEALTHY_TTL_SEC: u64 = 30;
const MAX_MARK_UNHEALTHY_TTL_SEC: u64 = 600;
const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;
const MIN_MAX_REQUEST_BODY_BYTES: usize = 1024;
const MAX_MAX_REQUEST_BODY_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_INVALID_REQUEST_GUARD_ENABLED: bool = true;
const DEFAULT_INVALID_REQUEST_GUARD_WINDOW_SEC: u64 = 30;
const MIN_INVALID_REQUEST_GUARD_WINDOW_SEC: u64 = 5;
const MAX_INVALID_REQUEST_GUARD_WINDOW_SEC: u64 = 600;
const DEFAULT_INVALID_REQUEST_GUARD_THRESHOLD: usize = 12;
const MIN_INVALID_REQUEST_GUARD_THRESHOLD: usize = 3;
const MAX_INVALID_REQUEST_GUARD_THRESHOLD: usize = 1_000;
const DEFAULT_INVALID_REQUEST_GUARD_BLOCK_SEC: u64 = 120;
const MIN_INVALID_REQUEST_GUARD_BLOCK_SEC: u64 = 10;
const MAX_INVALID_REQUEST_GUARD_BLOCK_SEC: u64 = 3_600;
#[cfg(feature = "redis-backend")]
const DEFAULT_ROUTING_CACHE_REDIS_PREFIX: &str = "codex_pool:data_plane:routing";
const CODEX_POOL_EDITION_ENV: &str = "CODEX_POOL_EDITION";
const REQUEST_ID_HEADER: &str = "x-request-id";

type InvalidRequestGuardEntry = (VecDeque<Instant>, Option<Instant>);
type InvalidRequestGuardState = HashMap<String, InvalidRequestGuardEntry>;

#[derive(Debug, Clone)]
pub struct CachedBillingPricing {
    pub input_price_microcredits: i64,
    pub cached_input_price_microcredits: i64,
    pub output_price_microcredits: i64,
    pub source: Arc<str>,
    pub expires_at: Instant,
}

#[derive(Debug, Clone)]
pub struct CachedModelsResponse {
    pub body: Bytes,
    pub etag: Arc<str>,
    pub content_type: Option<Arc<str>>,
    pub expires_at: Instant,
}

pub struct AppState {
    pub router: RoundRobinRouter,
    pub http_client: reqwest::Client,
    pub outbound_proxy_runtime: Arc<OutboundProxyRuntime>,
    pub control_plane_base_url: Option<String>,
    pub routing_strategy: RoutingStrategy,
    pub account_ejection_ttl: Duration,
    pub enable_request_failover: bool,
    pub same_account_quick_retry_max: u32,
    pub request_failover_wait: Duration,
    pub retry_poll_interval: Duration,
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
    pub stream_billing_drain_timeout: Duration,
    pub billing_capture_retry_max: u32,
    pub billing_capture_retry_backoff: Duration,
    pub billing_pricing_cache: RwLock<HashMap<String, CachedBillingPricing>>,
    pub models_cache: RwLock<HashMap<String, CachedModelsResponse>>,
    pub routing_cache: Arc<dyn RoutingCache>,
    #[cfg(feature = "redis-backend")]
    pub alive_ring_router: Option<Arc<AliveRingRouter>>,
    pub seen_ok_reporter: Option<Arc<SeenOkReporter>>,
    pub event_sink: Arc<dyn EventSink>,
    pub auth_validator: Option<AuthValidatorClient>,
    pub control_plane_internal_auth_token: Arc<str>,
    pub auth_fail_open: bool,
    pub allowed_api_keys: HashSet<String>,
    pub snapshot_revision: AtomicU64,
    pub snapshot_cursor: AtomicU64,
    pub snapshot_remote_cursor: AtomicU64,
    pub snapshot_events_apply_total: AtomicU64,
    pub snapshot_events_cursor_gone_total: AtomicU64,
    pub route_update_notify: Arc<Notify>,
    pub ai_error_learning_settings: RwLock<AiErrorLearningSettings>,
    pub approved_upstream_error_templates: RwLock<HashMap<String, UpstreamErrorTemplateRecord>>,
    pub builtin_error_templates: RwLock<HashMap<String, BuiltinErrorTemplateRecord>>,
    pub max_request_body_bytes: usize,
    pub failover_attempt_total: AtomicU64,
    pub failover_success_total: AtomicU64,
    pub failover_exhausted_total: AtomicU64,
    pub same_account_retry_total: AtomicU64,
    pub billing_authorize_total: AtomicU64,
    pub billing_authorize_failed_total: AtomicU64,
    pub billing_capture_total: AtomicU64,
    pub billing_capture_failed_total: AtomicU64,
    pub billing_release_total: AtomicU64,
    pub billing_idempotent_hit_total: AtomicU64,
    pub billing_preauth_dynamic_total: AtomicU64,
    pub billing_preauth_fallback_total: AtomicU64,
    pub billing_preauth_amount_microcredits_sum: AtomicU64,
    pub billing_preauth_error_ratio_ppm_sum_total: AtomicU64,
    pub billing_preauth_error_ratio_count_total: AtomicU64,
    pub billing_preauth_capture_missing_total: AtomicU64,
    pub billing_settle_complete_total: AtomicU64,
    pub billing_release_without_capture_total: AtomicU64,
    pub billing_preauth_error_ratio_recent_ppm: RwLock<VecDeque<u64>>,
    pub billing_preauth_error_ratio_by_model_ppm: RwLock<HashMap<String, VecDeque<u64>>>,
    pub stream_usage_missing_total: AtomicU64,
    pub stream_usage_estimated_total: AtomicU64,
    pub stream_drain_timeout_total: AtomicU64,
    pub stream_response_total: AtomicU64,
    pub stream_protocol_sse_header_total: AtomicU64,
    pub stream_protocol_header_missing_total: AtomicU64,
    pub stream_usage_json_line_fallback_total: AtomicU64,
    pub invalid_request_guard_enabled: bool,
    pub invalid_request_guard_window: Duration,
    pub invalid_request_guard_threshold: usize,
    pub invalid_request_guard_block_ttl: Duration,
    pub invalid_request_guard: RwLock<InvalidRequestGuardState>,
    pub invalid_request_guard_block_total: AtomicU64,
    pub background_responses: Arc<BackgroundResponsesRuntime>,
}

impl AppState {
    pub fn notify_route_updated(&self) {
        self.route_update_notify.notify_waiters();
    }

    pub async fn wait_for_route_update(&self, timeout: Duration) {
        if timeout.is_zero() {
            return;
        }
        tokio::select! {
            _ = self.route_update_notify.notified() => {}
            _ = tokio::time::sleep(timeout) => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventSinkKind {
    #[cfg(feature = "redis-backend")]
    Redis,
    ControlPlaneHttp,
    Noop,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoutingCacheKind {
    LocalOnly,
    #[cfg(feature = "redis-backend")]
    SharedRedis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeAdapterProfile {
    event_sink_kind: EventSinkKind,
    routing_cache_kind: RoutingCacheKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotPollerMode {
    Enabled,
    Disabled,
}

fn select_event_sink_kind(
    edition: ProductEdition,
    _redis_url: Option<&str>,
    control_plane_base_url: Option<&str>,
) -> EventSinkKind {
    #[cfg(feature = "redis-backend")]
    if _redis_url.is_some() {
        return EventSinkKind::Redis;
    }

    if matches!(edition, ProductEdition::Team | ProductEdition::Personal)
        && control_plane_base_url
            .map(str::trim)
            .is_some_and(|base_url| !base_url.is_empty())
    {
        return EventSinkKind::ControlPlaneHttp;
    }

    EventSinkKind::Noop
}

fn select_routing_cache_kind(
    _shared_routing_cache_enabled: bool,
    _redis_url: Option<&str>,
) -> RoutingCacheKind {
    #[cfg(feature = "redis-backend")]
    if _shared_routing_cache_enabled && _redis_url.is_some() {
        return RoutingCacheKind::SharedRedis;
    }

    RoutingCacheKind::LocalOnly
}

fn resolve_runtime_adapter_profile(
    edition: ProductEdition,
    control_plane_base_url: Option<&str>,
    shared_routing_cache_enabled: bool,
    redis_url: Option<&str>,
) -> RuntimeAdapterProfile {
    RuntimeAdapterProfile {
        event_sink_kind: select_event_sink_kind(edition, redis_url, control_plane_base_url),
        routing_cache_kind: select_routing_cache_kind(shared_routing_cache_enabled, redis_url),
    }
}

fn build_default_event_sink(config: &DataPlaneConfig) -> anyhow::Result<Arc<dyn EventSink>> {
    let control_plane_base_url = resolve_control_plane_base_url(config);
    let edition = ProductEdition::from_env_var(CODEX_POOL_EDITION_ENV);
    let runtime_profile = resolve_runtime_adapter_profile(
        edition,
        control_plane_base_url.as_deref(),
        config.shared_routing_cache_enabled,
        config.redis_url.as_deref(),
    );
    let request_log_sink: Arc<dyn EventSink> = match runtime_profile.event_sink_kind {
        #[cfg(feature = "redis-backend")]
        EventSinkKind::Redis => {
            let Some(redis_url) = config.redis_url.as_deref() else {
                return Err(anyhow!("redis event sink selected without redis_url"));
            };
            Arc::new(RedisStreamEventSink::new(
                redis_url,
                config.request_log_stream(),
            ))
        }
        EventSinkKind::ControlPlaneHttp => {
            let Some(base_url) = control_plane_base_url.as_deref() else {
                return Err(anyhow!(
                    "control plane http event sink selected without control_plane_base_url"
                ));
            };
            Arc::new(ControlPlaneHttpEventSink::new(
                base_url,
                resolve_internal_auth_token()?,
            ))
        }
        EventSinkKind::Noop => Arc::new(NoopEventSink),
    };

    let system_event_sink: Arc<dyn EventSink> = match control_plane_base_url.as_deref() {
        Some(base_url) if !base_url.trim().is_empty() => Arc::new(ControlPlaneHttpEventSink::new(
            base_url,
            resolve_internal_auth_token()?,
        )),
        _ => Arc::new(NoopEventSink),
    };

    Ok(Arc::new(SplitEventSink::new(
        request_log_sink,
        system_event_sink,
    )))
}

pub async fn build_app(config: DataPlaneConfig) -> anyhow::Result<Router> {
    let event_sink = build_default_event_sink(&config)?;

    build_app_with_options(
        config,
        event_sink,
        allowed_api_keys_from_env(),
        true,
        true,
        SnapshotPollerMode::Enabled,
    )
    .await
    .map(|(app, _)| app)
}

pub async fn build_app_without_status_routes(config: DataPlaneConfig) -> anyhow::Result<Router> {
    let event_sink = build_default_event_sink(&config)?;

    build_app_with_options(
        config,
        event_sink,
        allowed_api_keys_from_env(),
        false,
        true,
        SnapshotPollerMode::Enabled,
    )
    .await
    .map(|(app, _)| app)
}

pub async fn build_app_without_status_or_internal_metrics_routes(
    config: DataPlaneConfig,
) -> anyhow::Result<Router> {
    let event_sink = build_default_event_sink(&config)?;

    build_app_with_options(
        config,
        event_sink,
        allowed_api_keys_from_env(),
        false,
        false,
        SnapshotPollerMode::Enabled,
    )
    .await
    .map(|(app, _)| app)
}

pub async fn build_embedded_app_without_status_routes(
    config: DataPlaneConfig,
) -> anyhow::Result<(Router, Arc<AppState>)> {
    let event_sink = build_default_event_sink(&config)?;

    build_embedded_app_with_event_sink_without_status_routes(config, event_sink).await
}

pub async fn build_embedded_app_with_event_sink_without_status_routes(
    config: DataPlaneConfig,
    event_sink: Arc<dyn EventSink>,
) -> anyhow::Result<(Router, Arc<AppState>)> {
    build_app_with_options(
        config,
        event_sink,
        allowed_api_keys_from_env(),
        false,
        true,
        SnapshotPollerMode::Disabled,
    )
    .await
}

pub async fn build_embedded_app_with_event_sink_without_status_or_internal_metrics_routes(
    config: DataPlaneConfig,
    event_sink: Arc<dyn EventSink>,
) -> anyhow::Result<(Router, Arc<AppState>)> {
    build_app_with_options(
        config,
        event_sink,
        allowed_api_keys_from_env(),
        false,
        false,
        SnapshotPollerMode::Disabled,
    )
    .await
}

pub async fn build_app_with_event_sink(
    config: DataPlaneConfig,
    event_sink: Arc<dyn EventSink>,
) -> anyhow::Result<Router> {
    build_app_with_options(
        config,
        event_sink,
        allowed_api_keys_from_env(),
        true,
        true,
        SnapshotPollerMode::Enabled,
    )
    .await
    .map(|(app, _)| app)
}

pub async fn build_app_with_event_sink_without_status_routes(
    config: DataPlaneConfig,
    event_sink: Arc<dyn EventSink>,
) -> anyhow::Result<Router> {
    build_app_with_options(
        config,
        event_sink,
        allowed_api_keys_from_env(),
        false,
        true,
        SnapshotPollerMode::Enabled,
    )
    .await
    .map(|(app, _)| app)
}

pub async fn build_app_with_event_sink_and_allowed_keys(
    config: DataPlaneConfig,
    event_sink: Arc<dyn EventSink>,
    allowed_api_keys: Vec<String>,
) -> anyhow::Result<Router> {
    build_app_with_options(
        config,
        event_sink,
        allowed_api_keys,
        true,
        true,
        SnapshotPollerMode::Enabled,
    )
    .await
    .map(|(app, _)| app)
}

async fn build_app_with_options(
    config: DataPlaneConfig,
    event_sink: Arc<dyn EventSink>,
    allowed_api_keys: Vec<String>,
    include_status_routes: bool,
    include_internal_metrics_route: bool,
    snapshot_poller_mode: SnapshotPollerMode,
) -> anyhow::Result<(Router, Arc<AppState>)> {
    let control_plane_base_url = resolve_control_plane_base_url(&config);
    let control_plane_internal_auth_token = resolve_internal_auth_token()?;
    let runtime_profile = resolve_runtime_adapter_profile(
        ProductEdition::from_env_var(CODEX_POOL_EDITION_ENV),
        control_plane_base_url.as_deref(),
        config.shared_routing_cache_enabled,
        config.redis_url.as_deref(),
    );
    let routing_cache_kind = runtime_profile.routing_cache_kind;
    let enable_internal_debug_routes = config.enable_internal_debug_routes;
    let auth_validator = config.auth_validate_url.as_ref().map(|url| {
        AuthValidatorClient::new(
            url.clone(),
            config.auth_validate_cache_ttl_sec,
            config.auth_validate_negative_cache_ttl_sec,
            control_plane_internal_auth_token.to_string(),
        )
    });
    let local_routing_cache = Arc::new(InMemoryRoutingCache::new());
    let routing_cache: Arc<dyn RoutingCache> = {
        #[cfg(feature = "redis-backend")]
        {
            match routing_cache_kind {
                RoutingCacheKind::SharedRedis => {
                    let redis_url = config
                        .redis_url
                        .as_deref()
                        .expect("shared redis routing cache requires redis_url");
                    Arc::new(HybridRoutingCache::with_shared(
                        local_routing_cache.clone(),
                        Arc::new(RedisRoutingCache::new(
                            redis_url,
                            DEFAULT_ROUTING_CACHE_REDIS_PREFIX,
                        )),
                    ))
                }
                RoutingCacheKind::LocalOnly if config.shared_routing_cache_enabled => {
                    Arc::new(HybridRoutingCache::local_only(local_routing_cache.clone()))
                }
                RoutingCacheKind::LocalOnly => local_routing_cache,
            }
        }
        #[cfg(not(feature = "redis-backend"))]
        {
            let _ = routing_cache_kind;
            local_routing_cache
        }
    };
    #[cfg(feature = "redis-backend")]
    let alive_ring_config = alive_ring_config_from_env();
    #[cfg(feature = "redis-backend")]
    let alive_ring_router = if alive_ring_config.enabled {
        config.redis_url.as_deref().and_then(|redis_url| {
            AliveRingRouter::new(
                redis_url,
                &alive_ring_config.redis_prefix,
                alive_ring_config.fetch_limit,
                alive_ring_config.candidate_count,
                alive_ring_config.cache_ttl,
            )
            .ok()
            .map(Arc::new)
        })
    } else {
        None
    };
    #[cfg(not(feature = "redis-backend"))]
    let _alive_ring_router: Option<()> = None;
    let seen_ok_config = seen_ok_report_config_from_env();
    let seen_ok_reporter = if seen_ok_config.enabled {
        control_plane_base_url.as_ref().and_then(|base_url| {
            SeenOkReporter::new(
                base_url.clone(),
                control_plane_internal_auth_token.clone(),
                seen_ok_config.timeout,
                seen_ok_config.min_interval,
            )
            .ok()
            .map(Arc::new)
        })
    } else {
        None
    };
    let invalid_request_guard_enabled = parse_bool_env_with_default(
        INVALID_REQUEST_GUARD_ENABLED_ENV,
        DEFAULT_INVALID_REQUEST_GUARD_ENABLED,
    );
    let invalid_request_guard_window_sec = parse_u64_env_with_default_clamped(
        INVALID_REQUEST_GUARD_WINDOW_SEC_ENV,
        DEFAULT_INVALID_REQUEST_GUARD_WINDOW_SEC,
        MIN_INVALID_REQUEST_GUARD_WINDOW_SEC,
        MAX_INVALID_REQUEST_GUARD_WINDOW_SEC,
    );
    let invalid_request_guard_threshold = parse_usize_env_with_default_clamped(
        INVALID_REQUEST_GUARD_THRESHOLD_ENV,
        DEFAULT_INVALID_REQUEST_GUARD_THRESHOLD,
        MIN_INVALID_REQUEST_GUARD_THRESHOLD,
        MAX_INVALID_REQUEST_GUARD_THRESHOLD,
    );
    let invalid_request_guard_block_sec = parse_u64_env_with_default_clamped(
        INVALID_REQUEST_GUARD_BLOCK_SEC_ENV,
        DEFAULT_INVALID_REQUEST_GUARD_BLOCK_SEC,
        MIN_INVALID_REQUEST_GUARD_BLOCK_SEC,
        MAX_INVALID_REQUEST_GUARD_BLOCK_SEC,
    );

    let state = Arc::new(AppState {
        router: RoundRobinRouter::new(config.upstream_accounts),
        http_client: reqwest::Client::new(),
        outbound_proxy_runtime: Arc::new(OutboundProxyRuntime::new()),
        control_plane_base_url,
        routing_strategy: config.routing_strategy,
        account_ejection_ttl: Duration::from_secs(config.account_ejection_ttl_sec),
        enable_request_failover: config.enable_request_failover,
        same_account_quick_retry_max: config.same_account_quick_retry_max,
        request_failover_wait: Duration::from_millis(config.request_failover_wait_ms),
        retry_poll_interval: Duration::from_millis(config.retry_poll_interval_ms),
        sticky_prefer_non_conflicting: config.sticky_prefer_non_conflicting,
        shared_routing_cache_enabled: config.shared_routing_cache_enabled,
        enable_metered_stream_billing: config.enable_metered_stream_billing,
        billing_authorize_required_for_stream: config.billing_authorize_required_for_stream,
        stream_billing_reserve_microcredits: config.stream_billing_reserve_microcredits,
        billing_dynamic_preauth_enabled: config.billing_dynamic_preauth_enabled,
        billing_preauth_expected_output_tokens: config.billing_preauth_expected_output_tokens,
        billing_preauth_safety_factor: config.billing_preauth_safety_factor,
        billing_preauth_min_microcredits: config.billing_preauth_min_microcredits,
        billing_preauth_max_microcredits: config.billing_preauth_max_microcredits,
        billing_preauth_unit_price_microcredits: config.billing_preauth_unit_price_microcredits,
        stream_billing_drain_timeout: Duration::from_millis(config.stream_billing_drain_timeout_ms),
        billing_capture_retry_max: config.billing_capture_retry_max,
        billing_capture_retry_backoff: Duration::from_millis(
            config.billing_capture_retry_backoff_ms,
        ),
        billing_pricing_cache: RwLock::new(HashMap::new()),
        models_cache: RwLock::new(HashMap::new()),
        routing_cache,
        #[cfg(feature = "redis-backend")]
        alive_ring_router,
        seen_ok_reporter,
        event_sink,
        auth_validator,
        control_plane_internal_auth_token,
        auth_fail_open: config.auth_fail_open,
        allowed_api_keys: allowed_api_keys.into_iter().collect(),
        snapshot_revision: AtomicU64::new(0),
        snapshot_cursor: AtomicU64::new(0),
        snapshot_remote_cursor: AtomicU64::new(0),
        snapshot_events_apply_total: AtomicU64::new(0),
        snapshot_events_cursor_gone_total: AtomicU64::new(0),
        route_update_notify: Arc::new(Notify::new()),
        ai_error_learning_settings: RwLock::new(AiErrorLearningSettings::default()),
        approved_upstream_error_templates: RwLock::new(HashMap::new()),
        builtin_error_templates: RwLock::new(HashMap::new()),
        max_request_body_bytes: max_request_body_bytes_from_env(),
        failover_attempt_total: AtomicU64::new(0),
        failover_success_total: AtomicU64::new(0),
        failover_exhausted_total: AtomicU64::new(0),
        same_account_retry_total: AtomicU64::new(0),
        billing_authorize_total: AtomicU64::new(0),
        billing_authorize_failed_total: AtomicU64::new(0),
        billing_capture_total: AtomicU64::new(0),
        billing_capture_failed_total: AtomicU64::new(0),
        billing_release_total: AtomicU64::new(0),
        billing_idempotent_hit_total: AtomicU64::new(0),
        billing_preauth_dynamic_total: AtomicU64::new(0),
        billing_preauth_fallback_total: AtomicU64::new(0),
        billing_preauth_amount_microcredits_sum: AtomicU64::new(0),
        billing_preauth_error_ratio_ppm_sum_total: AtomicU64::new(0),
        billing_preauth_error_ratio_count_total: AtomicU64::new(0),
        billing_preauth_capture_missing_total: AtomicU64::new(0),
        billing_settle_complete_total: AtomicU64::new(0),
        billing_release_without_capture_total: AtomicU64::new(0),
        billing_preauth_error_ratio_recent_ppm: RwLock::new(VecDeque::new()),
        billing_preauth_error_ratio_by_model_ppm: RwLock::new(HashMap::new()),
        stream_usage_missing_total: AtomicU64::new(0),
        stream_usage_estimated_total: AtomicU64::new(0),
        stream_drain_timeout_total: AtomicU64::new(0),
        stream_response_total: AtomicU64::new(0),
        stream_protocol_sse_header_total: AtomicU64::new(0),
        stream_protocol_header_missing_total: AtomicU64::new(0),
        stream_usage_json_line_fallback_total: AtomicU64::new(0),
        invalid_request_guard_enabled,
        invalid_request_guard_window: Duration::from_secs(invalid_request_guard_window_sec),
        invalid_request_guard_threshold,
        invalid_request_guard_block_ttl: Duration::from_secs(invalid_request_guard_block_sec),
        invalid_request_guard: RwLock::new(HashMap::new()),
        invalid_request_guard_block_total: AtomicU64::new(0),
        background_responses: Arc::new(BackgroundResponsesRuntime::from_env(config.listen_addr)),
    });

    if matches!(snapshot_poller_mode, SnapshotPollerMode::Enabled) {
        if let Some(poller) = SnapshotPoller::from_env(state.http_client.clone(), state.clone()) {
            tokio::spawn(async move {
                poller.run().await;
            });
        }
    }

    {
        let runtime = state.background_responses.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(runtime.cleanup_interval());
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                runtime.cleanup_expired().await;
            }
        });
    }

    let protected_routes = Router::new()
        .route(
            "/v1/responses",
            post(proxy_handler).get(proxy_websocket_handler),
        )
        .route("/v1/responses/input_tokens", post(responses_input_tokens_handler))
        .route(
            "/v1/responses/{response_id}/input_items",
            get(responses_input_items_handler),
        )
        .route("/v1/responses/{response_id}", get(responses_retrieve_handler))
        .route(
            "/v1/responses/{response_id}/cancel",
            post(responses_cancel_handler),
        )
        .route("/v1/responses/compact", post(proxy_handler))
        .route("/v1/memories/trace_summarize", post(proxy_handler))
        .route(
            "/backend-api/codex/responses",
            post(proxy_handler).get(proxy_websocket_handler),
        )
        .route("/v1/chat/completions", post(proxy_handler))
        .route("/v1/models", get(proxy_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_api_key,
        ));

    let mut app = Router::new()
        .route("/api/codex/usage", get(usage_handler))
        .merge(protected_routes);

    if include_internal_metrics_route {
        let internal_metrics_routes = Router::new()
            .route("/internal/v1/metrics", get(internal_metrics))
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                require_internal_service_token,
            ));

        app = app.merge(internal_metrics_routes);
    }

    let internal_responses_test_routes = Router::new()
        .route(
            "/internal/v1/upstream-accounts/{account_id}/responses/test",
            post(account_responses_test_handler),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_internal_service_token,
        ));

    app = app.merge(internal_responses_test_routes);

    if include_status_routes {
        app = app
            .route("/health", get(healthz))
            .route("/livez", get(livez))
            .route("/readyz", get(readyz));
    }

    if enable_internal_debug_routes {
        let internal_debug_routes = Router::new()
            .route("/internal/v1/auth/whoami", get(internal_auth_whoami))
            .route("/internal/v1/debug/state", get(internal_debug_state))
            .route(
                "/internal/v1/debug/auth-cache",
                get(internal_debug_auth_cache),
            )
            .route(
                "/internal/v1/debug/auth-cache/stats",
                get(internal_debug_auth_cache_stats),
            )
            .route(
                "/internal/v1/debug/auth-cache/stats/reset",
                post(internal_debug_reset_auth_cache_stats),
            )
            .route(
                "/internal/v1/debug/auth-cache/lookup",
                post(internal_debug_lookup_auth_cache),
            )
            .route(
                "/internal/v1/debug/auth-cache/evict",
                post(internal_debug_evict_auth_cache),
            )
            .route(
                "/internal/v1/debug/auth-cache/clear",
                post(internal_debug_clear_auth_cache),
            )
            .route("/internal/v1/debug/accounts", get(internal_debug_accounts))
            .route(
                "/internal/v1/debug/accounts/unhealthy",
                get(internal_debug_unhealthy_accounts),
            )
            .route(
                "/internal/v1/debug/accounts/{account_id}",
                get(internal_debug_account_by_id),
            )
            .route(
                "/internal/v1/debug/accounts/clear-unhealthy",
                post(internal_debug_clear_unhealthy),
            )
            .route(
                "/internal/v1/debug/accounts/{account_id}/mark-unhealthy",
                post(internal_debug_mark_unhealthy),
            )
            .route(
                "/internal/v1/debug/accounts/{account_id}/mark-healthy",
                post(internal_debug_mark_healthy),
            )
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                require_api_key,
            ));

        app = app.merge(internal_debug_routes);
    }

    let app = app
        .layer(middleware::from_fn(request_id_middleware))
        .with_state(state.clone());

    Ok((app, state))
}

fn read_request_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn ensure_request_id(headers: &mut HeaderMap) -> String {
    let request_id =
        read_request_id_from_headers(headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        headers.insert(HeaderName::from_static(REQUEST_ID_HEADER), header_value);
    }
    request_id
}

async fn request_id_middleware(mut req: Request<Body>, next: Next) -> Response {
    let request_id = ensure_request_id(req.headers_mut());
    let mut response = next.run(req).await;
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(REQUEST_ID_HEADER), header_value);
    }
    response
}

async fn usage_handler(State(state): State<Arc<AppState>>) -> axum::Json<UsageSummary> {
    let now = chrono::Utc::now();
    axum::Json(UsageSummary {
        account_total: state.router.total(),
        active_account_total: state.router.enabled_total(),
        window_limit_tokens: 0,
        window_used_tokens: 0,
        window_reset_at: now,
    })
}

fn allowed_api_keys_from_env() -> Vec<String> {
    let Ok(raw) = std::env::var(ALLOWED_API_KEYS_ENV) else {
        return Vec::new();
    };

    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn max_request_body_bytes_from_env() -> usize {
    std::env::var(MAX_REQUEST_BODY_BYTES_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_REQUEST_BODY_BYTES)
        .clamp(MIN_MAX_REQUEST_BODY_BYTES, MAX_MAX_REQUEST_BODY_BYTES)
}

fn parse_bool_env_with_default(key: &str, default: bool) -> bool {
    std::env::var(key).ok().map_or(default, |raw| {
        matches!(
            raw.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn parse_u64_env_with_default_clamped(key: &str, default: u64, min: u64, max: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn parse_usize_env_with_default_clamped(
    key: &str,
    default: usize,
    min: usize,
    max: usize,
) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn resolve_internal_auth_token() -> anyhow::Result<Arc<str>> {
    let token = std::env::var(INTERNAL_AUTH_TOKEN_ENV)
        .with_context(|| format!("{INTERNAL_AUTH_TOKEN_ENV} is required and must be set"))?;
    let token = token.trim();
    if token.is_empty() {
        return Err(anyhow!(
            "{INTERNAL_AUTH_TOKEN_ENV} is required and must not be empty"
        ));
    }
    Ok(token.to_string().into())
}

fn resolve_control_plane_base_url(config: &DataPlaneConfig) -> Option<String> {
    if let Some(url) = std::env::var("CONTROL_PLANE_BASE_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
    {
        return Some(url);
    }

    config.auth_validate_url.as_ref().and_then(|url| {
        let trimmed = url.trim().trim_end_matches('/');
        trimmed
            .strip_suffix("/internal/v1/auth/validate")
            .map(|prefix| prefix.trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
    })
}

pub async fn healthz() -> axum::Json<serde_json::Value> {
    ok_response()
}

pub async fn livez() -> axum::Json<serde_json::Value> {
    ok_response()
}

fn ok_response() -> axum::Json<serde_json::Value> {
    axum::Json(json!({"ok": true}))
}

#[derive(Serialize)]
struct ReadinessResponse {
    ok: bool,
    reason: String,
    account_total: usize,
    active_account_total: usize,
    auth_validator_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<codex_pool_core::api::ErrorBody>,
}

async fn readyz(State(state): State<Arc<AppState>>) -> (StatusCode, axum::Json<ReadinessResponse>) {
    let account_total = state.router.total();
    let active_account_total = state.router.enabled_total();
    let auth_validator_enabled = state.auth_validator.is_some();

    if active_account_total > 0 {
        return (
            StatusCode::OK,
            axum::Json(ReadinessResponse {
                ok: true,
                reason: "ready".to_string(),
                account_total,
                active_account_total,
                auth_validator_enabled,
                error: None,
            }),
        );
    }

    let envelope = ErrorEnvelope::new("not_ready", "no active upstream accounts");

    (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(ReadinessResponse {
            ok: false,
            reason: "no_active_accounts".to_string(),
            account_total,
            active_account_total,
            auth_validator_enabled,
            error: Some(envelope.error),
        }),
    )
}

#[cfg(test)]
mod bootstrap_tests {
    use super::{
        build_app_with_event_sink_without_status_routes, select_event_sink_kind,
        select_routing_cache_kind, resolve_runtime_adapter_profile, EventSinkKind,
        RoutingCacheKind,
    };
    use crate::config::DataPlaneConfig;
    use crate::event::NoopEventSink;
    use crate::test_support::{set_env, ENV_LOCK};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use codex_pool_core::api::ProductEdition;
    use codex_pool_core::model::RoutingStrategy;
    use std::sync::Arc;
    use tower::util::ServiceExt;

    fn test_config() -> DataPlaneConfig {
        DataPlaneConfig {
            listen_addr: "127.0.0.1:8091".parse().unwrap(),
            routing_strategy: RoutingStrategy::RoundRobin,
            upstream_accounts: Vec::new(),
            account_ejection_ttl_sec: 30,
            enable_request_failover: true,
            same_account_quick_retry_max: 1,
            request_failover_wait_ms: 100,
            retry_poll_interval_ms: 50,
            sticky_prefer_non_conflicting: true,
            shared_routing_cache_enabled: false,
            enable_metered_stream_billing: false,
            billing_authorize_required_for_stream: false,
            stream_billing_reserve_microcredits: 0,
            billing_dynamic_preauth_enabled: false,
            billing_preauth_expected_output_tokens: 0,
            billing_preauth_safety_factor: 1.0,
            billing_preauth_min_microcredits: 0,
            billing_preauth_max_microcredits: 0,
            billing_preauth_unit_price_microcredits: 0,
            stream_billing_drain_timeout_ms: 500,
            billing_capture_retry_max: 1,
            billing_capture_retry_backoff_ms: 50,
            redis_url: None,
            auth_validate_url: None,
            auth_validate_cache_ttl_sec: 0,
            auth_validate_negative_cache_ttl_sec: 0,
            auth_fail_open: false,
            enable_internal_debug_routes: false,
        }
    }

    #[test]
    fn select_event_sink_kind_prefers_redis_when_available() {
        let selected = select_event_sink_kind(
            ProductEdition::Team,
            Some("redis://127.0.0.1:6379"),
            Some("http://127.0.0.1:8090"),
        );

        #[cfg(feature = "redis-backend")]
        assert_eq!(selected, EventSinkKind::Redis);
        #[cfg(not(feature = "redis-backend"))]
        assert_eq!(selected, EventSinkKind::ControlPlaneHttp);
    }

    #[test]
    fn select_event_sink_kind_uses_control_plane_http_for_team_without_redis() {
        let selected =
            select_event_sink_kind(ProductEdition::Team, None, Some("http://127.0.0.1:8090"));

        assert_eq!(selected, EventSinkKind::ControlPlaneHttp);
    }

    #[test]
    fn select_event_sink_kind_uses_control_plane_http_for_personal_without_redis() {
        let selected = select_event_sink_kind(
            ProductEdition::Personal,
            None,
            Some("http://127.0.0.1:8090"),
        );

        assert_eq!(selected, EventSinkKind::ControlPlaneHttp);
    }

    #[test]
    fn select_event_sink_kind_falls_back_to_noop_without_redis_or_team_base_url() {
        let selected = select_event_sink_kind(ProductEdition::Personal, None, None);

        assert_eq!(selected, EventSinkKind::Noop);
    }

    #[test]
    fn select_routing_cache_kind_stays_local_without_shared_redis() {
        let selected = select_routing_cache_kind(false, Some("redis://127.0.0.1:6379"));
        assert_eq!(selected, RoutingCacheKind::LocalOnly);

        let selected = select_routing_cache_kind(true, None);
        assert_eq!(selected, RoutingCacheKind::LocalOnly);
    }

    #[cfg(feature = "redis-backend")]
    #[test]
    fn select_routing_cache_kind_uses_shared_redis_when_enabled_and_available() {
        let selected = select_routing_cache_kind(true, Some("redis://127.0.0.1:6379"));
        assert_eq!(selected, RoutingCacheKind::SharedRedis);
    }

    #[test]
    fn resolve_runtime_adapter_profile_uses_control_plane_http_and_local_cache_without_redis() {
        let profile = resolve_runtime_adapter_profile(
            ProductEdition::Team,
            Some("http://127.0.0.1:8090"),
            false,
            None,
        );

        assert_eq!(profile.event_sink_kind, EventSinkKind::ControlPlaneHttp);
        assert_eq!(profile.routing_cache_kind, RoutingCacheKind::LocalOnly);
    }

    #[test]
    fn resolve_runtime_adapter_profile_prefers_redis_backends_when_available() {
        let profile = resolve_runtime_adapter_profile(
            ProductEdition::Team,
            Some("http://127.0.0.1:8090"),
            true,
            Some("redis://127.0.0.1:6379"),
        );

        #[cfg(feature = "redis-backend")]
        {
            assert_eq!(profile.event_sink_kind, EventSinkKind::Redis);
            assert_eq!(profile.routing_cache_kind, RoutingCacheKind::SharedRedis);
        }

        #[cfg(not(feature = "redis-backend"))]
        {
            assert_eq!(profile.event_sink_kind, EventSinkKind::ControlPlaneHttp);
            assert_eq!(profile.routing_cache_kind, RoutingCacheKind::LocalOnly);
        }
    }

    #[tokio::test]
    async fn build_app_without_status_routes_omits_health_endpoints() {
        let _guard = ENV_LOCK.lock().await;
        let old_internal_auth =
            set_env("CONTROL_PLANE_INTERNAL_AUTH_TOKEN", Some("test-internal-token"));

        let app = build_app_with_event_sink_without_status_routes(
            test_config(),
            Arc::new(NoopEventSink),
        )
        .await
        .expect("build app without status routes");

        let health = app
            .clone()
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .expect("health response");
        assert_eq!(health.status(), StatusCode::NOT_FOUND);

        let metrics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/internal/v1/metrics")
                    .header("authorization", "Bearer test-internal-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        assert_eq!(metrics.status(), StatusCode::OK);

        let usage = app
            .oneshot(
                Request::builder()
                    .uri("/api/codex/usage")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("usage response");
        assert_eq!(usage.status(), StatusCode::OK);

        set_env(
            "CONTROL_PLANE_INTERNAL_AUTH_TOKEN",
            old_internal_auth.as_deref(),
        );
    }
}
