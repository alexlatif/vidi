use bevy::prelude::*;

use crate::core::Dashboard;
use crate::render::{DashRenderPlugin, DashboardRes};

#[cfg(not(target_arch = "wasm32"))]
pub fn run_dashboard(dashboard: Dashboard) {
    let bg = dashboard.background;
    App::new()
        .insert_resource(ClearColor(Color::srgb(bg.r, bg.g, bg.b)))
        .insert_resource(DashboardRes::new(dashboard))
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            DashRenderPlugin,
        ))
        .run();
}

#[cfg(target_arch = "wasm32")]
pub fn run_dashboard(dashboard: Dashboard, canvas_id: &str) {
    let bg = dashboard.background;
    App::new()
        .insert_resource(ClearColor(Color::srgb(bg.r, bg.g, bg.b)))
        .insert_resource(DashboardRes::new(dashboard))
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
            DashRenderPlugin,
        ))
        .run();
}
