//! Vidi XP Dashboard Server
//!
//! A server for hosting WASM-compiled Vidi dashboards with real-time streaming.

mod api;
mod config;
mod error;
mod lifecycle;
mod models;
mod storage;
mod wasm_compiler;

use std::path::PathBuf;
use std::sync::Arc;

use axum::{Router, routing::get};
use clap::Parser;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::stream::BroadcastHub;
use crate::config::Config;
use crate::storage::sqlite::SqliteStore;
use crate::wasm_compiler::WasmCompiler;

/// Application state shared across handlers
pub struct AppState {
    pub store: SqliteStore,
    pub broadcast_hub: BroadcastHub,
    pub config: Config,
    pub wasm_compiler: Arc<WasmCompiler>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vidi_server=debug,tower_http=debug".into()),
        )
        .init();

    // Parse CLI args
    let config = Config::parse();
    info!("Starting vidi-server on {}:{}", config.host, config.port);

    // Initialize database
    let store = SqliteStore::new(&config.db_path).await?;
    store.run_migrations().await?;

    // Create broadcast hub for WebSocket streaming
    let broadcast_hub = BroadcastHub::new();

    // Create WASM compiler
    // Workspace root is the current directory when running with `cargo run -p vidi-server`
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let wasm_compiler = Arc::new(WasmCompiler::new(
        PathBuf::from(&config.wasm_dir),
        workspace_root,
        1, // max concurrent compilations
    ));

    // Verify WASM toolchain on startup
    if let Err(e) = wasm_compiler.verify_toolchain().await {
        tracing::warn!(
            "WASM toolchain verification failed: {}. Per-dashboard WASM compilation may not work.",
            e
        );
    }

    // Build app state
    let state = Arc::new(AppState {
        store,
        broadcast_hub,
        config: config.clone(),
        wasm_compiler,
    });

    // Start lifecycle cleanup task
    let cleanup_state = Arc::clone(&state);
    tokio::spawn(async move {
        lifecycle::cleanup_task(cleanup_state).await;
    });

    // Build router
    let app = Router::new()
        // Portal routes
        .route("/", get(api::portal::index))
        .route("/d/{id}", get(api::portal::dashboard_view))
        // API routes
        .nest("/api/v1", api::dashboard::router())
        // WebSocket route
        .route("/ws/v1/dashboards/{id}", get(api::stream::ws_handler))
        // Static files
        .nest_service("/static", ServeDir::new(&config.static_dir))
        .nest_service("/wasm", ServeDir::new(&config.wasm_dir))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);

    if let (Some(cert_path), Some(key_path)) = (&config.tls_cert, &config.tls_key) {
        // TLS enabled
        info!("TLS enabled with cert: {}", cert_path);
        let tls_config = config::load_tls_config(cert_path, key_path)?;
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        axum_server::from_tcp_rustls(listener.into_std()?, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        // Plain HTTP
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        info!("Listening on http://{}", addr);
        axum::serve(listener, app).await?;
    }

    Ok(())
}
