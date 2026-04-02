use std::collections::HashSet;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use codex_pool_core::api::{
    DataPlaneSnapshot, DataPlaneSnapshotEvent, DataPlaneSnapshotEventType,
    DataPlaneSnapshotEventsResponse,
};
use codex_pool_core::model::{
    AccountRoutingTraits, AiErrorLearningSettings, ClaudeCodeRoutingSettings,
    CompiledModelRoutingPolicy, CompiledRoutingPlan, CompiledRoutingProfile,
    LocalizedErrorTemplates, OutboundProxyNode, OutboundProxyPoolSettings, ProxyFailMode,
    RoutingStrategy, UpstreamAccount, UpstreamErrorAction, UpstreamErrorRetryScope,
    UpstreamErrorTemplateRecord, UpstreamErrorTemplateStatus, UpstreamMode,
};
use data_plane::app::AppState;
use data_plane::event::NoopEventSink;
use data_plane::outbound_proxy_runtime::OutboundProxyRuntime;
use data_plane::proxy::BackgroundResponsesRuntime;
use data_plane::router::RoundRobinRouter;
use data_plane::routing_cache::InMemoryRoutingCache;
use uuid::Uuid;

fn account_a() -> UpstreamAccount {
    UpstreamAccount {
        id: Uuid::new_v4(),
        label: "account-a".to_string(),
        mode: UpstreamMode::OpenAiApiKey,
        base_url: "https://api.openai.com/v1".to_string(),
        bearer_token: "token-a".to_string(),
        chatgpt_account_id: None,
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
    }
}

fn account_b() -> UpstreamAccount {
    UpstreamAccount {
        id: Uuid::new_v4(),
        label: "account-b".to_string(),
        mode: UpstreamMode::OpenAiApiKey,
        base_url: "https://api.openai.com/v1".to_string(),
        bearer_token: "token-b".to_string(),
        chatgpt_account_id: None,
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
    }
}

fn snapshot_with_revision(revision: u64, accounts: Vec<UpstreamAccount>) -> DataPlaneSnapshot {
    DataPlaneSnapshot {
        revision,
        cursor: 0,
        accounts,
        account_traits: Vec::<AccountRoutingTraits>::new(),
        compiled_routing_plan: None,
        ai_error_learning_settings: AiErrorLearningSettings::default(),
        approved_upstream_error_templates: Vec::new(),
        builtin_error_templates: Vec::new(),
        outbound_proxy_pool_settings: Default::default(),
        outbound_proxy_nodes: Vec::new(),
        claude_code_routing_settings: ClaudeCodeRoutingSettings::default(),
        issued_at: chrono::Utc::now(),
    }
}

