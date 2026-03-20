#![cfg(feature = "redis-backend")]

use codex_pool_core::events::RequestLogEvent;
use data_plane::event::redis_sink::RedisStreamEventSink;
use uuid::Uuid;

fn sample_event() -> RequestLogEvent {
    RequestLogEvent {
        id: Uuid::new_v4(),
        account_id: Uuid::new_v4(),
        tenant_id: Some(Uuid::new_v4()),
        api_key_id: Some(Uuid::new_v4()),
        event_version: 2,
        path: "/v1/responses".to_string(),
        method: "POST".to_string(),
        status_code: 200,
        latency_ms: 42,
        is_stream: false,
        error_code: None,
        request_id: Some("req-event-sink".to_string()),
        model: Some("gpt-5.3-codex".to_string()),
        service_tier: Some("priority".to_string()),
        input_tokens: Some(12),
        cached_input_tokens: None,
        output_tokens: Some(34),
        reasoning_tokens: None,
        first_token_latency_ms: None,
        billing_phase: None,
        authorization_id: None,
        capture_status: None,
        created_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn redis_event_sink_serializes_request_log_event() {
    let sink = RedisStreamEventSink::new("redis://127.0.0.1:6379", "stream.request_log");
    let event = sample_event();
    let payload = sink.serialize_for_test(&event).unwrap();

    assert!(payload.contains("\"path\":\"/v1/responses\""));
    assert!(payload.contains("\"tenant_id\""));
    assert!(payload.contains("\"api_key_id\""));
    assert!(payload.contains("\"event_version\":2"));
    assert!(payload.contains("\"input_tokens\":12"));
    assert!(payload.contains("\"output_tokens\":34"));
}
