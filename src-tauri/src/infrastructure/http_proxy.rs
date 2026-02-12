//! HTTP proxy for osu! web requests.
//!
//! This module provides an HTTP proxy that intercepts osu! client requests and
//! routes them to either the rai.moe beatmap mirror (for osu!direct functionality)
//! or the official osu! servers (for everything else).
//!
//! # Request Routing
//!
//! Requests are routed based on the host header and URL path:
//!
//! - **osu!direct requests** (search, download, thumbnails) -> `direct.rai.moe`
//! - **All other requests** (login, scores, multiplayer) -> official `*.ppy.sh` servers
//!
//! This selective routing ensures that only beatmap-related traffic goes through
//! the mirror, while sensitive operations remain on official servers.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use parking_lot::RwLock;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::domain::{
    inject_supporter_privileges, map_host_to_ppy, route_request, AppState, Packet, RouteDecision,
    ServerPacketId,
};

/// Runs the HTTP proxy server.
///
/// Listens on the specified port and handles incoming HTTP requests from the
/// osu! client. Each request is analyzed and routed to either rai.moe or
/// the official osu! servers based on the routing rules.
///
/// # Arguments
///
/// * `port` - The local port to listen on (typically 80 or 8080)
/// * `direct_base_url` - Base URL for the rai.moe direct API (e.g., `https://direct.rai.moe`)
/// * `inject_supporter` - If true, modifies Bancho responses to include supporter privileges
/// * `state` - Shared application state for tracking statistics
/// * `shutdown` - Receiver for graceful shutdown signal
/// * `ready_tx` - Optional channel to signal when the server is ready
///
/// # Connection Handling
///
/// Uses hyper's HTTP/1.1 implementation with a connection-pooled reqwest client
/// for upstream requests. The client is configured with:
/// - 10 max idle connections per host
/// - 30 second idle timeout
/// - 30 second request timeout
/// - 10 second connect timeout
///
/// # Returns
///
/// Returns `Ok(())` when the server shuts down gracefully, or an error if
/// binding to the port fails.
pub async fn run_http_proxy(
    port: u16,
    direct_base_url: &str,
    inject_supporter: bool,
    state: Arc<RwLock<AppState>>,
    mut shutdown: oneshot::Receiver<()>,
    ready_tx: Option<oneshot::Sender<()>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        let msg = if e.kind() == std::io::ErrorKind::AddrInUse {
            format!(
                "Port {} is already in use. Please close any application using this port (e.g., IIS, Skype, Docker, or another web server).",
                port
            )
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            format!(
                "Permission denied binding to port {}. Try running as Administrator.",
                port
            )
        } else {
            format!("Failed to bind to port {}: {}", port, e)
        };
        tracing::error!("{}", msg);
        msg
    })?;

    tracing::info!("HTTP proxy listening on {}", addr);

    // Signal that we're ready (port is bound)
    if let Some(tx) = ready_tx {
        let _ = tx.send(());
    }

    let direct_base_url = direct_base_url.to_string();

    // Create a shared HTTP client with connection pooling and timeouts
    let client = Arc::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default(),
    );

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                let io = TokioIo::new(stream);

                let state = Arc::clone(&state);
                let direct_base_url = direct_base_url.clone();
                let client = Arc::clone(&client);

                tokio::spawn(async move {
                    let service = service_fn(move |req| {
                        handle_request(req, direct_base_url.clone(), inject_supporter, Arc::clone(&state), Arc::clone(&client))
                    });

                    if let Err(err) = http1::Builder::new()
                        .serve_connection(io, service)
                        .await
                    {
                        tracing::error!("Connection error: {:?}", err);
                    }
                });
            }
            _ = &mut shutdown => {
                tracing::info!("HTTP proxy shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Handles a single HTTP request from the osu! client.
///
/// Extracts the host and path from the request, determines the routing
/// decision, updates statistics, and forwards the request to the appropriate
/// upstream server.
///
/// # Arguments
///
/// * `req` - The incoming HTTP request
/// * `direct_base_url` - Base URL for rai.moe direct API
/// * `inject_supporter` - Whether to inject supporter privileges in Bancho responses
/// * `state` - Shared application state for statistics
/// * `client` - Shared HTTP client for upstream requests
///
/// # Returns
///
/// Always returns `Ok` with an HTTP response. Errors from upstream servers
/// are converted to 502 Bad Gateway responses.
async fn handle_request(
    req: Request<Incoming>,
    direct_base_url: String,
    inject_supporter: bool,
    state: Arc<RwLock<AppState>>,
    client: Arc<reqwest::Client>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost")
        .to_string();

    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    tracing::debug!("Request: {} {} (host: {})", req.method(), path, &host);

    let decision = route_request(&host, path);

    {
        let mut s = state.write();
        s.requests_proxied += 1;
    }

    let response = match decision {
        RouteDecision::HandleLocally => {
            if path.starts_with("/d/") {
                let mut s = state.write();
                s.beatmaps_downloaded += 1;
            }
            forward_to_raimoe(req, &direct_base_url, &client).await
        }
        RouteDecision::ForwardToPpy => forward_to_ppy(req, &host, inject_supporter, &client).await,
    };

    Ok(response)
}

/// Forwards a request to the rai.moe beatmap mirror.
///
/// Constructs the target URL by appending the request path to the direct
/// base URL and forwards the request with all original headers (except
/// hop-by-hop headers).
///
/// # Arguments
///
/// * `req` - The incoming HTTP request
/// * `direct_base_url` - Base URL for rai.moe (e.g., `https://direct.rai.moe`)
/// * `client` - HTTP client for making the upstream request
///
/// # Returns
///
/// The response from rai.moe, or a 502 Bad Gateway response on failure.
async fn forward_to_raimoe(
    req: Request<Incoming>,
    direct_base_url: &str,
    client: &reqwest::Client,
) -> Response<BoxBody<Bytes, Infallible>> {
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let url = format!("{}{}", direct_base_url.trim_end_matches('/'), path);

    tracing::debug!("Forwarding to rai.moe: {}", url);

    match forward_request(req, &url, client).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Failed to forward to rai.moe: {}", e);
            error_response(StatusCode::BAD_GATEWAY, "Failed to reach rai.moe")
        }
    }
}

