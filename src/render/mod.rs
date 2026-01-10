pub mod components;
pub mod draw;
pub mod resources;
pub mod systems;

use components::*;
use draw::*;
pub use resources::*;
use systems::*;

use bevy::prelude::*;

#[derive(Default)]
pub struct DashRenderPlugin;

impl Plugin for DashRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileRegistry>()
            .init_resource::<HoveredTile>()
            .init_resource::<CursorWorldPos>()
            .add_systems(Startup, (setup_global_scene, setup_unit_meshes))
            .add_systems(
                Update,
                (
                    sync_plots_to_tiles,
                    update_tile_layout,
                    auto_fit_tiles,
                    sync_tile_cameras,
                    update_hovered_tile,
                    handle_input,
                    draw_dirty_tiles,
                    update_crosshair,
                ),
            );
    }
}
