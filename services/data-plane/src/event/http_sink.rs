use std::sync::Arc;

use async_trait::async_trait;
use codex_pool_core::events::{RequestLogEvent, SystemEventWrite};
use tracing::warn;

use super::EventSink;

const REQUEST_LOGS_PATH: &str = "/internal/v1/usage/request-logs";
const SYSTEM_EVENTS_PATH: &str = "/internal/v1/system-events";

pub struct ControlPlaneHttpEventSink {
    client: reqwest::Client,
    base_url: String,
    endpoint_url: String,
    internal_auth_token: Arc<str>,
}

impl ControlPlaneHttpEventSink {
    pub fn new(control_plane_base_url: impl AsRef<str>, internal_auth_token: Arc<str>) -> Self {
        let base_url = control_plane_base_url.as_ref().trim_end_matches('/').to_string();
        let endpoint_url = format!(
            "{}{}",
            base_url,
            REQUEST_LOGS_PATH
        );
        Self {
            client: reqwest::Client::new(),
            base_url,
            endpoint_url,
            internal_auth_token,
        }
    }

    async fn post_request_log(&self, event: &RequestLogEvent) -> anyhow::Result<()> {
        self.client
            .post(&self.endpoint_url)
            .bearer_auth(self.internal_auth_token.as_ref())
            .json(event)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn post_system_event(&self, event: &SystemEventWrite) -> anyhow::Result<()> {
        let endpoint_url = format!("{}{}", self.base_url, SYSTEM_EVENTS_PATH);
        self.client
            .post(&endpoint_url)
            .bearer_auth(self.internal_auth_token.as_ref())
            .json(event)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[async_trait]
impl EventSink for ControlPlaneHttpEventSink {
    async fn emit_request_log(&self, event: RequestLogEvent) {
        if let Err(error) = self.post_request_log(&event).await {
            warn!(
                endpoint_url = %self.endpoint_url,
                error = %error,
                "failed to post request log event to control plane"
            );
        }
    }

    async fn emit_system_event(&self, event: SystemEventWrite) {
        if let Err(error) = self.post_system_event(&event).await {
            warn!(error = %error, "failed to post system event to control plane");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use codex_pool_core::events::{SystemEventCategory, SystemEventSeverity};
    use serde_json::Value;
    use uuid::Uuid;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
            request_id: Some("req-http-sink".to_string()),
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
            created_at: Utc::now(),
        }
    }

    fn sample_system_event() -> SystemEventWrite {
        SystemEventWrite {
            event_id: None,
            ts: None,
            category: SystemEventCategory::Request,
            event_type: "cross_account_failover".to_string(),
            severity: SystemEventSeverity::Warn,
            source: "data-plane".to_string(),
            tenant_id: Some(Uuid::new_v4()),
            account_id: Some(Uuid::new_v4()),
            request_id: Some("req-system-event".to_string()),
            trace_request_id: None,
            job_id: None,
            account_label: Some("oauth-demo@example.com".to_string()),
            auth_provider: Some("legacy_bearer".to_string()),
            operator_state_from: None,
            operator_state_to: None,
            reason_class: Some("transient".to_string()),
            reason_code: Some("transport_error".to_string()),
            next_action_at: None,
            path: Some("/v1/responses".to_string()),
            method: Some("POST".to_string()),
            model: Some("gpt-5.4".to_string()),
            selected_account_id: None,
            selected_proxy_id: None,
            routing_decision: Some("cross_account_failover".to_string()),
            failover_scope: Some("cross_account".to_string()),
            status_code: Some(502),
            upstream_status_code: None,
            latency_ms: Some(123),
            message: Some("switching account after retryable upstream failure".to_string()),
            preview_text: None,
            payload_json: None,
            secret_preview: None,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn control_plane_http_event_sink_posts_request_log_with_bearer_token() {
        let control_plane = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(202))
            .mount(&control_plane)
            .await;

        let sink = ControlPlaneHttpEventSink::new(
            control_plane.uri(),
            Arc::<str>::from("cp-internal-token"),
        );
        let event = sample_event();

        sink.emit_request_log(event.clone()).await;

        let requests = control_plane.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);

        let request = &requests[0];
        assert_eq!(request.url.path(), "/internal/v1/usage/request-logs");
        assert_eq!(
            request
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer cp-internal-token")
        );

        let payload: Value = request.body_json().unwrap();
        assert_eq!(payload, serde_json::to_value(&event).unwrap());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn control_plane_http_event_sink_posts_system_event_with_bearer_token() {
        let control_plane = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&control_plane)
            .await;

        let sink = ControlPlaneHttpEventSink::new(
            control_plane.uri(),
            Arc::<str>::from("cp-internal-token"),
        );
        let event = sample_system_event();

        sink.emit_system_event(event.clone()).await;

        let requests = control_plane.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);

        let request = &requests[0];
        assert_eq!(request.url.path(), "/internal/v1/system-events");
        assert_eq!(
            request
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer cp-internal-token")
        );

        let payload: Value = request.body_json().unwrap();
        assert_eq!(payload, serde_json::to_value(&event).unwrap());
    }
}
