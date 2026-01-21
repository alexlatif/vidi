//! Dashboard Template WASM Crate
//!
//! This crate is compiled per-dashboard with the dashboard configuration
//! baked in at compile time via include_str!.
//!
//! Note: This crate is only intended to be compiled for wasm32-unknown-unknown.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;

use vidi::prelude::*;

/// The baked-in dashboard JSON from build time
static DASHBOARD_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/dashboard.json"));

/// Auto-start entry point when WASM module is loaded
#[wasm_bindgen(start)]
pub fn start() {
    // Set up panic hook for better error messages in console
    console_error_panic_hook::set_once();

    // Parse the baked-in dashboard
    let dashboard: Dashboard =
        serde_json::from_str(DASHBOARD_JSON).expect("Failed to parse baked-in dashboard JSON");

    // Run the dashboard with the default canvas ID
    run_dashboard(dashboard, "dashboard-canvas");
}

/// Get the baked-in dashboard as JSON (for debugging)
#[wasm_bindgen]
pub fn get_dashboard_json() -> String {
    DASHBOARD_JSON.to_string()
}
