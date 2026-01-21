//! WASM Compilation Service
//!
//! Compiles per-dashboard WASM binaries with baked-in dashboard configuration.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::process::Command;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::{Result, ServerError};

/// WASM Compiler service
pub struct WasmCompiler {
    /// Output directory for compiled WASM (e.g., "wasm/")
    wasm_out_dir: PathBuf,
    /// Path to the vidi workspace root
    vidi_workspace: PathBuf,
    /// Path to the dashboard template crate
    template_crate: PathBuf,
    /// Concurrency limiter (only one compilation at a time by default)
    compile_semaphore: Arc<tokio::sync::Semaphore>,
    /// Currently compiling dashboards (to prevent duplicate compilations)
    compiling: Arc<Mutex<std::collections::HashSet<Uuid>>>,
}

impl WasmCompiler {
    /// Create a new WasmCompiler
    ///
    /// # Arguments
    /// * `wasm_out_dir` - Directory where compiled WASM files are stored
    /// * `vidi_workspace` - Path to the vidi workspace root
    /// * `max_concurrent` - Maximum concurrent compilations (default: 1)
    pub fn new(wasm_out_dir: PathBuf, vidi_workspace: PathBuf, max_concurrent: usize) -> Self {
        let template_crate = vidi_workspace.join("vidi-server/dashboard-template");

        Self {
            wasm_out_dir,
            vidi_workspace,
            template_crate,
            compile_semaphore: Arc::new(tokio::sync::Semaphore::new(max_concurrent)),
            compiling: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }

    /// Check if WASM exists for a dashboard
    pub fn wasm_exists(&self, id: Uuid) -> bool {
        self.wasm_path(id).exists()
    }

    /// Get the output directory for a dashboard's WASM
    pub fn wasm_dir(&self, id: Uuid) -> PathBuf {
        self.wasm_out_dir.join(id.to_string())
    }

    /// Get the path to the main JS file for a dashboard
    pub fn wasm_path(&self, id: Uuid) -> PathBuf {
        self.wasm_dir(id).join("vidi.js")
    }

    /// Check if a dashboard is currently being compiled
    pub fn is_compiling(&self, id: Uuid) -> bool {
        self.compiling.lock().contains(&id)
    }

    /// Compile WASM for a dashboard
    ///
    /// This method:
    /// 1. Writes the dashboard JSON to the template crate
    /// 2. Runs cargo build for wasm32-unknown-unknown
    /// 3. Runs wasm-bindgen to generate JS bindings
    /// 4. Copies output to the dashboard's wasm directory
    pub async fn compile_dashboard(&self, id: Uuid, dashboard_json: &str) -> Result<()> {
        // Check if already compiling
        {
            let mut compiling = self.compiling.lock();
            if compiling.contains(&id) {
                return Err(ServerError::Internal(format!(
                    "Dashboard {} is already being compiled",
                    id
                )));
            }
            compiling.insert(id);
        }

        // Acquire semaphore permit for concurrency limiting
        let _permit = self
            .compile_semaphore
            .acquire()
            .await
            .map_err(|e| ServerError::Internal(format!("Semaphore error: {}", e)))?;

        let result = self.do_compile(id, dashboard_json).await;

        // Remove from compiling set
        self.compiling.lock().remove(&id);

        result
    }

    async fn do_compile(&self, id: Uuid, dashboard_json: &str) -> Result<()> {
        info!("Starting WASM compilation for dashboard {}", id);

        // Step 1: Write dashboard JSON to template crate's build directory
        let json_path = self.template_crate.join("dashboard.json");
        tokio::fs::write(&json_path, dashboard_json)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to write dashboard JSON: {}", e)))?;

        // Step 2: Build the template crate for wasm32
        info!("Building WASM for dashboard {}...", id);
        let build_output = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--target",
                "wasm32-unknown-unknown",
                "-p",
                "dashboard-template",
            ])
            .current_dir(&self.vidi_workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to run cargo build: {}", e)))?;

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            error!("WASM build failed for {}: {}", id, stderr);
            return Err(ServerError::Internal(format!(
                "WASM build failed: {}",
                stderr
            )));
        }

        // Step 3: Run wasm-bindgen
        let wasm_input = self
            .vidi_workspace
            .join("target/wasm32-unknown-unknown/release/dashboard_template.wasm");

        let out_dir = self.wasm_dir(id);
        tokio::fs::create_dir_all(&out_dir)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to create output dir: {}", e)))?;

        info!("Running wasm-bindgen for dashboard {}...", id);
        let bindgen_output = Command::new("wasm-bindgen")
            .args([
                wasm_input.to_str().unwrap(),
                "--target",
                "web",
                "--out-dir",
                out_dir.to_str().unwrap(),
                "--out-name",
                "vidi",
                "--no-typescript",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to run wasm-bindgen: {}", e)))?;

        if !bindgen_output.status.success() {
            let stderr = String::from_utf8_lossy(&bindgen_output.stderr);
            error!("wasm-bindgen failed for {}: {}", id, stderr);
            return Err(ServerError::Internal(format!(
                "wasm-bindgen failed: {}",
                stderr
            )));
        }

        // Step 4: Optionally run wasm-opt (if available)
        let bg_wasm = out_dir.join("vidi_bg.wasm");
        if let Ok(opt_output) = Command::new("wasm-opt")
            .args([
                "-Oz",
                "-o",
                bg_wasm.to_str().unwrap(),
                bg_wasm.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
        {
            if opt_output.status.success() {
                info!("wasm-opt optimization applied for dashboard {}", id);
            } else {
                warn!("wasm-opt failed, continuing without optimization");
            }
        }

        info!("WASM compilation complete for dashboard {}", id);
        Ok(())
    }

    /// Delete compiled WASM for a dashboard
    pub async fn delete_wasm(&self, id: Uuid) -> Result<()> {
        let dir = self.wasm_dir(id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await.map_err(|e| {
                ServerError::Internal(format!("Failed to delete WASM directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Verify that the compilation toolchain is available
    pub async fn verify_toolchain(&self) -> Result<()> {
        // Check cargo
        let cargo_check = Command::new("cargo")
            .args(["--version"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        if cargo_check.is_err() || !cargo_check.unwrap().status.success() {
            return Err(ServerError::Internal("cargo not found".into()));
        }

        // Check wasm32 target
        let target_check = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        if let Ok(output) = target_check {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.contains("wasm32-unknown-unknown") {
                warn!(
                    "wasm32-unknown-unknown target not installed. Run: rustup target add wasm32-unknown-unknown"
                );
            }
        }

        // Check wasm-bindgen
        let bindgen_check = Command::new("wasm-bindgen")
            .args(["--version"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        if bindgen_check.is_err() || !bindgen_check.unwrap().status.success() {
            warn!(
                "wasm-bindgen-cli not found. Run: cargo install wasm-bindgen-cli --version 0.2.106"
            );
        }

        Ok(())
    }
}
