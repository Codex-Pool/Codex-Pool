use std::time::Duration;

use chrono::Utc;
use codex_pool_core::model::{OutboundProxyNode, OutboundProxyPoolSettings, ProxyFailMode};
use data_plane::outbound_proxy_runtime::OutboundProxyRuntime;
use uuid::Uuid;

fn proxy_node(label: &str, proxy_url: &str) -> OutboundProxyNode {
    OutboundProxyNode {
        id: Uuid::new_v4(),
        label: label.to_string(),
        proxy_url: proxy_url.to_string(),
        enabled: true,
        weight: 1,
        last_test_status: None,
        last_latency_ms: None,
        last_error: None,
        last_tested_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn runtime_blocks_or_falls_back_based_on_fail_mode() {
    let runtime = OutboundProxyRuntime::new();
    let node = proxy_node("proxy-a", "http://127.0.0.1:19081");

    runtime.replace_config(
        OutboundProxyPoolSettings {
            enabled: true,
            fail_mode: ProxyFailMode::StrictProxy,
            updated_at: Utc::now(),
        },
        vec![node.clone()],
    );
    let selected = runtime
        .select_http_client(None)
        .await
        .expect("proxy selection should succeed");
    assert_eq!(selected.proxy_id, Some(node.id));
    runtime.mark_proxy_transport_failure(&selected).await;
    assert!(runtime.select_http_client(None).await.is_err());

    runtime.replace_config(
        OutboundProxyPoolSettings {
            enabled: true,
            fail_mode: ProxyFailMode::AllowDirectFallback,
            updated_at: Utc::now(),
        },
        vec![node],
    );
    let fallback = runtime
        .select_http_client(Some(Duration::from_secs(5)))
        .await
        .expect("direct fallback should succeed");
    assert!(fallback.proxy_id.is_none());
    assert!(fallback.used_direct_fallback);
}
