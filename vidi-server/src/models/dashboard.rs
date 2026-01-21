//! Dashboard metadata and record models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vidi::prelude::Dashboard;

/// Status of WASM compilation for a dashboard
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WasmStatus {
    /// Compilation not yet started
    #[default]
    Pending,
    /// Currently compiling
    Compiling,
    /// Compilation successful, WASM ready
    Ready,
    /// Compilation failed
    Failed,
}

impl WasmStatus {
    pub fn as_str(&self) -> &str {
        match self {
            WasmStatus::Pending => "pending",
            WasmStatus::Compiling => "compiling",
            WasmStatus::Ready => "ready",
            WasmStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => WasmStatus::Pending,
            "compiling" => WasmStatus::Compiling,
            "ready" => WasmStatus::Ready,
            "failed" => WasmStatus::Failed,
            _ => WasmStatus::Pending,
        }
    }
}

/// Dashboard metadata stored in the database
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardMeta {
    /// Unique identifier
    pub id: Uuid,

    /// Experiment name for grouping related dashboards
    pub xp_name: Option<String>,

    /// User who created the dashboard
    pub user: Option<String>,

    /// Tags for categorization and filtering
    pub tags: Vec<String>,

    /// Whether this dashboard is permanent (never expires)
    pub permanent: bool,

    /// Time-to-live in seconds (for temporary dashboards)
    pub ttl: Option<u64>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Last access timestamp (for TTL calculation)
    pub last_accessed_at: DateTime<Utc>,

    /// WASM compilation status
    #[serde(default)]
    pub wasm_status: WasmStatus,

    /// WASM compilation error message (if failed)
    #[serde(default)]
    pub wasm_error: Option<String>,
}

impl DashboardMeta {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            xp_name: None,
            user: None,
            tags: vec![],
            permanent: false,
            ttl: Some(86400), // 24 hours default
            created_at: now,
            updated_at: now,
            last_accessed_at: now,
            wasm_status: WasmStatus::Pending,
            wasm_error: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn with_xp_name(mut self, xp_name: impl Into<String>) -> Self {
        self.xp_name = Some(xp_name.into());
        self
    }

    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn permanent(mut self) -> Self {
        self.permanent = true;
        self.ttl = None;
        self
    }

    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.permanent = false;
        self.ttl = Some(ttl);
        self
    }

    /// Check if this dashboard has expired
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        if self.permanent {
            return false;
        }
        if let Some(ttl) = self.ttl {
            let expiry = self.last_accessed_at + chrono::Duration::seconds(ttl as i64);
            return Utc::now() > expiry;
        }
        false
    }
}

impl Default for DashboardMeta {
    fn default() -> Self {
        Self::new()
    }
}

/// Full dashboard record: metadata + dashboard data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardRecord {
    /// Dashboard metadata
    #[serde(flatten)]
    pub meta: DashboardMeta,

    /// The actual dashboard visualization data
    pub dashboard: Dashboard,
}

impl DashboardRecord {
    pub fn new(dashboard: Dashboard) -> Self {
        Self {
            meta: DashboardMeta::new(),
            dashboard,
        }
    }

    pub fn with_meta(mut self, meta: DashboardMeta) -> Self {
        self.meta = meta;
        self
    }
}

/// Request to create a new dashboard
#[derive(Clone, Debug, Deserialize)]
pub struct CreateDashboardRequest {
    /// Experiment name
    pub xp_name: Option<String>,

    /// User identifier
    pub user: Option<String>,

    /// Tags for filtering
    #[serde(default)]
    pub tags: Vec<String>,

    /// Whether permanent (never expires)
    #[serde(default)]
    pub permanent: bool,

    /// TTL in seconds (default: 86400 = 24 hours)
    pub ttl: Option<u64>,

    /// The dashboard data
    pub dashboard: Dashboard,
}

/// Request to update dashboard metadata
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateMetaRequest {
    pub xp_name: Option<String>,
    pub user: Option<String>,
    pub tags: Option<Vec<String>>,
    pub permanent: Option<bool>,
    pub ttl: Option<u64>,
}

/// Query parameters for listing dashboards
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ListQuery {
    /// Filter by experiment name
    pub xp_name: Option<String>,

    /// Filter by user
    pub user: Option<String>,

    /// Filter by tag (any match)
    pub tag: Option<String>,

    /// Filter permanent only
    pub permanent: Option<bool>,

    /// Sort field: created_at, updated_at, last_accessed_at
    #[serde(default = "default_sort")]
    pub sort: String,

    /// Sort direction: asc, desc
    #[serde(default = "default_order")]
    pub order: String,

    /// Maximum number of results
    #[serde(default = "default_limit")]
    pub limit: u32,

    /// Offset for pagination
    #[serde(default)]
    pub offset: u32,
}

fn default_sort() -> String {
    "updated_at".into()
}

fn default_order() -> String {
    "desc".into()
}

fn default_limit() -> u32 {
    50
}

/// Summary of a dashboard for list responses
#[derive(Clone, Debug, Serialize)]
pub struct DashboardSummary {
    pub id: Uuid,
    pub xp_name: Option<String>,
    pub user: Option<String>,
    pub tags: Vec<String>,
    pub permanent: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub plot_count: usize,
    pub wasm_status: WasmStatus,
}

impl From<&DashboardRecord> for DashboardSummary {
    fn from(record: &DashboardRecord) -> Self {
        Self {
            id: record.meta.id,
            xp_name: record.meta.xp_name.clone(),
            user: record.meta.user.clone(),
            tags: record.meta.tags.clone(),
            permanent: record.meta.permanent,
            created_at: record.meta.created_at,
            updated_at: record.meta.updated_at,
            plot_count: record.dashboard.plots.len()
                + record
                    .dashboard
                    .tabs
                    .iter()
                    .map(|t| t.plots.len())
                    .sum::<usize>(),
            wasm_status: record.meta.wasm_status.clone(),
        }
    }
}
