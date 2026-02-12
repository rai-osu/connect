use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::oneshot;

use crate::domain::{AppState, ConnectionStatus, ProxyConfig};

pub struct ProxyManager {
    state: Arc<RwLock<AppState>>,
    http_shutdown: Option<oneshot::Sender<()>>,
    tcp_shutdown: Option<oneshot::Sender<()>>,
    config: ProxyConfig,
}

impl ProxyManager {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(AppState::default())),
            http_shutdown: None,
            tcp_shutdown: None,
            config,
        }
    }

    pub fn state(&self) -> Arc<RwLock<AppState>> {
        Arc::clone(&self.state)
    }

    pub fn status(&self) -> ConnectionStatus {
        self.state.read().status
    }

    pub async fn start(&mut self) -> Result<(), String> {
        if self.status() == ConnectionStatus::Connected {
            return Ok(());
        }

        {
            let mut state = self.state.write();
            state.status = ConnectionStatus::Connecting;
            state.last_error = None;
        }

        let (http_tx, http_rx) = oneshot::channel();
        let (tcp_tx, tcp_rx) = oneshot::channel();

        self.http_shutdown = Some(http_tx);
        self.tcp_shutdown = Some(tcp_tx);

        let http_state = Arc::clone(&self.state);
        let http_config = self.config.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::infrastructure::http_proxy::run_http_proxy(
                http_config.http_port,
                &http_config.direct_base_url,
                http_state,
                http_rx,
            )
            .await
            {
                tracing::error!("HTTP proxy error: {}", e);
            }
        });

        let tcp_state = Arc::clone(&self.state);
        let tcp_config = self.config.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::infrastructure::tcp_proxy::run_tcp_proxy(
                tcp_config.bancho_port,
                tcp_config.inject_supporter,
                tcp_state,
                tcp_rx,
            )
            .await
            {
                tracing::error!("TCP proxy error: {}", e);
            }
        });

        {
            let mut state = self.state.write();
            state.status = ConnectionStatus::Connected;
        }

        tracing::info!(
            "Proxy started - HTTP: {}, Bancho: {}",
            self.config.http_port,
            self.config.bancho_port
        );

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(tx) = self.http_shutdown.take() {
            let _ = tx.send(());
        }

        if let Some(tx) = self.tcp_shutdown.take() {
            let _ = tx.send(());
        }

        {
            let mut state = self.state.write();
            state.status = ConnectionStatus::Disconnected;
        }

        tracing::info!("Proxy stopped");

        Ok(())
    }

    pub fn increment_requests(&self) {
        let mut state = self.state.write();
        state.requests_proxied += 1;
    }

    pub fn increment_downloads(&self) {
        let mut state = self.state.write();
        state.beatmaps_downloaded += 1;
    }

    pub fn set_error(&self, error: String) {
        let mut state = self.state.write();
        state.status = ConnectionStatus::Error;
        state.last_error = Some(error);
    }
}

impl Default for ProxyManager {
    fn default() -> Self {
        Self::new(ProxyConfig::default())
    }
}
