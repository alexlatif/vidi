//! WASM API exports for JavaScript interop
//!
//! This module provides `#[wasm_bindgen]` exports for controlling Vidi dashboards
//! from JavaScript. It is only compiled when targeting wasm32.

#![cfg(target_arch = "wasm32")]

use bevy_math::Vec2;
use parking_lot::Mutex;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::core::{Dashboard, Plot};
use crate::runtime::run_dashboard;

/// JavaScript-accessible dashboard wrapper
#[wasm_bindgen]
pub struct JsDashboard {
    /// The dashboard data
    dashboard: Arc<Mutex<Dashboard>>,
    /// Canvas ID for rendering
    canvas_id: String,
    /// Whether the Bevy app has started
    started: bool,
}

#[wasm_bindgen]
impl JsDashboard {
    /// Create a new JsDashboard from JSON
    ///
    /// # Arguments
    /// * `json` - JSON string representing the Dashboard
    /// * `canvas_id` - HTML canvas element ID (without #)
    #[wasm_bindgen(constructor)]
    pub fn new(json: &str, canvas_id: &str) -> Result<JsDashboard, JsValue> {
        let dashboard: Dashboard = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse dashboard JSON: {}", e)))?;

        Ok(JsDashboard {
            dashboard: Arc::new(Mutex::new(dashboard)),
            canvas_id: canvas_id.to_string(),
            started: false,
        })
    }

    /// Start the Bevy render loop
    ///
    /// This should only be called once. After calling this, use the update
    /// methods to modify the dashboard data.
    #[wasm_bindgen]
    pub fn start(&mut self) {
        if self.started {
            web_sys::console::warn_1(&"Dashboard already started".into());
            return;
        }

        let dashboard = self.dashboard.lock().clone();
        self.started = true;

        // Note: In the current architecture, run_dashboard takes ownership
        // and runs the Bevy app loop. For real-time updates, we'd need to
        // use Bevy's ECS system to mutate the DashboardRes resource.
        // This is a simplified implementation.
        run_dashboard(dashboard, &self.canvas_id);
    }

    /// Replace the entire dashboard
    ///
    /// Note: This currently requires recreating the Bevy app.
    /// A future implementation could use Bevy events/resources for hot updates.
    #[wasm_bindgen]
    pub fn set_dashboard(&mut self, json: &str) -> Result<(), JsValue> {
        let dashboard: Dashboard = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse dashboard JSON: {}", e)))?;

        *self.dashboard.lock() = dashboard;

        // TODO: Send update to running Bevy app via channel/event
        // For now, log that an update was received
        web_sys::console::log_1(&"Dashboard updated (requires restart to take effect)".into());

        Ok(())
    }

    /// Append points to a 2D layer
    ///
    /// # Arguments
    /// * `plot_id` - The plot ID (u64)
    /// * `layer_idx` - Layer index within the plot
    /// * `points` - Flat array of [x1, y1, x2, y2, ...]
    #[wasm_bindgen]
    pub fn append_points(
        &mut self,
        plot_id: u64,
        layer_idx: usize,
        points: &[f32],
    ) -> Result<(), JsValue> {
        if points.len() % 2 != 0 {
            return Err(JsValue::from_str(
                "Points array length must be even (x,y pairs)",
            ));
        }

        let mut dashboard = self.dashboard.lock();

        // Find the plot and append points
        for plot in dashboard.plots.iter_mut() {
            if let Plot::Graph2D(graph) = plot {
                if graph.id.0 == plot_id {
                    if let Some(layer) = graph.layers.get_mut(layer_idx) {
                        for chunk in points.chunks(2) {
                            layer.xy.push(Vec2::new(chunk[0], chunk[1]));
                        }
                        // TODO: Mark tile as dirty in running Bevy app
                        return Ok(());
                    } else {
                        return Err(JsValue::from_str(&format!("Layer {} not found", layer_idx)));
                    }
                }
            }
        }

        // Also check tabs
        for tab in dashboard.tabs.iter_mut() {
            for plot in tab.plots.iter_mut() {
                if let Plot::Graph2D(graph) = plot {
                    if graph.id.0 == plot_id {
                        if let Some(layer) = graph.layers.get_mut(layer_idx) {
                            for chunk in points.chunks(2) {
                                layer.xy.push(Vec2::new(chunk[0], chunk[1]));
                            }
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(JsValue::from_str(&format!("Plot {} not found", plot_id)))
    }

    /// Replace all points in a 2D layer
    #[wasm_bindgen]
    pub fn replace_trace(
        &mut self,
        plot_id: u64,
        layer_idx: usize,
        points: &[f32],
    ) -> Result<(), JsValue> {
        if points.len() % 2 != 0 {
            return Err(JsValue::from_str(
                "Points array length must be even (x,y pairs)",
            ));
        }

        let mut dashboard = self.dashboard.lock();

        // Convert flat array to Vec2
        let new_points: Vec<Vec2> = points
            .chunks(2)
            .map(|chunk| Vec2::new(chunk[0], chunk[1]))
            .collect();

        // Find and update the layer
        for plot in dashboard.plots.iter_mut() {
            if let Plot::Graph2D(graph) = plot {
                if graph.id.0 == plot_id {
                    if let Some(layer) = graph.layers.get_mut(layer_idx) {
                        layer.xy = new_points;
                        return Ok(());
                    }
                }
            }
        }

        for tab in dashboard.tabs.iter_mut() {
            for plot in tab.plots.iter_mut() {
                if let Plot::Graph2D(graph) = plot {
                    if graph.id.0 == plot_id {
                        if let Some(layer) = graph.layers.get_mut(layer_idx) {
                            layer.xy = new_points;
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(JsValue::from_str(&format!("Plot {} not found", plot_id)))
    }

    /// Update an entire plot by ID
    #[wasm_bindgen]
    pub fn update_plot(&mut self, plot_id: u64, json: &str) -> Result<(), JsValue> {
        let new_plot: Plot = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse plot JSON: {}", e)))?;

        let mut dashboard = self.dashboard.lock();

        // Find and replace the plot
        for plot in dashboard.plots.iter_mut() {
            let id = match plot {
                Plot::Graph2D(g) => g.id.0,
                Plot::Graph3D(g) => g.id.0,
                _ => continue,
            };
            if id == plot_id {
                *plot = new_plot;
                return Ok(());
            }
        }

        for tab in dashboard.tabs.iter_mut() {
            for plot in tab.plots.iter_mut() {
                let id = match plot {
                    Plot::Graph2D(g) => g.id.0,
                    Plot::Graph3D(g) => g.id.0,
                    _ => continue,
                };
                if id == plot_id {
                    *plot = new_plot;
                    return Ok(());
                }
            }
        }

        Err(JsValue::from_str(&format!("Plot {} not found", plot_id)))
    }

    /// Get the current dashboard as JSON
    #[wasm_bindgen]
    pub fn to_json(&self) -> Result<String, JsValue> {
        let dashboard = self.dashboard.lock();
        serde_json::to_string(&*dashboard)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize dashboard: {}", e)))
    }

    /// Get the canvas ID
    #[wasm_bindgen(getter)]
    pub fn canvas_id(&self) -> String {
        self.canvas_id.clone()
    }

    /// Check if the dashboard has been started
    #[wasm_bindgen(getter)]
    pub fn is_started(&self) -> bool {
        self.started
    }
}
