use std::collections::HashMap;
use std::io;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use codex_pool_core::model::{OutboundProxyNode, OutboundProxyPoolSettings, ProxyFailMode};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::{client::TlsStream, TlsConnector};
use tokio_tungstenite::tungstenite::handshake::client::{
    Request as TungsteniteRequest, Response as TungsteniteResponse,
};
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use tokio_tungstenite::{client_async_tls_with_config, MaybeTlsStream, WebSocketStream};
use url::Url;
use uuid::Uuid;

const DEFAULT_PROXY_TRANSPORT_FAILURE_TTL: Duration = Duration::from_secs(30);
const CONNECT_RESPONSE_MAX_BYTES: usize = 8 * 1024;

pub(crate) trait AsyncIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncIo for T {}

type BoxedAsyncIo = Box<dyn AsyncIo>;
pub(crate) type UpstreamWebSocket = WebSocketStream<MaybeTlsStream<BoxedAsyncIo>>;

#[derive(Clone)]
pub struct SelectedHttpClient {
    pub client: reqwest::Client,
    pub proxy_id: Option<Uuid>,
    pub proxy_label: Option<String>,
    pub proxy_url: Option<String>,
    pub used_direct_fallback: bool,
}

#[derive(Clone)]
pub struct SelectedUpstreamRoute {
    pub proxy_id: Option<Uuid>,
    pub proxy_label: Option<String>,
    pub proxy_url: Option<String>,
    pub used_direct_fallback: bool,
}

pub trait ProxySelectionLike {
    fn proxy_id(&self) -> Option<Uuid>;
}

impl ProxySelectionLike for SelectedHttpClient {
    fn proxy_id(&self) -> Option<Uuid> {
        self.proxy_id
    }
}

impl ProxySelectionLike for SelectedUpstreamRoute {
    fn proxy_id(&self) -> Option<Uuid> {
        self.proxy_id
    }
}

#[derive(Clone, Default)]
pub struct OutboundProxyRuntime {
    settings: Arc<RwLock<OutboundProxyPoolSettings>>,
    nodes: Arc<RwLock<Vec<OutboundProxyNode>>>,
    client_cache: Arc<Mutex<HashMap<String, reqwest::Client>>>,
    sequence: Arc<AtomicU64>,
    unavailable_until: Arc<RwLock<HashMap<Uuid, Instant>>>,
}