fn approved_template() -> UpstreamErrorTemplateRecord {
    UpstreamErrorTemplateRecord {
        id: Uuid::new_v4(),
        fingerprint: "openai:400:model_not_found".to_string(),
        provider: "openai_compatible".to_string(),
        normalized_status_code: 400,
        semantic_error_code: "unsupported_model".to_string(),
        action: UpstreamErrorAction::ReturnFailure,
        retry_scope: UpstreamErrorRetryScope::None,
        status: UpstreamErrorTemplateStatus::Approved,
        templates: LocalizedErrorTemplates {
            en: Some("The requested model is not available.".to_string()),
            zh_cn: Some("请求的模型当前不可用。".to_string()),
            ..LocalizedErrorTemplates::default()
        },
        representative_samples: vec!["The model {model} does not exist".to_string()],
        hit_count: 12,
        first_seen_at: chrono::Utc::now(),
        last_seen_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn upsert_event(account: UpstreamAccount, id: u64) -> DataPlaneSnapshotEvent {
    DataPlaneSnapshotEvent {
        id,
        event_type: DataPlaneSnapshotEventType::AccountUpsert,
        account_id: account.id,
        account: Some(account),
        compiled_routing_plan: None,
        ai_error_learning_settings: None,
        approved_upstream_error_templates: None,
        builtin_error_templates: None,
        outbound_proxy_pool_settings: None,
        outbound_proxy_nodes: None,
        claude_code_routing_settings: None,
        created_at: chrono::Utc::now(),
    }
}

fn delete_event(account_id: Uuid, id: u64) -> DataPlaneSnapshotEvent {
    DataPlaneSnapshotEvent {
        id,
        event_type: DataPlaneSnapshotEventType::AccountDelete,
        account_id,
        account: None,
        compiled_routing_plan: None,
        ai_error_learning_settings: None,
        approved_upstream_error_templates: None,
        builtin_error_templates: None,
        outbound_proxy_pool_settings: None,
        outbound_proxy_nodes: None,
        claude_code_routing_settings: None,
        created_at: chrono::Utc::now(),
    }
}

fn routing_plan_refresh_event(model: &str, account_id: Uuid, id: u64) -> DataPlaneSnapshotEvent {
    DataPlaneSnapshotEvent {
        id,
        event_type: DataPlaneSnapshotEventType::RoutingPlanRefresh,
        account_id: Uuid::nil(),
        account: None,
        compiled_routing_plan: Some(CompiledRoutingPlan {
            version_id: Uuid::new_v4(),
            published_at: chrono::Utc::now(),
            trigger_reason: Some("test".to_string()),
            default_route: Vec::new(),
            policies: vec![CompiledModelRoutingPolicy {
                id: Uuid::new_v4(),
                name: "test-policy".to_string(),
                family: "test-family".to_string(),
                exact_models: vec![model.to_string()],
                model_prefixes: Vec::new(),
                fallback_segments: vec![CompiledRoutingProfile {
                    id: Uuid::new_v4(),
                    name: "primary".to_string(),
                    account_ids: vec![account_id],
                }],
            }],
        }),
        ai_error_learning_settings: None,
        approved_upstream_error_templates: None,
        builtin_error_templates: None,
        outbound_proxy_pool_settings: None,
        outbound_proxy_nodes: None,
        claude_code_routing_settings: None,
        created_at: chrono::Utc::now(),
    }
}

fn ai_error_learning_refresh_event(
    settings: AiErrorLearningSettings,
    templates: Vec<UpstreamErrorTemplateRecord>,
    id: u64,
) -> DataPlaneSnapshotEvent {
    DataPlaneSnapshotEvent {
        id,
        event_type: DataPlaneSnapshotEventType::RoutingPlanRefresh,
        account_id: Uuid::nil(),
        account: None,
        compiled_routing_plan: None,
        ai_error_learning_settings: Some(settings),
        approved_upstream_error_templates: Some(templates),
        builtin_error_templates: Some(Vec::new()),
        outbound_proxy_pool_settings: None,
        outbound_proxy_nodes: None,
        claude_code_routing_settings: None,
        created_at: chrono::Utc::now(),
    }
}

fn test_state() -> AppState {
    AppState {
        router: RoundRobinRouter::new(vec![account_a()]),
        http_client: reqwest::Client::new(),
        control_plane_base_url: Some("http://127.0.0.1:8090".to_string()),
        routing_strategy: RoutingStrategy::RoundRobin,
        account_ejection_ttl: Duration::from_secs(30),
        enable_request_failover: true,
        same_account_quick_retry_max: 1,
        request_failover_wait: Duration::from_millis(2_000),
        retry_poll_interval: Duration::from_millis(100),
        sticky_prefer_non_conflicting: true,
        shared_routing_cache_enabled: true,
        enable_metered_stream_billing: true,
        billing_authorize_required_for_stream: true,
        stream_billing_reserve_microcredits: 2_000_000,
        billing_dynamic_preauth_enabled: true,
        billing_preauth_expected_output_tokens: 256,
        billing_preauth_safety_factor: 1.3,
        billing_preauth_min_microcredits: 1_000,
        billing_preauth_max_microcredits: 1_000_000_000_000,
        billing_preauth_unit_price_microcredits: 10_000,
        stream_billing_drain_timeout: Duration::from_millis(5_000),
        billing_capture_retry_max: 3,
        billing_capture_retry_backoff: Duration::from_millis(200),
        billing_pricing_cache: std::sync::RwLock::new(std::collections::HashMap::new()),
        models_cache: std::sync::RwLock::new(std::collections::HashMap::new()),
        outbound_proxy_runtime: Arc::new(OutboundProxyRuntime::new()),
        routing_cache: Arc::new(InMemoryRoutingCache::new()),
        #[cfg(feature = "redis-backend")]
        alive_ring_router: None,
        seen_ok_reporter: None,
        event_sink: Arc::new(NoopEventSink),
        auth_validator: None,
        control_plane_internal_auth_token: Arc::from("cp-internal-dev-token-change-me"),
        auth_fail_open: false,
        allowed_api_keys: HashSet::new(),
        snapshot_revision: AtomicU64::new(1),
        snapshot_cursor: AtomicU64::new(0),
        snapshot_remote_cursor: AtomicU64::new(0),
        snapshot_events_apply_total: AtomicU64::new(0),
        snapshot_events_cursor_gone_total: AtomicU64::new(0),
        route_update_notify: Arc::new(tokio::sync::Notify::new()),
        claude_code_routing_settings: std::sync::RwLock::new(ClaudeCodeRoutingSettings::default()),
        ai_error_learning_settings: std::sync::RwLock::new(AiErrorLearningSettings::default()),
        approved_upstream_error_templates: std::sync::RwLock::new(std::collections::HashMap::new()),
        builtin_error_templates: std::sync::RwLock::new(std::collections::HashMap::new()),
        max_request_body_bytes: 10 * 1024 * 1024,
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
        billing_preauth_error_ratio_recent_ppm: std::sync::RwLock::new(
            std::collections::VecDeque::new(),
        ),
        billing_preauth_error_ratio_by_model_ppm: std::sync::RwLock::new(
            std::collections::HashMap::new(),
        ),
        stream_usage_missing_total: AtomicU64::new(0),
        stream_usage_estimated_total: AtomicU64::new(0),
        stream_drain_timeout_total: AtomicU64::new(0),
        stream_response_total: AtomicU64::new(0),
        stream_protocol_sse_header_total: AtomicU64::new(0),
        stream_protocol_header_missing_total: AtomicU64::new(0),
        stream_usage_json_line_fallback_total: AtomicU64::new(0),
        invalid_request_guard_enabled: true,
        invalid_request_guard_window: Duration::from_secs(30),
        invalid_request_guard_threshold: 12,
        invalid_request_guard_block_ttl: Duration::from_secs(120),
        invalid_request_guard: std::sync::RwLock::new(std::collections::HashMap::new()),
        invalid_request_guard_block_total: AtomicU64::new(0),
        background_responses: Arc::new(BackgroundResponsesRuntime::from_env(
            "127.0.0.1:0".parse().unwrap(),
        )),
    }
}

fn proxy_node() -> OutboundProxyNode {
    OutboundProxyNode {
        id: Uuid::new_v4(),
        label: "proxy-a".to_string(),
        proxy_url: "http://127.0.0.1:19082".to_string(),
        enabled: true,
        weight: 1,
        last_test_status: None,
        last_latency_ms: None,
        last_error: None,
        last_tested_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn applies_new_snapshot_revision_and_replaces_accounts() {
    let state = test_state();

    state.apply_snapshot(snapshot_with_revision(2, vec![account_b()]));

    assert_eq!(state.router.total(), 1);
    assert_eq!(state.router.pick().unwrap().label, "account-b");
}

#[tokio::test]
async fn applies_outbound_proxy_settings_from_snapshot() {
    let state = test_state();
    let node = proxy_node();
    state.apply_snapshot(DataPlaneSnapshot {
        revision: 9,
        cursor: 3,
        accounts: vec![account_a()],
        account_traits: Vec::new(),
        compiled_routing_plan: None,
        ai_error_learning_settings: AiErrorLearningSettings::default(),
        approved_upstream_error_templates: Vec::new(),
        builtin_error_templates: Vec::new(),
        outbound_proxy_pool_settings: OutboundProxyPoolSettings {
            enabled: true,
            fail_mode: ProxyFailMode::StrictProxy,
            updated_at: chrono::Utc::now(),
        },
        outbound_proxy_nodes: vec![node.clone()],
        claude_code_routing_settings: ClaudeCodeRoutingSettings::default(),
        issued_at: chrono::Utc::now(),
    });

    let selected = state
        .outbound_proxy_runtime
        .select_http_client(None)
        .await
        .expect("proxy selection should succeed");
    assert_eq!(selected.proxy_id, Some(node.id));

    state
        .outbound_proxy_runtime
        .mark_proxy_transport_failure(&selected)
        .await;
    assert!(state
        .outbound_proxy_runtime
        .select_http_client(None)
        .await
        .is_err());
}

#[tokio::test]
async fn applies_snapshot_when_revision_same_but_cursor_newer() {
    let state = test_state();
    let mut snapshot = snapshot_with_revision(1, vec![account_b()]);
    snapshot.cursor = 10;

    state.apply_snapshot(snapshot);

    assert_eq!(state.router.pick().unwrap().label, "account-b");
    assert_eq!(
        state
            .snapshot_cursor
            .load(std::sync::atomic::Ordering::Relaxed),
        10
    );
}

#[tokio::test]
async fn applies_incremental_snapshot_events() {
    let state = test_state();
    let new_account = account_b();
    let old_account_id = state.router.pick().unwrap().id;

    let cursor = state.apply_snapshot_events(DataPlaneSnapshotEventsResponse {
        cursor: 2,
        high_watermark: 4,
        events: vec![
            delete_event(old_account_id, 1),
            upsert_event(new_account.clone(), 2),
        ],
    });

    assert_eq!(cursor, 2);
    assert_eq!(state.router.total(), 1);
    assert_eq!(state.router.pick().unwrap().id, new_account.id);
    assert_eq!(
        state
            .snapshot_remote_cursor
            .load(std::sync::atomic::Ordering::Relaxed),
        4
    );
}

#[tokio::test]
async fn applies_routing_plan_refresh_event_without_replacing_accounts() {
    let state = test_state();
    let existing = state.router.pick().expect("seed account");

    let cursor = state.apply_snapshot_events(DataPlaneSnapshotEventsResponse {
        cursor: 7,
        high_watermark: 7,
        events: vec![routing_plan_refresh_event("gpt-5.4", existing.id, 7)],
    });

    assert_eq!(cursor, 7);
    assert_eq!(state.router.total(), 1);
    let plan = state
        .router
        .compiled_routing_plan()
        .expect("compiled route plan should be stored");
    assert_eq!(plan.policies.len(), 1);
    assert_eq!(plan.policies[0].exact_models, vec!["gpt-5.4".to_string()]);
}

#[tokio::test]
async fn applies_ai_error_learning_snapshot_contract() {
    let state = test_state();
    let approved = approved_template();
    let mut snapshot = snapshot_with_revision(2, vec![account_b()]);
    snapshot.ai_error_learning_settings = AiErrorLearningSettings {
        enabled: true,
        ..AiErrorLearningSettings::default()
    };
    snapshot.approved_upstream_error_templates = vec![approved.clone()];

    state.apply_snapshot(snapshot);

    let settings = state
        .ai_error_learning_settings
        .read()
        .expect("ai error learning settings");
    assert!(settings.enabled);
    drop(settings);

    let templates = state
        .approved_upstream_error_templates
        .read()
        .expect("approved template map");
    let stored = templates
        .get(&approved.fingerprint)
        .expect("approved template stored by fingerprint");
    assert_eq!(stored.semantic_error_code, "unsupported_model");
    assert_eq!(stored.status, UpstreamErrorTemplateStatus::Approved);
}

#[tokio::test]
async fn applies_ai_error_learning_updates_from_routing_refresh_event() {
    let state = test_state();
    let approved = approved_template();
    let settings = AiErrorLearningSettings {
        enabled: true,
        first_seen_timeout_ms: 2_000,
        review_hit_threshold: 10,
        updated_at: Some(chrono::Utc::now()),
    };

    let cursor = state.apply_snapshot_events(DataPlaneSnapshotEventsResponse {
        cursor: 11,
        high_watermark: 11,
        events: vec![ai_error_learning_refresh_event(
            settings.clone(),
            vec![approved.clone()],
            11,
        )],
    });

    assert_eq!(cursor, 11);
    let stored_settings = state
        .ai_error_learning_settings
        .read()
        .expect("ai error learning settings");
    assert!(stored_settings.enabled);
    assert_eq!(
        stored_settings.first_seen_timeout_ms,
        settings.first_seen_timeout_ms
    );
    drop(stored_settings);

    let templates = state
        .approved_upstream_error_templates
        .read()
        .expect("approved template map");
    assert_eq!(templates.len(), 1);
    assert!(templates.contains_key(&approved.fingerprint));
}
