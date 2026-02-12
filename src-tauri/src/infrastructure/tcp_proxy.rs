//! TCP proxy for Bancho (osu!'s game server) connections.
//!
//! This module provides a transparent TCP proxy that sits between the osu! client
//! and the official Bancho server. It can optionally inject supporter privileges
//! into the packet stream to enable osu!direct functionality.

use std::net::SocketAddr;
use std::sync::Arc;

use parking_lot::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use crate::domain::{inject_supporter_privileges, AppState, Packet, ServerPacketId};

/// The hostname of the official Bancho server.
const BANCHO_HOST: &str = "c.ppy.sh";

/// The port number for Bancho's IRC/game protocol.
const BANCHO_PORT: u16 = 13381;

/// Maximum buffer size for packet reassembly (1MB).
///
/// This limit prevents memory exhaustion from malformed packets or
/// intentionally large payloads. If the buffer exceeds this size,
/// the connection is terminated.
const MAX_BUFFER_SIZE: usize = 1_048_576; // 1MB

/// Runs the TCP proxy server for Bancho connections.
///
/// This proxy listens on the specified port and forwards all traffic between
/// the osu! client and the official Bancho server at `c.ppy.sh:13381`.
///
/// # Arguments
///
/// * `port` - The local port to listen on (typically 13381)
/// * `inject_supporter` - If true, modifies `UserPrivileges` packets to include
///   supporter status, enabling osu!direct in the client
/// * `state` - Shared application state for tracking statistics
/// * `shutdown` - Receiver for graceful shutdown signal
/// * `ready_tx` - Optional channel to signal when the server is ready (port bound)
///
/// # Returns
///
/// Returns `Ok(())` when the server shuts down gracefully, or an error if
/// binding to the port fails.
///
/// # Example
///
/// ```ignore
/// let state = Arc::new(RwLock::new(AppState::default()));
/// let (shutdown_tx, shutdown_rx) = oneshot::channel();
/// let (ready_tx, ready_rx) = oneshot::channel();
///
/// tokio::spawn(run_tcp_proxy(13381, true, state, shutdown_rx, Some(ready_tx)));
///
/// // Wait for server to be ready
/// ready_rx.await.unwrap();
/// ```
pub async fn run_tcp_proxy(
    port: u16,
    inject_supporter: bool,
    state: Arc<RwLock<AppState>>,
    mut shutdown: oneshot::Receiver<()>,
    ready_tx: Option<oneshot::Sender<()>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("TCP proxy (Bancho) listening on {}", addr);

    // Signal that we're ready (port is bound)
    if let Some(tx) = ready_tx {
        let _ = tx.send(());
    }

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (client_stream, client_addr) = result?;
                tracing::info!("New Bancho connection from {}", client_addr);

                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_bancho_connection(client_stream, inject_supporter, state).await {
                        tracing::error!("Bancho connection error: {}", e);
                    }
                });
            }
            _ = &mut shutdown => {
                tracing::info!("TCP proxy shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Handles a single Bancho client connection.
///
/// This function establishes a bidirectional proxy between the osu! client
/// and the official Bancho server. Data flows in both directions simultaneously:
///
/// - **Client -> Server**: All packets forwarded unchanged
/// - **Server -> Client**: Packets are parsed and `UserPrivileges` packets are
///   modified to include supporter status (if `inject_supporter` is true)
///
/// # Packet Processing
///
/// When `inject_supporter` is enabled, incoming server data is buffered and
/// parsed as Bancho packets. This is necessary because TCP doesn't preserve
/// message boundaries, so packets may arrive fragmented across multiple reads.
/// The buffer accumulates data until complete packets can be extracted.
///
/// # Arguments
///
/// * `client` - The TCP stream from the osu! client
/// * `inject_supporter` - Whether to inject supporter privileges
/// * `_state` - Shared application state (currently unused, reserved for future metrics)
///
/// # Returns
///
/// Returns `Ok(())` when either side closes the connection, or an error if
/// the connection to Bancho fails.
async fn handle_bancho_connection(
    mut client: TcpStream,
    inject_supporter: bool,
    _state: Arc<RwLock<AppState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Connect to official Bancho server
    let bancho_addr = format!("{}:{}", BANCHO_HOST, BANCHO_PORT);
    let mut server = TcpStream::connect(&bancho_addr).await?;

    tracing::debug!("Connected to official Bancho at {}", bancho_addr);

    let (mut client_read, mut client_write) = client.split();
    let (mut server_read, mut server_write) = server.split();

    let mut server_buffer = Vec::new();

    let client_to_server = async {
        let mut buf = [0u8; 32768];
        loop {
            match client_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if let Err(e) = server_write.write_all(&buf[..n]).await {
                        tracing::error!("Failed to write to server: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read from client: {}", e);
                    break;
                }
            }
        }
    };

    let server_to_client = async {
        let mut buf = [0u8; 32768];
        loop {
            match server_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let data = &buf[..n];

                    let modified_data = if inject_supporter {
                        if server_buffer.len() + data.len() > MAX_BUFFER_SIZE {
                            tracing::error!(
                                "Server buffer size limit exceeded ({} + {} > {}), disconnecting",
                                server_buffer.len(),
                                data.len(),
                                MAX_BUFFER_SIZE
                            );
                            break;
                        }
                        server_buffer.extend_from_slice(data);
                        let (packets, remaining) = Packet::parse_stream(&server_buffer);
                        server_buffer = remaining;

                        let mut output = Vec::new();
                        for mut packet in packets {
                            if packet.packet_type() == ServerPacketId::UserPrivileges {
                                tracing::debug!("Injecting supporter privileges");
                                inject_supporter_privileges(&mut packet);
                            }
                            output.extend(packet.to_bytes());
                        }

                        if output.is_empty() && !server_buffer.is_empty() {
                            continue;
                        }
                        output
                    } else {
                        data.to_vec()
                    };

                    if !modified_data.is_empty() {
                        if let Err(e) = client_write.write_all(&modified_data).await {
                            tracing::error!("Failed to write to client: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read from server: {}", e);
                    break;
                }
            }
        }
    };

    tokio::select! {
        _ = client_to_server => {
            tracing::debug!("Client disconnected");
        }
        _ = server_to_client => {
            tracing::debug!("Server disconnected");
        }
    }

    Ok(())
}
