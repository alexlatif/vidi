//! Portal page handlers

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
};
use uuid::Uuid;

use crate::AppState;

/// GET / - Portal index page
pub async fn index(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    // Serve the static portal HTML
    let html = include_str!("../../static/portal.html");
    Html(html)
}

/// GET /d/:id - Dashboard viewer page
pub async fn dashboard_view(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Html<String> {
    // Parse UUID to validate
    match Uuid::parse_str(&id) {
        Ok(_) => {
            let html = include_str!("../../static/dashboard.html");
            Html(html.to_string())
        }
        Err(_) => {
            // Return 404 for invalid IDs
            Html("<h1>Dashboard not found</h1>".to_string())
        }
    }
}