impl OutboundProxyRuntime {
    pub fn new() -> Self {
        Self {
            settings: Arc::new(RwLock::new(OutboundProxyPoolSettings::default())),
            nodes: Arc::new(RwLock::new(Vec::new())),
            client_cache: Arc::new(Mutex::new(HashMap::new())),
            sequence: Arc::new(AtomicU64::new(0)),
            unavailable_until: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn replace_config(&self, settings: OutboundProxyPoolSettings, nodes: Vec<OutboundProxyNode>) {
        let allowed_ids = nodes.iter().map(|node| node.id).collect::<std::collections::HashSet<_>>();
        *self.settings.write().expect("outbound proxy settings lock") = settings;
        *self.nodes.write().expect("outbound proxy nodes lock") = nodes;
        self.unavailable_until
            .write()
            .expect("outbound proxy health lock")
            .retain(|proxy_id, _| allowed_ids.contains(proxy_id));
    }

    pub fn current_settings(&self) -> OutboundProxyPoolSettings {
        self.settings
            .read()
            .expect("outbound proxy settings lock")
            .clone()
    }

    pub fn current_nodes(&self) -> Vec<OutboundProxyNode> {
        self.nodes
            .read()
            .expect("outbound proxy nodes lock")
            .clone()
    }

    pub async fn select_http_client(
        &self,
        timeout: Option<Duration>,
    ) -> Result<SelectedHttpClient> {
        let route = self.select_route().await?;
        let client = self
            .cached_client(route.proxy_url.as_deref(), timeout)
            .await?;
        Ok(SelectedHttpClient {
            client,
            proxy_id: route.proxy_id,
            proxy_label: route.proxy_label,
            proxy_url: route.proxy_url,
            used_direct_fallback: route.used_direct_fallback,
        })
    }

    pub(crate) async fn connect_websocket(
        &self,
        request: TungsteniteRequest,
    ) -> Result<(SelectedUpstreamRoute, UpstreamWebSocket, TungsteniteResponse), TungsteniteError>
    {
        let route = self
            .select_route()
            .await
            .map_err(anyhow_to_ws_error)?;
        let result = match route.proxy_url.as_deref() {
            Some(proxy_url) => connect_websocket_via_proxy(proxy_url, request).await,
            None => connect_websocket_direct(request).await,
        };

        match &result {
            Ok(_) => self.mark_proxy_success(&route).await,
            Err(TungsteniteError::Io(_)) => self.mark_proxy_transport_failure(&route).await,
            Err(TungsteniteError::Http(response))
                if response.status().as_u16() == reqwest::StatusCode::PROXY_AUTHENTICATION_REQUIRED.as_u16() =>
            {
                self.mark_proxy_transport_failure(&route).await;
            }
            _ => {}
        }

        result.map(|(socket, response)| (route, socket, response))
    }

    pub async fn mark_proxy_transport_failure<T: ProxySelectionLike + ?Sized>(
        &self,
        selection: &T,
    ) {
        let Some(proxy_id) = selection.proxy_id() else {
            return;
        };
        self.unavailable_until
            .write()
            .expect("outbound proxy health lock")
            .insert(proxy_id, Instant::now() + DEFAULT_PROXY_TRANSPORT_FAILURE_TTL);
    }

    pub async fn mark_proxy_http_status<T: ProxySelectionLike + ?Sized>(
        &self,
        selection: &T,
        status: reqwest::StatusCode,
    ) {
        if status == reqwest::StatusCode::PROXY_AUTHENTICATION_REQUIRED {
            self.mark_proxy_transport_failure(selection).await;
            return;
        }
        self.mark_proxy_success(selection).await;
    }

    pub async fn mark_proxy_success<T: ProxySelectionLike + ?Sized>(&self, selection: &T) {
        let Some(proxy_id) = selection.proxy_id() else {
            return;
        };
        self.unavailable_until
            .write()
            .expect("outbound proxy health lock")
            .remove(&proxy_id);
    }

    async fn select_route(&self) -> Result<SelectedUpstreamRoute> {
        let settings = self
            .settings
            .read()
            .expect("outbound proxy settings lock")
            .clone();
        if !settings.enabled {
            return Ok(SelectedUpstreamRoute {
                proxy_id: None,
                proxy_label: None,
                proxy_url: None,
                used_direct_fallback: false,
            });
        }

        let nodes = self.available_nodes(
            self.nodes
                .read()
                .expect("outbound proxy nodes lock")
                .clone(),
        );
        if let Some(node) = self.pick_weighted(nodes) {
            return Ok(SelectedUpstreamRoute {
                proxy_id: Some(node.id),
                proxy_label: Some(node.label),
                proxy_url: Some(node.proxy_url),
                used_direct_fallback: false,
            });
        }

        if settings.fail_mode == ProxyFailMode::AllowDirectFallback {
            return Ok(SelectedUpstreamRoute {
                proxy_id: None,
                proxy_label: None,
                proxy_url: None,
                used_direct_fallback: true,
            });
        }

        Err(anyhow!("outbound proxy pool has no available proxy"))
    }

    fn available_nodes(&self, nodes: Vec<OutboundProxyNode>) -> Vec<OutboundProxyNode> {
        let now = Instant::now();
        let unavailable = self
            .unavailable_until
            .read()
            .expect("outbound proxy health lock");
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

    async fn cached_client(
        &self,
        proxy_url: Option<&str>,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Client> {
        let timeout_ms = timeout.map(|value| value.as_millis()).unwrap_or(0);
        let cache_key = match proxy_url {
            Some(proxy_url) => format!("proxy:{proxy_url}|timeout_ms:{timeout_ms}"),
            None => format!("direct|timeout_ms:{timeout_ms}"),
        };
        if let Some(client) = self.client_cache.lock().await.get(&cache_key).cloned() {
            return Ok(client);
        }

        let mut builder = reqwest::Client::builder();
        if let Some(timeout) = timeout {
            builder = builder.timeout(timeout);
        }
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

fn anyhow_to_ws_error(err: anyhow::Error) -> TungsteniteError {
    TungsteniteError::Io(io::Error::new(io::ErrorKind::NotConnected, err.to_string()))
}

fn websocket_target_host_port(request: &TungsteniteRequest) -> Result<(String, u16), TungsteniteError> {
    let uri = request.uri();
    let host = uri
        .host()
        .ok_or_else(|| anyhow_to_ws_error(anyhow!("upstream websocket url is missing host")))?;
    let port = uri.port_u16().or_else(|| match uri.scheme_str() {
        Some("wss") => Some(443),
        Some("ws") => Some(80),
        _ => None,
    });
    let port = port.ok_or_else(|| {
        anyhow_to_ws_error(anyhow!("upstream websocket url uses unsupported scheme"))
    })?;
    Ok((host.to_string(), port))
}

async fn connect_websocket_direct(
    request: TungsteniteRequest,
) -> Result<(UpstreamWebSocket, TungsteniteResponse), TungsteniteError> {
    let (host, port) = websocket_target_host_port(&request)?;
    let socket = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(TungsteniteError::Io)?;
    let boxed: BoxedAsyncIo = Box::new(socket);
    client_async_tls_with_config(request, boxed, None, None).await
}

async fn connect_websocket_via_proxy(
    proxy_url: &str,
    request: TungsteniteRequest,
) -> Result<(UpstreamWebSocket, TungsteniteResponse), TungsteniteError> {
    let proxy = ParsedProxyUrl::parse(proxy_url).map_err(anyhow_to_ws_error)?;
    let (target_host, target_port) = websocket_target_host_port(&request)?;
    let tunnel = match proxy.scheme.as_str() {
        "http" => {
            let mut socket = TcpStream::connect((proxy.host.as_str(), proxy.port))
                .await
                .map_err(TungsteniteError::Io)?;
            send_http_connect(&mut socket, &target_host, target_port, proxy.basic_auth.as_deref())
                .await
                .map_err(TungsteniteError::Io)?;
            Box::new(socket) as BoxedAsyncIo
        }
        "https" => {
            let socket = TcpStream::connect((proxy.host.as_str(), proxy.port))
                .await
                .map_err(TungsteniteError::Io)?;
            let mut tls_stream = connect_tls_proxy(socket, &proxy.host)
                .await
                .map_err(TungsteniteError::Io)?;
            send_http_connect(
                &mut tls_stream,
                &target_host,
                target_port,
                proxy.basic_auth.as_deref(),
            )
            .await
            .map_err(TungsteniteError::Io)?;
            Box::new(tls_stream) as BoxedAsyncIo
        }
        "socks5" => {
            let mut socket = TcpStream::connect((proxy.host.as_str(), proxy.port))
                .await
                .map_err(TungsteniteError::Io)?;
            send_socks5_connect(
                &mut socket,
                &target_host,
                target_port,
                proxy.username.as_deref(),
                proxy.password.as_deref(),
            )
            .await
            .map_err(TungsteniteError::Io)?;
            Box::new(socket) as BoxedAsyncIo
        }
        _ => {
            return Err(anyhow_to_ws_error(anyhow!(
                "unsupported proxy scheme {}",
                proxy.scheme
            )));
        }
    };

    client_async_tls_with_config(request, tunnel, None, None).await
}

struct ParsedProxyUrl {
    scheme: String,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    basic_auth: Option<String>,
}

impl ParsedProxyUrl {
    fn parse(raw: &str) -> Result<Self> {
        let url = Url::parse(raw).with_context(|| format!("invalid outbound proxy url: {raw}"))?;
        let host = url
            .host_str()
            .map(ToString::to_string)
            .ok_or_else(|| anyhow!("outbound proxy url is missing host"))?;
        let port = url
            .port()
            .ok_or_else(|| anyhow!("outbound proxy url is missing port"))?;
        let username = (!url.username().is_empty()).then(|| url.username().to_string());
        let password = url.password().map(ToString::to_string);
        let basic_auth = username.as_ref().map(|username| {
            let raw = format!("{}:{}", username, password.as_deref().unwrap_or_default());
            format!("Basic {}", BASE64_STANDARD.encode(raw))
        });
        Ok(Self {
            scheme: url.scheme().to_string(),
            host,
            port,
            username,
            password,
            basic_auth,
        })
    }
}

async fn send_http_connect<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    basic_auth: Option<&str>,
) -> io::Result<()> {
    let mut request = format!(
        "CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\n"
    );
    if let Some(basic_auth) = basic_auth {
        request.push_str("Proxy-Authorization: ");
        request.push_str(basic_auth);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;

    let mut buffer = Vec::with_capacity(1024);
    loop {
        if buffer.len() >= CONNECT_RESPONSE_MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "proxy connect response exceeded limit",
            ));
        }
        let mut chunk = [0_u8; 512];
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "proxy closed during connect handshake",
            ));
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let response_text = String::from_utf8_lossy(&buffer);
    let status_line = response_text.lines().next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|raw| raw.parse::<u16>().ok())
        .unwrap_or_default();
    if status == 200 {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("proxy connect failed with status {status}"),
    ))
}

