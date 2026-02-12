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

use crate::domain::{route_request, AppState, RouteDecision};

pub async fn run_http_proxy(
    port: u16,
    direct_base_url: &str,
    state: Arc<RwLock<AppState>>,
    mut shutdown: oneshot::Receiver<()>,
    ready_tx: Option<oneshot::Sender<()>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

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
                        handle_request(req, direct_base_url.clone(), Arc::clone(&state), Arc::clone(&client))
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

async fn handle_request(
    req: Request<Incoming>,
    direct_base_url: String,
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
        RouteDecision::ForwardToPpy => forward_to_ppy(req, &host, &client).await,
    };

    Ok(response)
}

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

async fn forward_to_ppy(
    req: Request<Incoming>,
    host: &str,
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

    match forward_request(req, &url, client).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Failed to forward to ppy.sh: {}", e);
            error_response(StatusCode::BAD_GATEWAY, "Failed to reach osu! servers")
        }
    }
}

fn map_host_to_ppy(host: &str) -> &'static str {
    let host = host.split(':').next().unwrap_or(host);

    if host.starts_with("c.") || host.starts_with("c1.") || host.starts_with("ce.") {
        "c.ppy.sh"
    } else if host.starts_with("a.") {
        "a.ppy.sh"
    } else if host.starts_with("b.") {
        "b.ppy.sh"
    } else if host.starts_with("s.") {
        "s.ppy.sh"
    } else {
        "osu.ppy.sh"
    }
}

async fn forward_request(
    req: Request<Incoming>,
    url: &str,
    client: &reqwest::Client,
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
            "transfer-encoding" | "connection"
        ) {
            if let Ok(v) = value.to_str() {
                response_builder = response_builder.header(name_str, v);
            }
        }
    }

    let body_bytes = resp.bytes().await.unwrap_or_default();
    let body = Full::new(body_bytes).map_err(|_| unreachable!()).boxed();

    Ok(response_builder.body(body).unwrap())
}

fn error_response(status: StatusCode, message: &str) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(status)
        .body(
            Full::new(Bytes::from(message.to_string()))
                .map_err(|_| unreachable!())
                .boxed(),
        )
        .unwrap()
}
