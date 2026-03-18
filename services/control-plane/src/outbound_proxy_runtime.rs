use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use codex_pool_core::model::{OutboundProxyNode, ProxyFailMode};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::store::ControlPlaneStore;

const DEFAULT_PROXY_TRANSPORT_FAILURE_TTL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct SelectedHttpClient {
    pub client: reqwest::Client,
    pub proxy_id: Option<Uuid>,
    pub proxy_label: Option<String>,
    pub proxy_url: Option<String>,
    pub used_direct_fallback: bool,
}

#[derive(Clone, Default)]
pub struct OutboundProxyRuntime {
    store: Arc<RwLock<Option<Arc<dyn ControlPlaneStore>>>>,
    client_cache: Arc<Mutex<HashMap<String, reqwest::Client>>>,
    sequence: Arc<AtomicU64>,
    unavailable_until: Arc<RwLock<HashMap<Uuid, Instant>>>,
}

impl OutboundProxyRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attach_store(&self, store: Arc<dyn ControlPlaneStore>) {
        *self.store.write().expect("outbound proxy runtime store lock") = Some(store);
    }

    pub async fn select_http_client(&self, timeout: Duration) -> Result<SelectedHttpClient> {
        let Some(store) = self
            .store
            .read()
            .expect("outbound proxy runtime store lock")
            .clone()
        else {
            return self.direct_client(timeout, false).await;
        };

        let settings = store.outbound_proxy_pool_settings().await?;
        if !settings.enabled {
            return self.direct_client(timeout, false).await;
        }

        let nodes = store.list_outbound_proxy_nodes().await?;
        let candidates = self.available_nodes(nodes);
        if let Some(node) = self.pick_weighted(candidates) {
            return self.proxy_client(&node, timeout, false).await;
        }

        if settings.fail_mode == ProxyFailMode::AllowDirectFallback {
            return self.direct_client(timeout, true).await;
        }

        Err(anyhow!("outbound proxy pool has no available proxy"))
    }

    pub async fn mark_proxy_transport_failure(&self, selection: &SelectedHttpClient) {
        let Some(proxy_id) = selection.proxy_id else {
            return;
        };
        self.unavailable_until
            .write()
            .expect("outbound proxy runtime health lock")
            .insert(proxy_id, Instant::now() + DEFAULT_PROXY_TRANSPORT_FAILURE_TTL);
    }

    pub async fn mark_proxy_http_status(
        &self,
        selection: &SelectedHttpClient,
        status: reqwest::StatusCode,
    ) {
        let Some(proxy_id) = selection.proxy_id else {
            return;
        };
        if status == reqwest::StatusCode::PROXY_AUTHENTICATION_REQUIRED {
            self.mark_proxy_transport_failure(selection).await;
            return;
        }
        self.unavailable_until
            .write()
            .expect("outbound proxy runtime health lock")
            .remove(&proxy_id);
    }

    fn available_nodes(&self, nodes: Vec<OutboundProxyNode>) -> Vec<OutboundProxyNode> {
        let now = Instant::now();
        let unavailable = self
            .unavailable_until
            .read()
            .expect("outbound proxy runtime health lock");
        nodes.into_iter()
            .filter(|node| {
                node.enabled
                    && node.weight > 0
                    && unavailable
                        .get(&node.id)
                        .is_none_or(|until| *until <= now)
            })
            .collect()
    }

    fn pick_weighted(&self, nodes: Vec<OutboundProxyNode>) -> Option<OutboundProxyNode> {
        if nodes.is_empty() {
            return None;
        }
        let total_weight = nodes
            .iter()
            .map(|node| u64::from(node.weight.max(1)))
            .sum::<u64>()
            .max(1);
        let slot = self.sequence.fetch_add(1, Ordering::Relaxed) % total_weight;
        let mut cursor = 0_u64;
        for node in nodes {
            cursor = cursor.saturating_add(u64::from(node.weight.max(1)));
            if slot < cursor {
                return Some(node);
            }
        }
        None
    }

    async fn direct_client(
        &self,
        timeout: Duration,
        used_direct_fallback: bool,
    ) -> Result<SelectedHttpClient> {
        Ok(SelectedHttpClient {
            client: self.cached_client(None, timeout).await?,
            proxy_id: None,
            proxy_label: None,
            proxy_url: None,
            used_direct_fallback,
        })
    }

    async fn proxy_client(
        &self,
        node: &OutboundProxyNode,
        timeout: Duration,
        used_direct_fallback: bool,
    ) -> Result<SelectedHttpClient> {
        Ok(SelectedHttpClient {
            client: self.cached_client(Some(node.proxy_url.as_str()), timeout).await?,
            proxy_id: Some(node.id),
            proxy_label: Some(node.label.clone()),
            proxy_url: Some(node.proxy_url.clone()),
            used_direct_fallback,
        })
    }

    async fn cached_client(
        &self,
        proxy_url: Option<&str>,
        timeout: Duration,
    ) -> Result<reqwest::Client> {
        let timeout_ms = timeout.as_millis();
        let cache_key = match proxy_url {
            Some(proxy_url) => format!("proxy:{proxy_url}|timeout_ms:{timeout_ms}"),
            None => format!("direct|timeout_ms:{timeout_ms}"),
        };
        if let Some(client) = self.client_cache.lock().await.get(&cache_key).cloned() {
            return Ok(client);
        }

        let mut builder = reqwest::Client::builder().timeout(timeout);
        if let Some(proxy_url) = proxy_url {
            let proxy = reqwest::Proxy::all(proxy_url)
                .with_context(|| format!("failed to configure outbound proxy client for {proxy_url}"))?;
            builder = builder.proxy(proxy);
        }
        let client = builder
            .build()
            .context("failed to build outbound proxy runtime client")?;
        self.client_cache
            .lock()
            .await
            .insert(cache_key, client.clone());
        Ok(client)
    }
}
