use std::net::SocketAddr;
use std::sync::Arc;

use parking_lot::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use crate::domain::{inject_supporter_privileges, AppState, Packet, ServerPacketId};

const BANCHO_HOST: &str = "c.ppy.sh";
const BANCHO_PORT: u16 = 13381;

pub async fn run_tcp_proxy(
    port: u16,
    inject_supporter: bool,
    state: Arc<RwLock<AppState>>,
    mut shutdown: oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("TCP proxy (Bancho) listening on {}", addr);

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
        let mut buf = [0u8; 8192];
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
        let mut buf = [0u8; 8192];
        loop {
            match server_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let data = &buf[..n];

                    let modified_data = if inject_supporter {
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
