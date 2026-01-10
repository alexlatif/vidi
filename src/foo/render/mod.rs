// src/render/mod.rs
//! Bevy 0.17.3 dashboard renderer (module layout)
//!
//! Goals retained:
//! - 2D + 3D are peers: each tile has its own camera + viewport scissor (hard clip)
//! - Each tile has a unique RenderLayers mask for isolation
//! - No artifacts: we despawn all Rendered entities on redraw
//! - Hover split stays correct
//! - Layout fills dashboard, tight spacing, no overflow, content respects frame

pub mod cameras;
pub mod draw2d;
pub mod draw3d;
pub mod gestures;
pub mod layout;
pub mod mesh;
pub mod resources;
pub mod specs;

pub use gestures::handle_gestures;
pub use resources::setup_global_scene;
pub use resources::{DashboardRes, HoveredTile, TileViews, View2D, View3D};
pub use specs::update_hovered_tile;

use bevy::prelude::*;

pub struct DashRenderPlugin;

impl Plugin for DashRenderPlugin {
    fn build(&self, app: &mut App) {
        // app.add_plugins(cameras::CamerasPlugin);
        app.init_resource::<HoveredTile>()
            .init_resource::<TileViews>()
            .add_systems(Startup, setup_global_scene)
            .add_systems(
                Update,
                (
                    update_hovered_tile,
                    handle_gestures.after(update_hovered_tile),
                ),
            )
            .add_plugins(cameras::CamerasPlugin);
    }
}
