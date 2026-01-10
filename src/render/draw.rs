use crate::render::{TileRect, TileView, UnitMeshes};
use bevy::prelude::*;
use bevy_camera::visibility::RenderLayers;

pub fn draw_2d_plot(
    commands: &mut Commands,
    root: Entity,
    graph: &crate::core::Graph2D,
    rect: &TileRect,
    view: &TileView,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    let axis_mat = materials.add(ColorMaterial::from(Color::srgb(0.5, 0.5, 0.5)));

    // ✅ clone once for the axis closure
    let layers_axis = layers.clone();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(axis_mat.clone()),
            Transform {
                translation: rect.world_center.extend(0.0),
                scale: Vec3::new(rect.world_size.x, 1.0, 1.0),
                ..default()
            },
            layers_axis.clone(), // ✅ clone inside closure
        ));

        parent.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(axis_mat),
            Transform {
                translation: rect.world_center.extend(0.0),
                scale: Vec3::new(1.0, rect.world_size.y, 1.0),
                ..default()
            },
            layers_axis, // ✅ last use inside this closure
        ));
    });

    for layer in &graph.layers {
        match layer.geometry {
            crate::core::Geometry2D::Line => {
                if layer.xy.len() >= 2 {
                    let color = Color::srgba(
                        layer.style.color.r,
                        layer.style.color.g,
                        layer.style.color.b,
                        layer.style.opacity,
                    );
                    let mat = materials.add(ColorMaterial::from(color));

                    for window in layer.xy.windows(2) {
                        let a = data_to_world(window[0], rect, view);
                        let b = data_to_world(window[1], rect, view);

                        let length = a.distance(b);
                        let angle = (b.y - a.y).atan2(b.x - a.x);

                        // ✅ clone for each closure (cheap); or compute `let layers_line = layers.clone();`
                        let layers_line = layers.clone();

                        commands.entity(root).with_children(|parent| {
                            parent.spawn((
                                Mesh2d(unit.quad.clone()),
                                MeshMaterial2d(mat.clone()),
                                Transform {
                                    translation: ((a + b) * 0.5).extend(0.0),
                                    rotation: Quat::from_rotation_z(angle),
                                    scale: Vec3::new(length, layer.style.size, 1.0),
                                    ..default()
                                },
                                layers_line,
                            ));
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// Draw placeholder
pub fn draw_placeholder(
    commands: &mut Commands,
    root: Entity,
    rect: &TileRect,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    let mat = materials.add(ColorMaterial::from(Color::srgb(0.2, 0.2, 0.2)));

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(mat),
            Transform {
                translation: rect.world_center.extend(0.0),
                scale: rect.world_size.extend(1.0) * 0.8,
                ..default()
            },
            layers,
        ));
    });
}

/// Convert data coordinates to world coordinates
fn data_to_world(data: Vec2, rect: &TileRect, view: &TileView) -> Vec2 {
    rect.world_center + view.offset + data * view.scale
}