/// Forwards a request to the official osu! servers.
///
/// Maps the incoming host to the appropriate `*.ppy.sh` domain and forwards
/// the request over HTTPS. If `inject_supporter` is enabled and this is a
/// Bancho request (to c.ppy.sh), the response body is parsed for UserPrivileges
/// packets and supporter status is injected.
///
/// # Host Mapping
///
/// - `c.*`, `c1.*`, `ce.*` -> `c.ppy.sh` (Bancho)
/// - `a.*` -> `a.ppy.sh` (avatars)
/// - `b.*` -> `b.ppy.sh` (beatmap assets)
/// - `s.*` -> `s.ppy.sh` (spectator)
/// - Everything else -> `osu.ppy.sh`
///
/// # Arguments
///
/// * `req` - The incoming HTTP request
/// * `host` - The original host header value
/// * `inject_supporter` - Whether to inject supporter privileges in Bancho responses
/// * `client` - HTTP client for making the upstream request
///
/// # Returns
///
/// The response from ppy.sh, or a 502 Bad Gateway response on failure.
async fn forward_to_ppy(
    req: Request<Incoming>,
    host: &str,
    inject_supporter: bool,
    client: &reqwest::Client,
) -> Response<BoxBody<Bytes, Infallible>> {
    let ppy_host = map_host_to_ppy(host);
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let url = format!("https://{}{}", ppy_host, path);

    tracing::debug!("Forwarding to ppy.sh: {}", url);

    // Check if this is a Bancho request (c.ppy.sh) that needs supporter injection
    let is_bancho = ppy_host == "c.ppy.sh";

    match forward_request_with_injection(req, &url, client, inject_supporter && is_bancho).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Failed to forward to ppy.sh: {}", e);
            error_response(StatusCode::BAD_GATEWAY, "Failed to reach osu! servers")
        }
    }
}

/// Forwards an HTTP request to the specified URL.
///
/// This is the core forwarding function used by `forward_to_raimoe`.
/// It handles:
///
/// - HTTP method conversion (hyper -> reqwest)
/// - Header forwarding (excluding hop-by-hop headers)
/// - Request body forwarding
/// - Response construction
///
/// # Filtered Headers
///
/// The following headers are not forwarded (they are hop-by-hop or would
/// cause issues with the proxy):
///
/// - `Host` (replaced by target host)
/// - `Connection`, `Keep-Alive`
/// - `Transfer-Encoding`, `TE`, `Trailer`
///
/// # Arguments
///
/// * `req` - The incoming HTTP request
/// * `url` - The full URL to forward to
/// * `client` - HTTP client for making the request
///
/// # Returns
///
/// The upstream response converted to a hyper response, or a reqwest error.
async fn forward_request(
    req: Request<Incoming>,
    url: &str,
    client: &reqwest::Client,
) -> Result<Response<BoxBody<Bytes, Infallible>>, reqwest::Error> {
    forward_request_with_injection(req, url, client, false).await
}

