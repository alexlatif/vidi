//! Build script for dashboard-template
//!
//! Copies the dashboard.json file to OUT_DIR for inclusion in the binary.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Get the OUT_DIR where we'll copy the dashboard JSON
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("dashboard.json");

    // Look for dashboard.json in the crate root
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src_path = Path::new(&manifest_dir).join("dashboard.json");

    if src_path.exists() {
        fs::copy(&src_path, &dest_path).expect("Failed to copy dashboard.json");
        println!("cargo:rerun-if-changed={}", src_path.display());
    } else {
        // Create a default empty dashboard if no JSON provided
        let default_dashboard = r#"{
            "plots": [],
            "tabs": [],
            "layout": { "cols": 1, "rows": 1 }
        }"#;
        fs::write(&dest_path, default_dashboard).expect("Failed to write default dashboard.json");
    }

    // Always recompile when dashboard.json changes
    println!("cargo:rerun-if-changed=dashboard.json");
}
