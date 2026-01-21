//! 2D chart rendering: scatter plots, line charts, area fills.

#![allow(clippy::too_many_arguments)]

use super::common::{data_to_world, draw_tile_border};
use crate::render::{TileRect, TileView, UnitMeshes};
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_camera::visibility::RenderLayers;
use bevy_mesh::{Indices, PrimitiveTopology};

/// Draw a 2D plot with multiple layers (lines, points, fills).
pub fn draw_2d_plot(
    commands: &mut Commands,
    root: Entity,
    graph: &crate::core::Graph2D,
    rect: &TileRect,
    view: &TileView,
    unit: &UnitMeshes,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    draw_tile_border(
        commands,
        root,
        rect,
        unit,
        materials,
        layers.clone(),
        Color::srgb(0.3, 0.3, 0.4),
        1.0,
    );

    // Draw axis at data origin (0,0) - moves with pan/zoom
    let axis_mat = materials.add(ColorMaterial::from(Color::srgb(0.5, 0.5, 0.5)));
    let axis_origin = data_to_world(Vec2::ZERO, rect, view);

    commands.entity(root).with_children(|parent| {
        // X-axis (horizontal line at y=0)
        parent.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(axis_mat.clone()),
            Transform {
                translation: Vec3::new(rect.world_center.x, axis_origin.y, 0.5),
                scale: Vec3::new(rect.world_size.x, 1.0, 1.0),
                ..default()
            },
            layers.clone(),
        ));

        // Y-axis (vertical line at x=0)
        parent.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(axis_mat),
            Transform {
                translation: Vec3::new(axis_origin.x, rect.world_center.y, 0.5),
                scale: Vec3::new(1.0, rect.world_size.y, 1.0),
                ..default()
            },
            layers.clone(),
        ));
    });

    // Draw axis labels
    if let Some(ref x_label) = graph.x_label {
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(x_label.clone()),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                Transform::from_translation(Vec3::new(
                    rect.world_center.x,
                    rect.world_center.y - rect.world_size.y * 0.5 + 12.0,
                    2.0,
                )),
                layers.clone(),
            ));
        });
    }

    if let Some(ref y_label) = graph.y_label {
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(y_label.clone()),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                Transform {
                    translation: Vec3::new(
                        rect.world_center.x - rect.world_size.x * 0.5 + 12.0,
                        rect.world_center.y,
                        2.0,
                    ),
                    rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                    ..default()
                },
                layers.clone(),
            ));
        });
    }

    for layer in &graph.layers {
        let color = Color::srgba(
            layer.style.color.r,
            layer.style.color.g,
            layer.style.color.b,
            layer.style.opacity,
        );
        let mat = materials.add(ColorMaterial::from(color));

        match layer.geometry {
            crate::core::Geometry2D::Line => {
                draw_line_layer(commands, root, layer, rect, view, unit, &mat, &layers);
            }
            crate::core::Geometry2D::Points => {
                draw_points_layer(commands, root, layer, rect, view, unit, &mat, &layers);
            }
            crate::core::Geometry2D::FillBetween => {
                draw_fill_between_layer(
                    commands, root, layer, rect, view, meshes, materials, &layers,
                );
            }
            _ => {}
        }
    }
}

