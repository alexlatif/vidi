//! Dashboard lifecycle management - TTL cleanup

use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, error, info};

use crate::AppState;
use crate::storage::DashboardStore;

/// Background task that cleans up expired dashboards
pub async fn cleanup_task(state: Arc<AppState>) {
    let interval = Duration::from_secs(state.config.cleanup_interval);
    info!("Starting cleanup task with interval: {:?}", interval);

    loop {
        tokio::time::sleep(interval).await;
        debug!("Running dashboard cleanup...");

        // Get dashboards with active WebSocket connections
        let active_ids = state.broadcast_hub.active_dashboard_ids();

        match state.store.cleanup_expired(&active_ids).await {
            Ok(count) => {
                if count > 0 {
                    info!("Cleaned up {} expired dashboards", count);
                }
            }
            Err(e) => {
                error!("Cleanup task error: {}", e);
            }
        }
    }
}
