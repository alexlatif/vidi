//! WebSocket streaming handlers

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::AppState;
use crate::models::{ClientMessage, ServerMessage, UpdateCommand};
use crate::storage::DashboardStore;

const CHANNEL_CAPACITY: usize = 256;

/// Hub for managing per-dashboard broadcast channels
pub struct BroadcastHub {
    /// Map of dashboard ID to broadcast sender
    channels: DashMap<Uuid, broadcast::Sender<ServerMessage>>,
    /// Sequence counter per dashboard
    sequences: DashMap<Uuid, AtomicU64>,
    /// Track active connections per dashboard
    connections: DashMap<Uuid, AtomicU64>,
}

impl BroadcastHub {
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
            sequences: DashMap::new(),
            connections: DashMap::new(),
        }
    }

    /// Get or create a broadcast channel for a dashboard
    fn get_or_create_channel(&self, id: Uuid) -> broadcast::Sender<ServerMessage> {
        self.channels
            .entry(id)
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
                self.sequences.insert(id, AtomicU64::new(0));
                self.connections.insert(id, AtomicU64::new(0));
                tx
            })
            .clone()
    }

    /// Subscribe to updates for a dashboard
    pub fn subscribe(&self, id: Uuid) -> broadcast::Receiver<ServerMessage> {
        let sender = self.get_or_create_channel(id);
        if let Some(conn) = self.connections.get(&id) {
            conn.fetch_add(1, Ordering::Relaxed);
        }
        sender.subscribe()
    }

    /// Unsubscribe from a dashboard (decrement connection count)
    pub fn unsubscribe(&self, id: Uuid) {
        if let Some(conn) = self.connections.get(&id) {
            conn.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Get next sequence number for a dashboard
    fn next_seq(&self, id: Uuid) -> u64 {
        self.sequences
            .get(&id)
            .map(|seq| seq.fetch_add(1, Ordering::Relaxed) + 1)
            .unwrap_or(1)
    }

    /// Broadcast an update to all connected clients
    pub fn broadcast(&self, id: Uuid, cmd: UpdateCommand) {
        let seq = self.next_seq(id);
        let msg = cmd.to_server_message(seq);

        if let Some(sender) = self.channels.get(&id) {
            // Ignore errors (no receivers is fine)
            let _ = sender.send(msg);
        }
    }

    /// Remove a dashboard's channel
    pub fn remove_dashboard(&self, id: Uuid) {
        self.channels.remove(&id);
        self.sequences.remove(&id);
        self.connections.remove(&id);
    }

    /// Get list of dashboard IDs with active connections
    pub fn active_dashboard_ids(&self) -> Vec<Uuid> {
        self.connections
            .iter()
            .filter(|entry| entry.value().load(Ordering::Relaxed) > 0)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Get connection count for a dashboard
    pub fn connection_count(&self, id: Uuid) -> u64 {
        self.connections
            .get(&id)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

impl Default for BroadcastHub {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler
pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(state, id, socket))
}

/// Handle a WebSocket connection
async fn handle_socket(state: Arc<AppState>, dashboard_id: Uuid, socket: WebSocket) {
    info!("WebSocket connected for dashboard {}", dashboard_id);

    // Verify dashboard exists
    match state.store.get(dashboard_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            warn!(
                "WebSocket connection for non-existent dashboard {}",
                dashboard_id
            );
            return;
        }
        Err(e) => {
            error!("Database error: {}", e);
            return;
        }
    }

    // Touch to update last_accessed_at
    let _ = state.store.touch(dashboard_id).await;

    // Subscribe to broadcast channel
    let mut rx = state.broadcast_hub.subscribe(dashboard_id);

    // Split socket
    let (mut sender, mut receiver) = socket.split();

    // Send initial connected message
    let connected_msg = ServerMessage::Connected {
        seq: 0,
        dashboard_id: dashboard_id.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    // Spawn task to forward broadcast messages to client
    let forward_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // Handle incoming messages from client
    let state_clone = Arc::clone(&state);
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Sync { last_seq }) => {
                            debug!("Client requesting sync from seq {}", last_seq);
                            // For now, send full refresh on sync request
                            if let Ok(Some(record)) = state_clone.store.get(dashboard_id).await {
                                state_clone.broadcast_hub.broadcast(
                                    dashboard_id,
                                    UpdateCommand::RefreshAll {
                                        dashboard: record.dashboard,
                                    },
                                );
                            }
                        }
                        Ok(ClientMessage::Ack { seq }) => {
                            debug!("Client acked seq {}", seq);
                        }
                        Ok(ClientMessage::GetState) => {
                            debug!("Client requesting state");
                            if let Ok(Some(record)) = state_clone.store.get(dashboard_id).await {
                                state_clone.broadcast_hub.broadcast(
                                    dashboard_id,
                                    UpdateCommand::RefreshAll {
                                        dashboard: record.dashboard,
                                    },
                                );
                            }
                        }
                        Err(e) => {
                            warn!("Invalid client message: {}", e);
                        }
                    }
                }
                Ok(Message::Ping(_)) => {
                    // Handled automatically by axum
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = forward_task => {}
        _ = receive_task => {}
    }

    // Cleanup
    state.broadcast_hub.unsubscribe(dashboard_id);
    info!("WebSocket disconnected for dashboard {}", dashboard_id);
}
