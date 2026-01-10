use bevy::prelude::*;
use bevy_camera::visibility::RenderLayers;
use bevy_camera::{ClearColorConfig, ScalingMode};

use crate::render_gpt1::layout::TileRect;
use crate::render_gpt1::tile::{PlotKind, PlotTile, TileRegistry};

fn layer_for(index: usize) -> RenderLayers {
    // 31 usable layers if you reserve 0; adjust if you want.
    RenderLayers::layer(1 + (index % 31))
}

#[derive(Component)]
pub struct TileCameraTag;

pub fn sync_tile_cameras(
    mut commands: Commands,
    mut reg: ResMut<TileRegistry>,
    tiles: Query<(&PlotTile, &TileRect)>,
    existing: Query<(Entity, &TileCameraTag)>,
) {
    // Remove orphan cameras
    for (cam_e, _) in existing.iter() {
        if !reg.cam_of.values().any(|&e| e == cam_e) {
            commands.entity(cam_e).despawn_recursive();
        }
    }

    for (tile, rect) in tiles.iter() {
        let cam_e = *reg.cam_of.entry(tile.id).or_insert_with(|| {
            commands
                .spawn((Name::new(format!("cam_{:x}", tile.id.0)), TileCameraTag))
                .id()
        });

        match tile.kind {
            PlotKind::TwoD | PlotKind::Placeholder => {
                let mut ortho = OrthographicProjection::default_2d();
                ortho.scaling_mode = ScalingMode::FixedVertical {
                    viewport_height: rect.world_size.y.max(1.0),
                };
                ortho.scale = 1.0;

                commands.entity(cam_e).insert((
                    Camera {
                        viewport: Some(rect.viewport.clone()),
                        order: 10 + tile.index as isize,
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    Camera2d,
                    Projection::Orthographic(ortho),
                    Transform::from_translation(rect.world_center.extend(1000.0)), // ✅ critical
                    layer_for(tile.index),
                ));
            }
            PlotKind::ThreeD => {
                let aspect = (rect.world_size.x / rect.world_size.y).max(0.01);
                let proj = PerspectiveProjection {
                    aspect_ratio: aspect,
                    ..default()
                };

                // View3D-driven transform can be added later; this is correct baseline.
                commands.entity(cam_e).insert((
                    Camera {
                        viewport: Some(rect.viewport.clone()),
                        order: 10 + tile.index as isize,
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    Camera3d::default(),
                    Projection::Perspective(proj),
                    Transform::from_translation(rect.world_center.extend(50.0))
                        .looking_at(rect.world_center.extend(0.0), Vec3::Y),
                    layer_for(tile.index),
                ));
            }
        }
    }
}
