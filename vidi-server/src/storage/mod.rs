//! Dashboard storage backends

pub mod memory;
pub mod sqlite;

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;
use crate::models::{DashboardRecord, DashboardSummary, ListQuery, UpdateMetaRequest, WasmStatus};

/// Trait for dashboard storage backends
#[async_trait]
pub trait DashboardStore: Send + Sync {
    /// Create a new dashboard
    async fn create(&self, record: DashboardRecord) -> Result<DashboardRecord>;

    /// Get a dashboard by ID
    async fn get(&self, id: Uuid) -> Result<Option<DashboardRecord>>;

    /// List dashboards with optional filters
    async fn list(&self, query: &ListQuery) -> Result<Vec<DashboardSummary>>;

    /// Replace dashboard data (full update)
    async fn replace(&self, id: Uuid, record: DashboardRecord) -> Result<DashboardRecord>;

    /// Update dashboard metadata only
    async fn update_meta(&self, id: Uuid, update: UpdateMetaRequest) -> Result<DashboardRecord>;

    /// Delete a dashboard
    async fn delete(&self, id: Uuid) -> Result<bool>;

    /// Touch a dashboard (update last_accessed_at, extend TTL)
    async fn touch(&self, id: Uuid) -> Result<()>;

    /// Cleanup expired dashboards, skipping those with active connections
    async fn cleanup_expired(&self, active_ids: &[Uuid]) -> Result<u64>;

    /// Update WASM compilation status
    async fn update_wasm_status(
        &self,
        id: Uuid,
        status: WasmStatus,
        error: Option<String>,
    ) -> Result<()>;

    /// Get dashboard JSON (for WASM compilation)
    async fn get_dashboard_json(&self, id: Uuid) -> Result<Option<String>>;
}
