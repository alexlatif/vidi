// src/render/cameras.rs

use bevy::prelude::*;
use bevy_camera::visibility::RenderLayers;
use bevy_camera::{
    ClearColorConfig, OrthographicProjection, PerspectiveProjection, Projection, ScalingMode,
    Viewport,
};

use super::layout::Tile;
use super::resources::TileViews;

// Render layers want usize in bevy_camera 0.17.x
pub const LAYER_WORLD: usize = 0;
pub const LAYER_TILE: usize = 1;

#[derive(Component)]
pub struct WorldCam;

#[derive(Component, Clone, Copy, Debug)]
pub struct TileCam {
    pub tile_index: usize,
}

pub struct CamerasPlugin;

impl Plugin for CamerasPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_world_camera).add_systems(
            Update,
            (
                sync_tile_cameras,             // create/despawn to match tiles
                update_tile_camera_viewports,  // viewport rects
                update_tile_camera_projection, // ortho scaling
            ),
        );
    }
}

fn setup_world_camera(mut commands: Commands) {
    // World camera (3D)
    commands.spawn((
        Name::new("world_cam"),
        WorldCam,
        Camera {
            order: 0,
            ..default()
        },
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::FRAC_PI_4,
            near: 0.01,
            far: 10_000.0,
            ..default()
        }),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(LAYER_WORLD),
    ));

    // Light (3D)
    commands.spawn((
        Name::new("world_light"),
        PointLight {
            intensity: 20_000.0,
            range: 10_000.0,
            ..default()
        },
        Transform::from_xyz(5.0, 10.0, 5.0),
        RenderLayers::layer(LAYER_WORLD),
    ));
}

/// Create/despawn tile cameras so there is exactly one per tile.
/// This is the key to avoiding "black screen" when tiles appear after Startup.
fn sync_tile_cameras(
    mut commands: Commands,
    windows: Query<&Window>,
    tile_views: Option<Res<TileViews>>,
    existing: Query<(Entity, &TileCam)>,
) {
    let Ok(win) = windows.single() else {
        return;
    };
    let Some(tile_views) = tile_views else {
        return;
    };

    // Adjust this to your actual field:
    let tiles: &Vec<Tile> = &tile_views.tiles;

    // Track existing tile cameras by index
    let mut have = vec![false; tiles.len()];
    for (e, tc) in existing.iter() {
        if tc.tile_index < tiles.len() {
            have[tc.tile_index] = true;
        } else {
            // stale camera (tile removed)
            commands.entity(e).despawn_recursive();
        }
    }

    // Spawn missing cameras
    for (i, tile) in tiles.iter().enumerate() {
        if have[i] {
            continue;
        }

        let (vp_pos, vp_size) = tile.viewport_physical_px_single(win);
        let tile_h_logical_px = tile.logical_h_px_single(win);

        commands.spawn((
            Name::new(format!("tile_cam_{i}")),
            TileCam { tile_index: i },
            Camera {
                order: 10 + i as isize,
                viewport: Some(Viewport {
                    physical_position: vp_pos,
                    physical_size: vp_size,
                    ..default()
                }),
                clear_color: ClearColorConfig::Default,
                ..default()
            },
            Camera2d,
            Projection::Orthographic(make_tile_ortho(tile_h_logical_px)),
            Transform::from_xyz(0.0, 0.0, 1000.0),
            RenderLayers::layer(LAYER_TILE),
        ));
    }
}

fn make_tile_ortho(tile_h_logical_px: f32) -> OrthographicProjection {
    let mut ortho = OrthographicProjection::default_2d();
    ortho.scaling_mode = ScalingMode::FixedVertical {
        viewport_height: tile_h_logical_px.max(1.0),
    };
    ortho.scale = 1.0;
    ortho
}

fn update_tile_camera_viewports(
    windows: Query<&Window>,
    tile_views: Option<Res<TileViews>>,
    mut q: Query<(&TileCam, &mut Camera)>,
) {
    let Ok(win) = windows.single() else {
        return;
    };
    let Some(tile_views) = tile_views else {
        return;
    };
    let tiles: &Vec<Tile> = &tile_views.tiles;

    for (tc, mut cam) in q.iter_mut() {
        let Some(tile) = tiles.get(tc.tile_index) else {
            continue;
        };
        let (vp_pos, vp_size) = tile.viewport_physical_px_single(win);
        cam.viewport = Some(Viewport {
            physical_position: vp_pos,
            physical_size: vp_size,
            ..default()
        });
    }
}

fn update_tile_camera_projection(
    windows: Query<&Window>,
    tile_views: Option<Res<TileViews>>,
    mut q: Query<(&TileCam, &mut Projection)>,
) {
    let Ok(win) = windows.single() else {
        return;
    };
    let Some(tile_views) = tile_views else {
        return;
    };
    let tiles: &Vec<Tile> = &tile_views.tiles;

    for (tc, mut proj) in q.iter_mut() {
        let Some(tile) = tiles.get(tc.tile_index) else {
            continue;
        };
        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scaling_mode = ScalingMode::FixedVertical {
                viewport_height: tile.logical_h_px_single(win).max(1.0),
            };
            ortho.scale = 1.0;
        }
    }
}

// These impls are fine to keep here, but I'd personally move them into layout.rs.
impl Tile {
    pub fn viewport_physical_px_single(&self, _win: &Window) -> (UVec2, UVec2) {
        (
            self.vp_min.as_uvec2(),
            (self.vp_max - self.vp_min).as_uvec2(),
        )
    }

    pub fn logical_h_px_single(&self, win: &Window) -> f32 {
        let (_pos, size) = self.viewport_physical_px_single(win);
        let sf = win.resolution.scale_factor() as f32;
        (size.y as f32 / sf).max(1.0)
    }
}
