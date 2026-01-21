//! Drawing functions for all chart types.
//!
//! This module is organized into focused submodules:
//! - `common`: Shared utilities (borders, transforms, axis ticks)
//! - `charts_2d`: 2D scatter/line/fill plots
//! - `charts_3d`: 3D scatter and surface plots
//! - `distribution`: Histogram, PDF, boxplot, ECDF
//! - `financial`: Candlestick/OHLC charts
//! - `heatmap`: Grid-based color visualizations
//! - `radial`: Pie charts and radar/spider charts

mod charts_2d;
mod charts_3d;
mod common;
mod distribution;
mod financial;
mod heatmap;
mod radial;

// Re-export public drawing functions
pub use charts_2d::draw_2d_plot;
pub use charts_3d::draw_3d_plot;
pub use common::{
    data_to_world, draw_axis_ticks, draw_placeholder, draw_plot_title, draw_tile_border,
    format_tick, nice_step, world_to_data,
};
pub use distribution::{draw_boxplot, draw_ecdf, draw_histogram, draw_pdf};
pub use financial::draw_candlestick;
pub use heatmap::draw_heatmap;
pub use radial::draw_radial;
