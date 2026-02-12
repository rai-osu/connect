use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::oneshot;

use crate::domain::{AppState, ConnectionStatus, ProxyConfig};
use crate::infrastructure::{hosts, tls};

pub struct ProxyManager {
    state: Arc<RwLock<AppState>>,
    http_shutdown: Option<oneshot::Sender<()>>,
    config: ProxyConfig,
}

impl ProxyManager {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(AppState::default())),
            http_shutdown: None,
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

        // Ensure certificate is installed before starting proxy
        if !tls::is_certificate_installed() {
            tracing::info!("Certificate not installed, installing now...");
            match tls::install_certificate() {
                Ok(true) => tracing::info!("Certificate installed successfully"),
                Ok(false) => tracing::info!("Certificate was already installed"),
                Err(e) => {
                    tracing::warn!("Failed to auto-install certificate: {}. You may need to install it manually.", e);
                }
            }
        }

        // Ensure hosts file entries exist for *.localhost resolution
        if !hosts::are_hosts_entries_present() {
            tracing::info!("Hosts entries not present, adding now...");
            match hosts::add_hosts_entries() {
                Ok(true) => tracing::info!("Hosts entries added successfully"),
                Ok(false) => tracing::info!("Hosts entries were already present"),
                Err(e) => {
                    tracing::warn!(
                        "Failed to add hosts entries: {}. You may need to add them manually.",
                        e
                    );
                }
            }
        }

        let (http_tx, http_rx) = oneshot::channel();

        // Create ready channel to verify port is bound
        let (http_ready_tx, http_ready_rx) = oneshot::channel();

        self.http_shutdown = Some(http_tx);

        let https_state = Arc::clone(&self.state);
        let https_config = self.config.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::infrastructure::http_proxy::run_https_proxy(
                https_config.https_port,
                &https_config.direct_base_url,
                https_config.inject_supporter,
                https_state,
                http_rx,
                Some(http_ready_tx),
            )
            .await
            {
                tracing::error!("HTTPS proxy error: {}", e);
            }
        });

        // Wait for HTTPS proxy to be ready (with timeout)
        let timeout = std::time::Duration::from_secs(5);
        match tokio::time::timeout(timeout, http_ready_rx).await {
            Ok(Ok(())) => {
                let mut state = self.state.write();
                state.status = ConnectionStatus::Connected;
                tracing::info!("HTTPS proxy started on port {}", self.config.https_port);
                Ok(())
            }
            _ => {
                // Cleanup on failure
                if let Some(tx) = self.http_shutdown.take() {
                    let _ = tx.send(());
                }
                let mut state = self.state.write();
                state.status = ConnectionStatus::Error;
                state.last_error = Some("Failed to start proxy: port binding timeout".to_string());
                Err("Failed to start proxy: port binding timeout".to_string())
            }
        }
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(tx) = self.http_shutdown.take() {
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
