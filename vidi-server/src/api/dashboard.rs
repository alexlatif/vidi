//! Dashboard REST API handlers

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
};
use tracing::{error, info};
use uuid::Uuid;

use crate::AppState;
use crate::error::{Result, ServerError};
use crate::models::{
    CreateDashboardRequest, DashboardMeta, DashboardRecord, DashboardSummary, ListQuery,
    UpdateCommand, UpdateMetaRequest, WasmStatus,
};
use crate::storage::DashboardStore;

/// Build the dashboard API router
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/dashboards", post(create_dashboard))
        .route("/dashboards", get(list_dashboards))
        .route("/dashboards/{id}", get(get_dashboard))
        .route("/dashboards/{id}", put(replace_dashboard))
        .route("/dashboards/{id}", patch(update_meta))
        .route("/dashboards/{id}", delete(delete_dashboard))
        .route("/dashboards/{id}/touch", post(touch_dashboard))
        .route("/dashboards/{id}/update", post(push_update))
        .route("/dashboards/{id}/wasm-status", get(get_wasm_status))
        .route("/dashboards/{id}/recompile", post(trigger_recompile))
}

/// POST /api/v1/dashboards - Create a new dashboard
async fn create_dashboard(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDashboardRequest>,
) -> Result<impl IntoResponse> {
    let meta = DashboardMeta::new()
        .with_xp_name(req.xp_name.unwrap_or_default())
        .with_user(req.user.unwrap_or_default())
        .with_tags(req.tags);

    let meta = if req.permanent {
        meta.permanent()
    } else {
        meta.with_ttl(req.ttl.unwrap_or(state.config.default_ttl))
    };

    let record = DashboardRecord::new(req.dashboard).with_meta(meta);
    let created = state.store.create(record).await?;

    // Trigger async WASM compilation
    let id = created.meta.id;
    spawn_wasm_compilation(state, id);

    Ok((StatusCode::CREATED, Json(created)))
}

/// Spawn async WASM compilation task
fn spawn_wasm_compilation(state: Arc<AppState>, id: Uuid) {
    tokio::spawn(async move {
        info!("Starting WASM compilation for dashboard {}", id);

        // Update status to compiling
        if let Err(e) = state
            .store
            .update_wasm_status(id, WasmStatus::Compiling, None)
            .await
        {
            error!("Failed to update wasm_status to compiling: {}", e);
            return;
        }

        // Get dashboard JSON for compilation
        let dashboard_json = match state.store.get_dashboard_json(id).await {
            Ok(Some(json)) => json,
            Ok(None) => {
                error!("Dashboard {} not found for WASM compilation", id);
                let _ = state
                    .store
                    .update_wasm_status(id, WasmStatus::Failed, Some("Dashboard not found".into()))
                    .await;
                return;
            }
            Err(e) => {
                error!("Failed to get dashboard JSON for {}: {}", id, e);
                let _ = state
                    .store
                    .update_wasm_status(
                        id,
                        WasmStatus::Failed,
                        Some(format!("Failed to get dashboard: {}", e)),
                    )
                    .await;
                return;
            }
        };

        // Compile WASM
        match state
            .wasm_compiler
            .compile_dashboard(id, &dashboard_json)
            .await
        {
            Ok(()) => {
                info!("WASM compilation successful for dashboard {}", id);
                let _ = state
                    .store
                    .update_wasm_status(id, WasmStatus::Ready, None)
                    .await;
            }
            Err(e) => {
                error!("WASM compilation failed for dashboard {}: {}", id, e);
                let _ = state
                    .store
                    .update_wasm_status(id, WasmStatus::Failed, Some(e.to_string()))
                    .await;
            }
        }
    });
}

