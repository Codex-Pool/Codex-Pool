use std::collections::HashSet;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use codex_pool_core::api::{
    DataPlaneSnapshot, DataPlaneSnapshotEvent, DataPlaneSnapshotEventType,
    DataPlaneSnapshotEventsResponse,
};
use codex_pool_core::model::{
    AccountRoutingTraits, CompiledModelRoutingPolicy, CompiledRoutingPlan, CompiledRoutingProfile,
    RoutingStrategy, UpstreamAccount, UpstreamMode,
};
use data_plane::app::AppState;
use data_plane::event::NoopEventSink;
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
        issued_at: chrono::Utc::now(),
    }
}

fn upsert_event(account: UpstreamAccount, id: u64) -> DataPlaneSnapshotEvent {
    DataPlaneSnapshotEvent {
        id,
        event_type: DataPlaneSnapshotEventType::AccountUpsert,
        account_id: account.id,
        account: Some(account),
        compiled_routing_plan: None,
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
        models_cache: std::sync::RwLock::new(None),
        routing_cache: Arc::new(InMemoryRoutingCache::new()),
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
