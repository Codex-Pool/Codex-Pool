use std::env;
use std::time::Duration;

use chrono::Utc;
use codex_pool_core::model::{OutboundProxyNode, OutboundProxyPoolSettings, ProxyFailMode};
use data_plane::outbound_proxy_runtime::OutboundProxyRuntime;
use uuid::Uuid;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support;

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

#[tokio::test]
async fn direct_client_bypasses_ambient_http_proxy() {
    let _env_guard = support::lock_env().await;
    let target = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_string("target"))
        .mount(&target)
        .await;

    let fake_proxy = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(503).set_body_string("proxy"))
        .mount(&fake_proxy)
        .await;

    let previous_http_proxy = env::var_os("http_proxy");
    let previous_http_proxy_upper = env::var_os("HTTP_PROXY");
    let previous_https_proxy = env::var_os("https_proxy");
    let previous_https_proxy_upper = env::var_os("HTTPS_PROXY");
    let previous_all_proxy = env::var_os("all_proxy");
    let previous_all_proxy_upper = env::var_os("ALL_PROXY");
    let previous_no_proxy = env::var_os("no_proxy");
    let previous_no_proxy_upper = env::var_os("NO_PROXY");

    unsafe {
        env::set_var("http_proxy", fake_proxy.uri());
        env::set_var("HTTP_PROXY", fake_proxy.uri());
        env::remove_var("DATA_PLANE_ALLOW_SYSTEM_PROXY");
        env::remove_var("https_proxy");
        env::remove_var("HTTPS_PROXY");
        env::remove_var("all_proxy");
        env::remove_var("ALL_PROXY");
        env::remove_var("no_proxy");
        env::remove_var("NO_PROXY");
    }

    let runtime = OutboundProxyRuntime::new();
    runtime.replace_config(OutboundProxyPoolSettings::default(), Vec::new());
    let selected = runtime
        .select_http_client(Some(Duration::from_secs(5)))
        .await
        .expect("direct selection should succeed");
    let response = selected
        .client
        .get(target.uri())
        .send()
        .await
        .expect("request should succeed");
    let status = response.status();
    let body = response.text().await.expect("response body");

    unsafe {
        restore_env("http_proxy", previous_http_proxy);
        restore_env("HTTP_PROXY", previous_http_proxy_upper);
        restore_env("https_proxy", previous_https_proxy);
        restore_env("HTTPS_PROXY", previous_https_proxy_upper);
        restore_env("all_proxy", previous_all_proxy);
        restore_env("ALL_PROXY", previous_all_proxy_upper);
        restore_env("no_proxy", previous_no_proxy);
        restore_env("NO_PROXY", previous_no_proxy_upper);
    }

    assert_eq!(status, reqwest::StatusCode::OK, "unexpected body: {body}");
    assert_eq!(body, "target");
}

#[tokio::test]
async fn direct_client_can_use_ambient_http_proxy_when_enabled() {
    let _env_guard = support::lock_env().await;
    let target = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_string("target"))
        .mount(&target)
        .await;

    let fake_proxy = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(503).set_body_string("proxy"))
        .mount(&fake_proxy)
        .await;

    let previous_http_proxy = env::var_os("http_proxy");
    let previous_http_proxy_upper = env::var_os("HTTP_PROXY");
    let previous_https_proxy = env::var_os("https_proxy");
    let previous_https_proxy_upper = env::var_os("HTTPS_PROXY");
    let previous_all_proxy = env::var_os("all_proxy");
    let previous_all_proxy_upper = env::var_os("ALL_PROXY");
    let previous_no_proxy = env::var_os("no_proxy");
    let previous_no_proxy_upper = env::var_os("NO_PROXY");
    let previous_allow_system_proxy = env::var_os("DATA_PLANE_ALLOW_SYSTEM_PROXY");

    unsafe {
        env::set_var("http_proxy", fake_proxy.uri());
        env::set_var("HTTP_PROXY", fake_proxy.uri());
        env::set_var("DATA_PLANE_ALLOW_SYSTEM_PROXY", "true");
        env::remove_var("https_proxy");
        env::remove_var("HTTPS_PROXY");
        env::remove_var("all_proxy");
        env::remove_var("ALL_PROXY");
        env::remove_var("no_proxy");
        env::remove_var("NO_PROXY");
    }

    let runtime = OutboundProxyRuntime::new();
    runtime.replace_config(OutboundProxyPoolSettings::default(), Vec::new());
    let selected = runtime
        .select_http_client(Some(Duration::from_secs(5)))
        .await
        .expect("direct selection should succeed");
    let response = selected
        .client
        .get(target.uri())
        .send()
        .await
        .expect("request should succeed");
    let status = response.status();
    let body = response.text().await.expect("response body");

    unsafe {
        restore_env("http_proxy", previous_http_proxy);
        restore_env("HTTP_PROXY", previous_http_proxy_upper);
        restore_env("https_proxy", previous_https_proxy);
        restore_env("HTTPS_PROXY", previous_https_proxy_upper);
        restore_env("all_proxy", previous_all_proxy);
        restore_env("ALL_PROXY", previous_all_proxy_upper);
        restore_env("no_proxy", previous_no_proxy);
        restore_env("NO_PROXY", previous_no_proxy_upper);
        restore_env("DATA_PLANE_ALLOW_SYSTEM_PROXY", previous_allow_system_proxy);
    }

    assert_eq!(status, reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, "proxy");
}

unsafe fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => env::set_var(key, value),
        None => env::remove_var(key),
    }
}
