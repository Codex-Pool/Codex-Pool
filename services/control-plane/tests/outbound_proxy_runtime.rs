use std::sync::Arc;
use std::time::Duration;

use codex_pool_core::api::{
    CreateOutboundProxyNodeRequest, UpdateOutboundProxyPoolSettingsRequest,
};
use codex_pool_core::model::ProxyFailMode;
use control_plane::outbound_proxy_runtime::OutboundProxyRuntime;
use control_plane::store::{ControlPlaneStore, InMemoryStore};

#[tokio::test]
async fn runtime_prefers_enabled_proxy_and_respects_fail_mode() {
    let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
    let runtime = Arc::new(OutboundProxyRuntime::new());
    runtime.attach_store(store.clone());

    let node = store
        .create_outbound_proxy_node(CreateOutboundProxyNodeRequest {
            label: "proxy-a".to_string(),
            proxy_url: "http://127.0.0.1:19080".to_string(),
            enabled: Some(true),
            weight: Some(1),
        })
        .await
        .expect("create proxy node");
    store
        .update_outbound_proxy_pool_settings(UpdateOutboundProxyPoolSettingsRequest {
            enabled: true,
            fail_mode: ProxyFailMode::StrictProxy,
        })
        .await
        .expect("enable proxy pool");

    let selected = runtime
        .select_http_client(Duration::from_secs(3))
        .await
        .expect("proxy selection should succeed");
    assert_eq!(selected.proxy_id, Some(node.id));
    runtime.mark_proxy_transport_failure(&selected).await;

    let strict_err = runtime.select_http_client(Duration::from_secs(3)).await;
    assert!(strict_err.is_err());

    store
        .update_outbound_proxy_pool_settings(UpdateOutboundProxyPoolSettingsRequest {
            enabled: true,
            fail_mode: ProxyFailMode::AllowDirectFallback,
        })
        .await
        .expect("switch to direct fallback");

    let fallback = runtime
        .select_http_client(Duration::from_secs(3))
        .await
        .expect("direct fallback should succeed");
    assert!(fallback.proxy_id.is_none());
    assert!(fallback.used_direct_fallback);
}
