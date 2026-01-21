//! WebSocket message types for real-time streaming

use serde::{Deserialize, Serialize};
use vidi::prelude::{Dashboard, Plot};

/// Messages sent from server to clients
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Append points to an existing layer
    AppendPoints {
        seq: u64,
        plot_id: u64,
        layer_idx: usize,
        /// Flattened [x1, y1, x2, y2, ...] for 2D or [x1, y1, z1, ...] for 3D
        points: Vec<f32>,
    },

    /// Replace all points in a layer
    ReplaceTrace {
        seq: u64,
        plot_id: u64,
        layer_idx: usize,
        points: Vec<f32>,
    },

    /// Update an entire plot
    UpdatePlot { seq: u64, plot_id: u64, plot: Plot },

    /// Full dashboard refresh
    RefreshAll { seq: u64, dashboard: Dashboard },

    /// Error message
    Error { seq: u64, message: String },

    /// Connection established
    Connected { seq: u64, dashboard_id: String },
}

/// Messages sent from clients to server
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Request sync from a specific sequence number (reconnection recovery)
    Sync { last_seq: u64 },

    /// Acknowledge receipt of a message
    Ack { seq: u64 },

    /// Client requests current state
    GetState,
}

/// Update command used internally and via REST API to push updates
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UpdateCommand {
    /// Append points to a 2D layer
    AppendPoints2D {
        plot_id: u64,
        layer_idx: usize,
        points: Vec<[f32; 2]>,
    },

    /// Append points to a 3D layer
    AppendPoints3D {
        plot_id: u64,
        layer_idx: usize,
        points: Vec<[f32; 3]>,
    },

    /// Replace all points in a 2D layer
    ReplaceTrace2D {
        plot_id: u64,
        layer_idx: usize,
        points: Vec<[f32; 2]>,
    },

    /// Replace all points in a 3D layer
    ReplaceTrace3D {
        plot_id: u64,
        layer_idx: usize,
        points: Vec<[f32; 3]>,
    },

    /// Update an entire plot
    UpdatePlot { plot_id: u64, plot: Plot },

    /// Replace the entire dashboard
    RefreshAll { dashboard: Dashboard },
}

impl UpdateCommand {
    /// Convert to a server message with sequence number
    pub fn to_server_message(&self, seq: u64) -> ServerMessage {
        match self {
            UpdateCommand::AppendPoints2D {
                plot_id,
                layer_idx,
                points,
            } => ServerMessage::AppendPoints {
                seq,
                plot_id: *plot_id,
                layer_idx: *layer_idx,
                points: points.iter().flat_map(|p| [p[0], p[1]]).collect(),
            },
            UpdateCommand::AppendPoints3D {
                plot_id,
                layer_idx,
                points,
            } => ServerMessage::AppendPoints {
                seq,
                plot_id: *plot_id,
                layer_idx: *layer_idx,
                points: points.iter().flat_map(|p| [p[0], p[1], p[2]]).collect(),
            },
            UpdateCommand::ReplaceTrace2D {
                plot_id,
                layer_idx,
                points,
            } => ServerMessage::ReplaceTrace {
                seq,
                plot_id: *plot_id,
                layer_idx: *layer_idx,
                points: points.iter().flat_map(|p| [p[0], p[1]]).collect(),
            },
            UpdateCommand::ReplaceTrace3D {
                plot_id,
                layer_idx,
                points,
            } => ServerMessage::ReplaceTrace {
                seq,
                plot_id: *plot_id,
                layer_idx: *layer_idx,
                points: points.iter().flat_map(|p| [p[0], p[1], p[2]]).collect(),
            },
            UpdateCommand::UpdatePlot { plot_id, plot } => ServerMessage::UpdatePlot {
                seq,
                plot_id: *plot_id,
                plot: plot.clone(),
            },
            UpdateCommand::RefreshAll { dashboard } => ServerMessage::RefreshAll {
                seq,
                dashboard: dashboard.clone(),
            },
        }
    }
}