/// GET /api/v1/dashboards - List dashboards with optional filters
async fn list_dashboards(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<DashboardSummary>>> {
    let results = state.store.list(&query).await?;
    Ok(Json(results))
}

/// GET /api/v1/dashboards/:id - Get a single dashboard
async fn get_dashboard(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DashboardRecord>> {
    // Touch to update last_accessed_at
    let _ = state.store.touch(id).await;

    let record = state
        .store
        .get(id)
        .await?
        .ok_or_else(|| ServerError::NotFound(id.to_string()))?;

    Ok(Json(record))
}

/// PUT /api/v1/dashboards/:id - Replace entire dashboard
async fn replace_dashboard(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateDashboardRequest>,
) -> Result<Json<DashboardRecord>> {
    // Get existing to preserve created_at
    let existing = state
        .store
        .get(id)
        .await?
        .ok_or_else(|| ServerError::NotFound(id.to_string()))?;

    let meta = DashboardMeta {
        id,
        xp_name: req.xp_name.or(existing.meta.xp_name),
        user: req.user.or(existing.meta.user),
        tags: req.tags,
        permanent: req.permanent,
        ttl: if req.permanent {
            None
        } else {
            req.ttl.or(existing.meta.ttl)
        },
        created_at: existing.meta.created_at,
        updated_at: chrono::Utc::now(),
        last_accessed_at: chrono::Utc::now(),
        wasm_status: WasmStatus::Pending, // Reset for recompilation
        wasm_error: None,
    };

    let record = DashboardRecord {
        meta,
        dashboard: req.dashboard,
    };

    let updated = state.store.replace(id, record).await?;

    // Broadcast refresh to connected clients
    state.broadcast_hub.broadcast(
        id,
        UpdateCommand::RefreshAll {
            dashboard: updated.dashboard.clone(),
        },
    );

    // Trigger async WASM recompilation
    spawn_wasm_compilation(Arc::clone(&state), id);

    Ok(Json(updated))
}

/// PATCH /api/v1/dashboards/:id - Update metadata only
async fn update_meta(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMetaRequest>,
) -> Result<Json<DashboardRecord>> {
    let updated = state.store.update_meta(id, req).await?;
    Ok(Json(updated))
}

/// DELETE /api/v1/dashboards/:id - Delete a dashboard
async fn delete_dashboard(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let deleted = state.store.delete(id).await?;

    if deleted {
        // Remove from broadcast hub
        state.broadcast_hub.remove_dashboard(id);

        // Clean up WASM files
        if let Err(e) = state.wasm_compiler.delete_wasm(id).await {
            error!("Failed to delete WASM for dashboard {}: {}", id, e);
        }

        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ServerError::NotFound(id.to_string()))
    }
}

/// POST /api/v1/dashboards/:id/touch - Extend TTL
async fn touch_dashboard(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    // Verify exists
    let _ = state
        .store
        .get(id)
        .await?
        .ok_or_else(|| ServerError::NotFound(id.to_string()))?;

    state.store.touch(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/dashboards/:id/update - Push an update to connected clients
async fn push_update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(cmd): Json<UpdateCommand>,
) -> Result<StatusCode> {
    // Verify dashboard exists
    let _ = state
        .store
        .get(id)
        .await?
        .ok_or_else(|| ServerError::NotFound(id.to_string()))?;

    // Broadcast to connected clients
    state.broadcast_hub.broadcast(id, cmd);

    Ok(StatusCode::ACCEPTED)
}

/// Response for WASM status endpoint
#[derive(serde::Serialize)]
struct WasmStatusResponse {
    status: WasmStatus,
    error: Option<String>,
    wasm_ready: bool,
}

/// GET /api/v1/dashboards/:id/wasm-status - Get WASM compilation status
async fn get_wasm_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<WasmStatusResponse>> {
    let record = state
        .store
        .get(id)
        .await?
        .ok_or_else(|| ServerError::NotFound(id.to_string()))?;

    let wasm_ready = state.wasm_compiler.wasm_exists(id);

    Ok(Json(WasmStatusResponse {
        status: record.meta.wasm_status,
        error: record.meta.wasm_error,
        wasm_ready,
    }))
}

/// POST /api/v1/dashboards/:id/recompile - Manually trigger WASM recompilation
async fn trigger_recompile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    // Verify dashboard exists
    let _ = state
        .store
        .get(id)
        .await?
        .ok_or_else(|| ServerError::NotFound(id.to_string()))?;

    // Check if already compiling
    if state.wasm_compiler.is_compiling(id) {
        return Ok(StatusCode::ACCEPTED); // Already in progress
    }

    // Trigger recompilation
    spawn_wasm_compilation(Arc::clone(&state), id);

    Ok(StatusCode::ACCEPTED)
}
