use crate::backend::BluetoothBackend;
use crate::config::Config;
use crate::error::{BtError, Result};
use crate::protocol::{AgentMessage, HandoffStatus};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};

pub struct Agent {
    backend: Arc<Box<dyn BluetoothBackend>>,
    config: Arc<Config>,
}

impl Agent {
    pub fn new(backend: Box<dyn BluetoothBackend>, config: Config) -> Self {
        Self {
            backend: Arc::new(backend),
            config: Arc::new(config),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.config.agent_port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| BtError::Backend(format!("cannot bind on {addr}: {e}")))?;
        info!("Agent listening on {addr}");

        let backend = self.backend.clone();
        let config = self.config.clone();

        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    info!("Connection from {peer}");
                    let backend = backend.clone();
                    let config = config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, peer, backend, config).await {
                            error!("Connection handler error from {peer}: {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {e}");
                }
            }
        }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    peer: SocketAddr,
    backend: Arc<Box<dyn BluetoothBackend>>,
    config: Arc<Config>,
) -> Result<()> {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    buf_reader
        .read_line(&mut line)
        .await
        .map_err(|e| BtError::Protocol(format!("read from {peer}: {e}")))?;

    let msg: AgentMessage = serde_json::from_str(line.trim())
        .map_err(|e| BtError::Protocol(format!("invalid message from {peer}: {e}")))?;

    match msg {
        AgentMessage::HandoffRequest {
            device_name,
            bt_address,
            from_host,
        } => {
            info!(
                "Handoff request from {from_host}: {device_name} ({bt_address})"
            );
            match backend.connect(&bt_address) {
                Ok(()) => {
                    let ack = AgentMessage::HandoffAck {
                        device_name,
                        status: HandoffStatus::Connected,
                        message: format!("connected on {}", config.identity.hostname),
                    };
                    let mut resp = serde_json::to_string(&ack)?;
                    resp.push('\n');
                    writer.write_all(resp.as_bytes()).await?;
                }
                Err(e) => {
                    warn!("Failed to connect {bt_address}: {e}");
                    let ack = AgentMessage::HandoffAck {
                        device_name,
                        status: HandoffStatus::Failed(e.to_string()),
                        message: String::new(),
                    };
                    let mut resp = serde_json::to_string(&ack)?;
                    resp.push('\n');
                    let _ = writer.write_all(resp.as_bytes()).await;
                }
            }
        }
        AgentMessage::Ping { hostname: _, version } => {
            let pong = AgentMessage::Pong {
                hostname: config.identity.hostname.clone(),
                version: version,
            };
            let mut resp = serde_json::to_string(&pong)?;
            resp.push('\n');
            writer.write_all(resp.as_bytes()).await?;
        }
        _ => {
            return Err(BtError::Protocol(format!(
                "unexpected message type from {peer}"
            )));
        }
    }

    Ok(())
}