fn draw_line_layer(
    commands: &mut Commands,
    root: Entity,
    layer: &crate::core::Layer2D,
    rect: &TileRect,
    view: &TileView,
    unit: &UnitMeshes,
    mat: &Handle<ColorMaterial>,
    layers: &RenderLayers,
) {
    if layer.xy.len() < 2 {
        return;
    }

    // Compute tile bounds for culling
    let half_size = rect.world_size * 0.5;
    let bounds_min = rect.world_center - half_size;
    let bounds_max = rect.world_center + half_size;

    for window in layer.xy.windows(2) {
        let a = data_to_world(window[0], rect, view);
        let b = data_to_world(window[1], rect, view);

        // Skip line segments entirely outside tile bounds
        if (a.x < bounds_min.x && b.x < bounds_min.x)
            || (a.x > bounds_max.x && b.x > bounds_max.x)
            || (a.y < bounds_min.y && b.y < bounds_min.y)
            || (a.y > bounds_max.y && b.y > bounds_max.y)
        {
            continue;
        }

        let length = a.distance(b);
        let angle = (b.y - a.y).atan2(b.x - a.x);

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(mat.clone()),
                Transform {
                    translation: ((a + b) * 0.5).extend(0.0),
                    rotation: Quat::from_rotation_z(angle),
                    scale: Vec3::new(length, layer.style.size, 1.0),
                },
                layers.clone(),
            ));
        });
    }
}

fn draw_points_layer(
    commands: &mut Commands,
    root: Entity,
    layer: &crate::core::Layer2D,
    rect: &TileRect,
    view: &TileView,
    unit: &UnitMeshes,
    mat: &Handle<ColorMaterial>,
    layers: &RenderLayers,
) {
    let base_size = layer.style.size;

    // Compute tile bounds for culling
    let half_size = rect.world_size * 0.5;
    let bounds_min = rect.world_center - half_size;
    let bounds_max = rect.world_center + half_size;

    for (i, &pt) in layer.xy.iter().enumerate() {
        let world_pos = data_to_world(pt, rect, view);

        // Use per-point size if available, otherwise use style.size
        let point_size = layer
            .sizes
            .as_ref()
            .and_then(|sizes| sizes.get(i).copied())
            .unwrap_or(base_size);

        let radius = point_size * 0.5;

        // Skip points entirely outside tile bounds
        if world_pos.x + radius < bounds_min.x || world_pos.x - radius > bounds_max.x {
            continue;
        }
        if world_pos.y + radius < bounds_min.y || world_pos.y - radius > bounds_max.y {
            continue;
        }

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(mat.clone()),
                Transform {
                    translation: world_pos.extend(0.1),
                    scale: Vec3::splat(point_size),
                    ..default()
                },
                layers.clone(),
            ));
        });
    }
}

fn draw_fill_between_layer(
    commands: &mut Commands,
    root: Entity,
    layer: &crate::core::Layer2D,
    rect: &TileRect,
    view: &TileView,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layers: &RenderLayers,
) {
    let Some(lower) = &layer.lower_line else {
        return;
    };

    let upper = &layer.xy;
    let n = upper.len().min(lower.len());

    if n < 2 {
        return;
    }

    // Build triangle mesh for seamless fill
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n * 2);
    let mut indices: Vec<u32> = Vec::with_capacity((n - 1) * 6);

    // Add all vertices: upper points first, then lower points
    for i in 0..n {
        let up = data_to_world(upper[i], rect, view);
        positions.push([up.x, up.y, 0.0]);
    }
    for i in 0..n {
        let lo = data_to_world(lower[i], rect, view);
        positions.push([lo.x, lo.y, 0.0]);
    }

    // Create triangles for each segment
    for i in 0..(n - 1) {
        let u0 = i as u32;
        let u1 = (i + 1) as u32;
        let l0 = (n + i) as u32;
        let l1 = (n + i + 1) as u32;

        // Two triangles per segment (CCW winding for front-facing)
        indices.extend_from_slice(&[u0, l1, l0]);
        indices.extend_from_slice(&[u0, u1, l1]);
    }

    let vertex_count = positions.len();
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; vertex_count];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; vertex_count];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    let fill_mesh = meshes.add(mesh);

    let fill_color = Color::srgba(
        layer.style.color.r,
        layer.style.color.g,
        layer.style.color.b,
        layer.style.opacity,
    );
    let fill_mat = materials.add(ColorMaterial::from(fill_color));

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Mesh2d(fill_mesh),
            MeshMaterial2d(fill_mat),
            Transform::from_translation(Vec3::ZERO),
            layers.clone(),
        ));
    });
}
