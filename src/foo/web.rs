#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::core::Dashboard;
use crate::render::{DashboardPlugin, DashboardState};
use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
pub fn run_dashboard(dashboard: Dashboard, canvas_id: &str) {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        canvas: Some(format!("#{}", canvas_id)),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            DashboardPlugin,
        ))
        .insert_resource(DashboardState::new(dashboard))
        .run();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_dashboard(_dashboard: Dashboard, _canvas_id: &str) {
    panic!("Web dashboard can only run in WASM target");
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WebDashboard {
    dashboard: Dashboard,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebDashboard {
    #[wasm_bindgen(constructor)]
    pub fn new(json_str: &str) -> Result<WebDashboard, JsValue> {
        console_error_panic_hook::set_once();

        let dashboard: Dashboard = serde_json::from_str(json_str)
            .change_context(crate::error::VidiError)
            .attach("Failed to parse dashboard JSON")
            .map_err(|report| JsValue::from_str(&report.to_string()))?;

        Ok(WebDashboard { dashboard })
    }

    #[wasm_bindgen(js_name = show)]
    pub fn show(&self, canvas_id: &str) {
        run_dashboard(self.dashboard.clone(), canvas_id);
    }

    #[wasm_bindgen(js_name = update2DTrace)]
    pub fn update_2d_trace(
        &mut self,
        plot_id: u32,
        trace_idx: usize,
        points: js_sys::Float32Array,
    ) -> Result<(), JsValue> {
        let points_vec: Vec<f32> = points.to_vec();
        if points_vec.len() % 2 != 0 {
            return Err(JsValue::from_str("Points array must have even length"));
        }

        let points_2d: Vec<crate::core::Point2D> = points_vec
            .chunks_exact(2)
            .map(|chunk| crate::core::Point2D::new(chunk[0], chunk[1]))
            .collect();

        let plot_id = crate::core::PlotId(plot_id);

        if let Some(plot) = self.dashboard.find_plot_mut(plot_id) {
            if let Some(trace) = plot.traces_2d.get_mut(trace_idx) {
                trace.points = points_2d;
                Ok(())
            } else {
                Err(JsValue::from_str(
                    &error_stack::report!(crate::error::VidiError)
                        .attach("Trace index out of bounds")
                        .attach(format!("Trace index: {}", trace_idx))
                        .to_string(),
                ))
            }
        } else {
            Err(JsValue::from_str(
                &error_stack::report!(crate::error::VidiError)
                    .attach("Plot not found")
                    .attach(format!("Plot ID: {}", plot_id.raw()))
                    .to_string(),
            ))
        }
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.dashboard)
            .change_context(crate::error::VidiError)
            .attach("Failed to serialize dashboard")
            .map_err(|report| JsValue::from_str(&report.to_string()))
    }
}