async fn send_socks5_connect<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    target_host: &str,
    target_port: u16,
    username: Option<&str>,
    password: Option<&str>,
) -> io::Result<()> {
    let mut methods = vec![0x00_u8];
    if username.is_some() || password.is_some() {
        methods.push(0x02);
    }
    let greeting = [vec![0x05, methods.len() as u8], methods].concat();
    stream.write_all(&greeting).await?;
    stream.flush().await?;

    let mut method_response = [0_u8; 2];
    stream.read_exact(&mut method_response).await?;
    if method_response[0] != 0x05 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid socks5 greeting response",
        ));
    }
    match method_response[1] {
        0x00 => {}
        0x02 => {
            let username = username.unwrap_or_default().as_bytes();
            let password = password.unwrap_or_default().as_bytes();
            if username.len() > 255 || password.len() > 255 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "socks5 auth credential is too long",
                ));
            }
            let mut auth = vec![0x01, username.len() as u8];
            auth.extend_from_slice(username);
            auth.push(password.len() as u8);
            auth.extend_from_slice(password);
            stream.write_all(&auth).await?;
            stream.flush().await?;

            let mut auth_response = [0_u8; 2];
            stream.read_exact(&mut auth_response).await?;
            if auth_response != [0x01, 0x00] {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "socks5 username/password authentication failed",
                ));
            }
        }
        0xFF => {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "socks5 proxy rejected all auth methods",
            ));
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported socks5 auth method",
            ));
        }
    }

    let mut request = vec![0x05, 0x01, 0x00];
    match target_host.parse::<IpAddr>() {
        Ok(IpAddr::V4(addr)) => {
            request.push(0x01);
            request.extend_from_slice(&addr.octets());
        }
        Ok(IpAddr::V6(addr)) => {
            request.push(0x04);
            request.extend_from_slice(&addr.octets());
        }
        Err(_) => {
            let host_bytes = target_host.as_bytes();
            if host_bytes.len() > 255 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "socks5 target host is too long",
                ));
            }
            request.push(0x03);
            request.push(host_bytes.len() as u8);
            request.extend_from_slice(host_bytes);
        }
    }
    request.extend_from_slice(&target_port.to_be_bytes());
    stream.write_all(&request).await?;
    stream.flush().await?;

    let mut header = [0_u8; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != 0x05 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid socks5 connect response",
        ));
    }
    if header[1] != 0x00 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("socks5 connect failed with code {}", header[1]),
        ));
    }
    match header[3] {
        0x01 => {
            let mut buf = [0_u8; 4];
            stream.read_exact(&mut buf).await?;
        }
        0x03 => {
            let mut len = [0_u8; 1];
            stream.read_exact(&mut len).await?;
            let mut buf = vec![0_u8; len[0] as usize];
            stream.read_exact(&mut buf).await?;
        }
        0x04 => {
            let mut buf = [0_u8; 16];
            stream.read_exact(&mut buf).await?;
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid socks5 bind address type",
            ));
        }
    }
    let mut port = [0_u8; 2];
    stream.read_exact(&mut port).await?;
    Ok(())
}

async fn connect_tls_proxy(socket: TcpStream, proxy_host: &str) -> io::Result<TlsStream<TcpStream>> {
    let mut roots = RootCertStore::empty();
    let rustls_native_certs::CertificateResult { certs, errors, .. } =
        rustls_native_certs::load_native_certs();
    if !errors.is_empty() {
        tracing::warn!(errors = ?errors, "encountered native cert load errors for https proxy");
    }
    let _ = roots.add_parsable_certificates(certs);
    let config = Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    );
    let server_name = ServerName::try_from(proxy_host.to_string())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid proxy tls host"))?;
    TlsConnector::from(config)
        .connect(server_name, socket)
        .await
        .map_err(io::Error::other)
}