/// Forwards an HTTP request to the specified URL, optionally injecting
/// supporter privileges into Bancho response packets.
///
/// When `inject_supporter` is true, the response body is parsed as Bancho
/// packets and any UserPrivileges packets are modified to include supporter
/// status before being returned to the client.
///
/// # Arguments
///
/// * `req` - The incoming HTTP request
/// * `url` - The full URL to forward to
/// * `client` - HTTP client for making the request
/// * `inject_supporter` - Whether to inject supporter privileges
///
/// # Returns
///
/// The upstream response (possibly modified), or a reqwest error.
async fn forward_request_with_injection(
    req: Request<Incoming>,
    url: &str,
    client: &reqwest::Client,
    inject_supporter: bool,
) -> Result<Response<BoxBody<Bytes, Infallible>>, reqwest::Error> {
    let method = match *req.method() {
        Method::GET => reqwest::Method::GET,
        Method::POST => reqwest::Method::POST,
        Method::PUT => reqwest::Method::PUT,
        Method::DELETE => reqwest::Method::DELETE,
        Method::HEAD => reqwest::Method::HEAD,
        Method::OPTIONS => reqwest::Method::OPTIONS,
        Method::PATCH => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    };

    let mut builder = client.request(method, url);

    for (name, value) in req.headers() {
        let name_str = name.as_str();
        if !matches!(
            name_str.to_lowercase().as_str(),
            "host" | "connection" | "keep-alive" | "transfer-encoding" | "te" | "trailer"
        ) {
            if let Ok(v) = value.to_str() {
                builder = builder.header(name_str, v);
            }
        }
    }

    let body_bytes = req.collect().await.ok().map(|b| b.to_bytes());
    if let Some(bytes) = body_bytes {
        if !bytes.is_empty() {
            builder = builder.body(bytes.to_vec());
        }
    }

    let resp = builder.send().await?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::OK);
    let mut response_builder = Response::builder().status(status);

    for (name, value) in resp.headers() {
        let name_str = name.as_str();
        if !matches!(
            name_str.to_lowercase().as_str(),
            "transfer-encoding" | "connection" | "content-length"
        ) {
            if let Ok(v) = value.to_str() {
                response_builder = response_builder.header(name_str, v);
            }
        }
    }

    let mut body_bytes = resp.bytes().await.unwrap_or_default();

    // If supporter injection is enabled, parse and modify Bancho packets
    if inject_supporter && !body_bytes.is_empty() {
        body_bytes = inject_supporter_into_bancho_response(body_bytes);
    }

    let body = Full::new(body_bytes).map_err(|_| unreachable!()).boxed();

    Ok(response_builder.body(body).unwrap())
}

/// Parses Bancho packets from the response body and injects supporter
/// privileges into any UserPrivileges packets.
///
/// This function:
/// 1. Parses the binary response as a stream of Bancho packets
/// 2. For each UserPrivileges packet (ID 71), modifies the privileges to
///    include supporter status (bit 2)
/// 3. Reassembles the packets into a new response body
///
/// If parsing fails or there are incomplete packets, they are preserved
/// as-is to avoid breaking the client connection.
fn inject_supporter_into_bancho_response(body: Bytes) -> Bytes {
    let (mut packets, remaining) = Packet::parse_stream(&body);

    if packets.is_empty() && remaining.is_empty() {
        // No valid packets found, return original
        return body;
    }

    let mut modified = false;

    // Process each packet
    for packet in &mut packets {
        if packet.packet_type() == ServerPacketId::UserPrivileges {
            tracing::debug!("Injecting supporter privileges into UserPrivileges packet");
            inject_supporter_privileges(packet);
            modified = true;
        }
    }

    if !modified {
        // No modifications needed, return original
        return body;
    }

    // Reassemble packets into response body
    let mut output = Vec::new();
    for packet in packets {
        output.extend(packet.to_bytes());
    }
    // Append any remaining unparsed data (incomplete packets)
    output.extend(remaining);

    Bytes::from(output)
}

/// Creates an error response with the given status code and message.
///
/// Used for returning error responses when upstream requests fail.
///
/// # Arguments
///
/// * `status` - The HTTP status code (typically 502 Bad Gateway)
/// * `message` - Human-readable error message
///
/// # Returns
///
/// An HTTP response with the specified status and plain text body.
fn error_response(status: StatusCode, message: &str) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(
            Full::new(Bytes::from(message.to_string()))
                .map_err(|_| unreachable!())
                .boxed(),
        )
        .unwrap()
}
