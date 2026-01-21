//! # Vidi
//!
//! A high-performance data visualization library for Rust, powered by [Bevy](https://bevyengine.org/).
//!
//! Vidi provides a declarative API for creating interactive 2D/3D plots and dashboards
//! that run natively or in the browser via WebAssembly.
//!
//! ## Features
//!
//! - **2D Charts**: Line plots, scatter plots, area charts, bar charts, bubble charts
//! - **3D Visualization**: 3D scatter plots and surface plots with orbit controls
//! - **Statistical Plots**: Histograms, PDFs, box plots, ECDF
//! - **Financial Charts**: Candlestick/OHLC charts
//! - **Heatmaps**: 2D heatmaps with multiple colormaps
//! - **Radial Charts**: Pie charts and radar/spider charts
//! - **Interactive**: Pan, zoom, and rotate controls
//! - **Real-time Updates**: Stream data via WebSocket
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use vidi::prelude::*;
//! use glam::Vec2;
//!
//! fn main() {
//!     dash()
//!         .add_2d(|p| {
//!             let data: Vec<Vec2> = (0..100)
//!                 .map(|i| {
//!                     let x = i as f32 * 0.1;
//!                     Vec2::new(x, x.sin())
//!                 })
//!                 .collect();
//!
//!             p.line(data, None)
//!                 .x_label("Time")
//!                 .y_label("Amplitude")
//!         })
//!         .run_local();
//! }
//! ```
//!
//! ## Dashboard Builder
//!
//! Use the [`dash()`] function to start building a dashboard:
//!
//! ```rust,no_run
//! use vidi::prelude::*;
//!
//! dash()
//!     .add_2d(|p| p.scatter(vec![], None))     // 2D plot
//!     .add_3d(|p| p.points(vec![], None))      // 3D plot
//!     .add_distribution(|d| d.histogram(vec![])) // Statistical
//!     .add_heatmap(|h| h.data(10, 10, vec![0.0; 100])) // Heatmap
//!     .run_local();
//! ```
//!
//! ## Web Dashboard
//!
//! Post dashboards to a vidi-server for browser viewing:
//!
//! ```rust,ignore
//! let handle = dash()
//!     .add_2d(|p| p.line(data, None))
//!     .run_web("http://localhost:8080", WebConfig::default())?;
//!
//! // Stream updates
//! handle.append_points_2d(plot_id, 0, &new_points)?;
//! ```
//!
//! ## Modules
//!
//! - [`core`]: Data model definitions (Plot, Graph2D, Graph3D, etc.)
//! - [`dash`]: Builder API for constructing dashboards
//! - [`render`]: Bevy ECS rendering implementation
//! - [`runtime`]: Application bootstrap and run loop

pub mod core;
pub mod dash;
pub mod render;
pub mod runtime;
#[cfg(target_arch = "wasm32")]
pub mod wasm_api;

use std::fmt;

/// Error type for Vidi operations.
#[derive(Debug)]
pub struct VidiError;

impl fmt::Display for VidiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VidiError")
    }
}

impl std::error::Error for VidiError {}

/// Result type alias using error-stack for rich error context.
pub type Result<T> = std::result::Result<T, error_stack::Report<VidiError>>;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Prelude module - import everything you need with `use vidi::prelude::*;`
///
/// This includes:
/// - All core data types (Plot, Graph2D, Graph3D, Color, Style, etc.)
/// - Dashboard builder API (dash, DashBuilder, Plot2DBuilder, etc.)
/// - Render components (PlotId, etc.)
/// - Runtime functions (run_dashboard)
/// - Web dashboard types (WebConfig, WebDashboard) on native
pub mod prelude {
    pub use crate::core::*;
    pub use crate::dash::*;
    pub use crate::render::*;
    pub use crate::runtime::*;

    // Re-export web dashboard types (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::dash::{WebConfig, WebDashboard};
}
