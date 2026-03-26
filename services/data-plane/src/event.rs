use async_trait::async_trait;
use codex_pool_core::events::{RequestLogEvent, SystemEventWrite};
use std::sync::Arc;

pub mod http_sink;
#[cfg(feature = "redis-backend")]
pub mod redis_sink;

#[async_trait]
pub trait EventSink: Send + Sync {
    async fn emit_request_log(&self, event: RequestLogEvent);

    async fn emit_system_event(&self, _event: SystemEventWrite) {}
}

#[derive(Default)]
pub struct NoopEventSink;

#[async_trait]
impl EventSink for NoopEventSink {
    async fn emit_request_log(&self, _event: RequestLogEvent) {}
}

pub struct SplitEventSink {
    request_sink: Arc<dyn EventSink>,
    system_event_sink: Arc<dyn EventSink>,
}

impl SplitEventSink {
    pub fn new(request_sink: Arc<dyn EventSink>, system_event_sink: Arc<dyn EventSink>) -> Self {
        Self {
            request_sink,
            system_event_sink,
        }
    }
}

#[async_trait]
impl EventSink for SplitEventSink {
    async fn emit_request_log(&self, event: RequestLogEvent) {
        self.request_sink.emit_request_log(event).await;
    }

    async fn emit_system_event(&self, event: SystemEventWrite) {
        self.system_event_sink.emit_system_event(event).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    struct RecordingSink {
        request_events: Mutex<Vec<RequestLogEvent>>,
        system_events: Mutex<Vec<SystemEventWrite>>,
    }

    #[async_trait]
    impl EventSink for RecordingSink {
        async fn emit_request_log(&self, event: RequestLogEvent) {
            self.request_events.lock().unwrap().push(event);
        }

        async fn emit_system_event(&self, event: SystemEventWrite) {
            self.system_events.lock().unwrap().push(event);
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn split_event_sink_routes_request_and_system_events_independently() {
        let request_sink = Arc::new(RecordingSink::default());
        let system_sink = Arc::new(RecordingSink::default());
        let sink = SplitEventSink::new(request_sink.clone(), system_sink.clone());

        sink.emit_request_log(RequestLogEvent {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            tenant_id: None,
            api_key_id: None,
            event_version: 2,
            path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            status_code: 200,
            latency_ms: 42,
            is_stream: false,
            error_code: None,
            request_id: Some("req-1".to_string()),
            model: Some("gpt-5.4".to_string()),
            service_tier: None,
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            first_token_latency_ms: None,
            billing_phase: None,
            authorization_id: None,
            capture_status: None,
            created_at: Utc::now(),
        })
        .await;

        sink.emit_system_event(SystemEventWrite {
            event_id: None,
            ts: None,
            category: codex_pool_core::events::SystemEventCategory::Infra,
            event_type: "continuation_cursor_saved".to_string(),
            severity: codex_pool_core::events::SystemEventSeverity::Info,
            source: "data-plane".to_string(),
            tenant_id: None,
            account_id: None,
            request_id: Some("req-1".to_string()),
            trace_request_id: None,
            job_id: None,
            account_label: None,
            auth_provider: None,
            operator_state_from: None,
            operator_state_to: None,
            reason_class: None,
            reason_code: None,
            next_action_at: None,
            path: Some("/v1/responses".to_string()),
            method: Some("POST".to_string()),
            model: Some("gpt-5.4".to_string()),
            selected_account_id: None,
            selected_proxy_id: None,
            routing_decision: None,
            failover_scope: None,
            status_code: None,
            upstream_status_code: None,
            latency_ms: None,
            message: Some("saved".to_string()),
            preview_text: None,
            payload_json: None,
            secret_preview: None,
        })
        .await;

        assert_eq!(request_sink.request_events.lock().unwrap().len(), 1);
        assert!(request_sink.system_events.lock().unwrap().is_empty());
        assert_eq!(system_sink.system_events.lock().unwrap().len(), 1);
        assert!(system_sink.request_events.lock().unwrap().is_empty());
    }
}
