//! 3D chart rendering: scatter points and surfaces.

#![allow(clippy::too_many_arguments)]

use super::common::draw_tile_border;
use crate::render::{AxisInfo3D, AxisInfo3DStore, ScatterPoints3D, TileRect, UnitMeshes};
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_camera::visibility::RenderLayers;
use bevy_mesh::{Indices, PrimitiveTopology};

/// Draw a 3D plot with lighting, axes, and data layers.
pub fn draw_3d_plot(
    commands: &mut Commands,
    root: Entity,
    graph: &crate::core::Graph3D,
    rect: &TileRect,
    tile_index: usize,
    unit: &UnitMeshes,
    meshes: &mut Assets<Mesh>,
    color_materials: &mut Assets<ColorMaterial>,
    materials: &mut Assets<StandardMaterial>,
    layers: RenderLayers,
    overlay_layers: RenderLayers,
    scatter_points: &mut ScatterPoints3D,
    axis_info_store: &mut AxisInfo3DStore,
) {
    // Calculate data bounds for normalization
    let bounds = compute_3d_bounds(&graph.layers);

    // Store axis info for label rendering
    axis_info_store.info.insert(
        tile_index,
        AxisInfo3D {
            x_label: graph.x_label.clone(),
            y_label: graph.y_label.clone(),
            z_label: graph.z_label.clone(),
            bounds_min: bounds.0,
            bounds_max: bounds.1,
        },
    );

    // Clear previous scatter points for this tile
    scatter_points.points.remove(&tile_index);

    // Multiple lights for better visibility
    // Key light (main, from top-front-right) - brighter for better surface visibility
    let light1 = commands
        .spawn((
            PointLight {
                intensity: 800000.0,
                range: 100.0,
                color: Color::srgb(1.0, 0.98, 0.95),
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(8.0, 12.0, 8.0),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(light1);

    // Fill light (softer, from opposite side)
    let light2 = commands
        .spawn((
            PointLight {
                intensity: 350000.0,
                range: 100.0,
                color: Color::srgb(0.9, 0.95, 1.0),
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(-6.0, 8.0, -6.0),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(light2);

    // Rim light (from behind for edge definition)
    let light3 = commands
        .spawn((
            PointLight {
                intensity: 200000.0,
                range: 100.0,
                color: Color::srgb(1.0, 1.0, 1.0),
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, -5.0, -10.0),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(light3);

    // Collect scatter points for tooltip
    let mut all_points = Vec::new();

    // Draw each layer
    for layer_data in &graph.layers {
        if let Some(points) = draw_layer_3d(
            commands,
            root,
            meshes,
            materials,
            layers.clone(),
            layer_data,
            &bounds,
        ) {
            all_points.extend(points);
        }
    }

    // Store scatter points for tooltip lookup
    if !all_points.is_empty() {
        scatter_points.points.insert(tile_index, all_points);
    }

    // Draw 3D axes with labels (scaled to data)
    draw_3d_axes(commands, root, meshes, materials, layers);

    // Draw title and border on overlay layer (2D elements)
    draw_3d_title_and_border(
        commands,
        root,
        &graph.meta,
        rect,
        unit,
        color_materials,
        overlay_layers,
    );
}

/// Draw title and border for 3D plot using 2D overlay
fn draw_3d_title_and_border(
    commands: &mut Commands,
    root: Entity,
    meta: &crate::core::PlotMeta,
    rect: &TileRect,
    unit: &UnitMeshes,
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
        Color::srgba(0.3, 0.3, 0.35, 0.8),
        1.0,
    );

    // Title and description
    let title_y = rect.world_center.y + rect.world_size.y * 0.5 - 18.0;

    if let Some(title) = &meta.title {
        let title_entity = commands
            .spawn((
                Text2d::new(title.clone()),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                Transform::from_translation(Vec3::new(rect.world_center.x, title_y, 3.0)),
                layers.clone(),
            ))
            .id();
        commands.entity(root).add_child(title_entity);
    }

    if let Some(desc) = &meta.description {
        let desc_y = title_y - if meta.title.is_some() { 16.0 } else { 0.0 };
        let desc_entity = commands
            .spawn((
                Text2d::new(desc.clone()),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.85)),
                Transform::from_translation(Vec3::new(rect.world_center.x, desc_y, 3.0)),
                layers,
            ))
            .id();
        commands.entity(root).add_child(desc_entity);
    }
    // Note: Axis labels (X, Y, Z) are rendered dynamically by update_3d_axis_labels
    // at the actual axis tip positions in screen space
}

/// Compute bounding box for 3D data
fn compute_3d_bounds(layers: &[crate::core::Layer3D]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];

    for layer in layers {
        for pt in &layer.xyz {
            min[0] = min[0].min(pt.x);
            min[1] = min[1].min(pt.y);
            min[2] = min[2].min(pt.z);
            max[0] = max[0].max(pt.x);
            max[1] = max[1].max(pt.y);
            max[2] = max[2].max(pt.z);
        }
    }

    // Ensure we have valid bounds
    if min[0] >= max[0] {
        min[0] = 0.0;
        max[0] = 1.0;
    }
    if min[1] >= max[1] {
        min[1] = 0.0;
        max[1] = 1.0;
    }
    if min[2] >= max[2] {
        min[2] = 0.0;
        max[2] = 1.0;
    }

    (min, max)
}

/// Normalize a 3D point to fit within a standard viewing volume
fn normalize_point(pt: Vec3, bounds: &([f32; 3], [f32; 3])) -> Vec3 {
    let (min, max) = bounds;
    let scale = 5.0; // Size of normalized volume
    Vec3::new(
        (pt.x - min[0]) / (max[0] - min[0]) * scale - scale * 0.5,
        (pt.y - min[1]) / (max[1] - min[1]) * scale - scale * 0.5,
        (pt.z - min[2]) / (max[2] - min[2]) * scale - scale * 0.5,
    )
}

/// Draw a single 3D layer (points or surface)
/// Returns the list of (original, normalized) points if this is a Points layer
fn draw_layer_3d(
    commands: &mut Commands,
    root: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    layers: RenderLayers,
    layer_data: &crate::core::Layer3D,
    bounds: &([f32; 3], [f32; 3]),
) -> Option<Vec<(Vec3, Vec3)>> {
    let color = Color::srgba(
        layer_data.style.color.r,
        layer_data.style.color.g,
        layer_data.style.color.b,
        layer_data.style.opacity,
    );

    match layer_data.geometry {
        crate::core::Geometry3D::Points => {
            // Larger, more visible spheres
            let radius = (layer_data.style.size * 0.04).max(0.08);
            let sphere_mesh = meshes.add(Sphere::new(radius));
            // Subtle emissive for visibility without being too bright
            let emissive_color = Color::srgb(
                layer_data.style.color.r * 0.3,
                layer_data.style.color.g * 0.3,
                layer_data.style.color.b * 0.3,
            );
            let mat = materials.add(StandardMaterial {
                base_color: color,
                emissive: emissive_color.into(),
                perceptual_roughness: 0.4,
                metallic: 0.2,
                ..default()
            });

            let mut point_pairs = Vec::with_capacity(layer_data.xyz.len());

            for &pt in &layer_data.xyz {
                let normalized = normalize_point(pt, bounds);
                point_pairs.push((pt, normalized));
                // Spawn at world level, then parent to root for cleanup
                let entity = commands
                    .spawn((
                        Mesh3d(sphere_mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_translation(normalized),
                        layers.clone(),
                    ))
                    .id();
                commands.entity(root).add_child(entity);
            }

            Some(point_pairs)
        }

        crate::core::Geometry3D::Surface { grid } => {
            // Normalize all points for the surface
            let normalized_xyz: Vec<Vec3> = layer_data
                .xyz
                .iter()
                .map(|&pt| normalize_point(pt, bounds))
                .collect();

            let mesh = create_surface_mesh(&normalized_xyz, grid);
            let mesh_handle = meshes.add(mesh);
            // Surface material with slight emissive for better visibility
            let emissive_color = Color::srgb(
                layer_data.style.color.r * 0.15,
                layer_data.style.color.g * 0.15,
                layer_data.style.color.b * 0.15,
            );
            let mat = materials.add(StandardMaterial {
                base_color: color,
                emissive: emissive_color.into(),
                perceptual_roughness: 0.35,
                metallic: 0.05,
                reflectance: 0.4,
                cull_mode: None, // Render both sides
                double_sided: true,
                ..default()
            });

            // Spawn at world level, then parent to root for cleanup
            let entity = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(mat),
                    Transform::IDENTITY,
                    layers.clone(),
                ))
                .id();
            commands.entity(root).add_child(entity);

            None // Surface doesn't return individual points for tooltip
        }
    }
}

/// Draw 3D coordinate axes with grids on floor and back walls
/// Grid covers the normalized data volume [-2.5, 2.5] in each dimension
fn draw_3d_axes(
    commands: &mut Commands,
    root: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    layers: RenderLayers,
) {
    // The normalized data volume spans [-2.5, 2.5] (scale=5.0 centered at origin)
    let half_size = 2.5;
    let axis_len = 5.5; // Slightly longer than data to show axis tips
    let thickness = 0.025;
    let grid_step = 1.0;
    let origin = Vec3::new(-half_size, -half_size, -half_size);

    // Materials - brighter axes with glow
    let mat_x = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.3, 0.3),
        emissive: Color::srgb(0.8, 0.2, 0.2).into(),
        unlit: true,
        ..default()
    });
    let mat_y = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 1.0, 0.3),
        emissive: Color::srgb(0.2, 0.8, 0.2).into(),
        unlit: true,
        ..default()
    });
    let mat_z = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.5, 1.0),
        emissive: Color::srgb(0.2, 0.3, 0.8).into(),
        unlit: true,
        ..default()
    });
    let mat_grid = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.4, 0.45, 0.5),
        unlit: true,
        ..default()
    });
    let mat_label = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 1.0),
        emissive: Color::srgb(0.5, 0.5, 0.5).into(),
        unlit: true,
        ..default()
    });

    // X axis (red) - from origin extending in +X
    let mesh_x = meshes.add(Cuboid::new(axis_len, thickness, thickness));
    let x_axis = commands
        .spawn((
            Mesh3d(mesh_x),
            MeshMaterial3d(mat_x.clone()),
            Transform::from_translation(origin + Vec3::new(axis_len * 0.5, 0.0, 0.0)),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(x_axis);

    // X axis label (cone/arrow at tip)
    let x_tip = meshes.add(Sphere::new(0.12));
    let x_label = commands
        .spawn((
            Mesh3d(x_tip),
            MeshMaterial3d(mat_x),
            Transform::from_translation(origin + Vec3::new(axis_len + 0.15, 0.0, 0.0)),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(x_label);

    // Y axis (green)
    let mesh_y = meshes.add(Cuboid::new(thickness, axis_len, thickness));
    let y_axis = commands
        .spawn((
            Mesh3d(mesh_y),
            MeshMaterial3d(mat_y.clone()),
            Transform::from_translation(origin + Vec3::new(0.0, axis_len * 0.5, 0.0)),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(y_axis);

    // Y axis label
    let y_tip = meshes.add(Sphere::new(0.12));
    let y_label = commands
        .spawn((
            Mesh3d(y_tip),
            MeshMaterial3d(mat_y),
            Transform::from_translation(origin + Vec3::new(0.0, axis_len + 0.15, 0.0)),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(y_label);

    // Z axis (blue)
    let mesh_z = meshes.add(Cuboid::new(thickness, thickness, axis_len));
    let z_axis = commands
        .spawn((
            Mesh3d(mesh_z),
            MeshMaterial3d(mat_z.clone()),
            Transform::from_translation(origin + Vec3::new(0.0, 0.0, axis_len * 0.5)),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(z_axis);

    // Z axis label
    let z_tip = meshes.add(Sphere::new(0.12));
    let z_label = commands
        .spawn((
            Mesh3d(z_tip),
            MeshMaterial3d(mat_z),
            Transform::from_translation(origin + Vec3::new(0.0, 0.0, axis_len + 0.15)),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(z_label);

    // Origin marker (white sphere)
    let origin_mesh = meshes.add(Sphere::new(0.08));
    let origin_marker = commands
        .spawn((
            Mesh3d(origin_mesh),
            MeshMaterial3d(mat_label),
            Transform::from_translation(origin),
            layers.clone(),
        ))
        .id();
    commands.entity(root).add_child(origin_marker);

    // Grid parameters - cover the full [-2.5, 2.5] volume
    let grid_thick = thickness * 0.4;
    let full_len = half_size * 2.0; // 5.0
    let n_lines = (half_size / grid_step).ceil() as i32;

    // === XZ floor grid (y = -2.5) ===
    // Lines parallel to X
    for iz in -n_lines..=n_lines {
        let z = iz as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(full_len, grid_thick, grid_thick));
        let grid_line = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_grid.clone()),
                Transform::from_translation(Vec3::new(0.0, -half_size, z)),
                layers.clone(),
            ))
            .id();
        commands.entity(root).add_child(grid_line);
    }
    // Lines parallel to Z
    for ix in -n_lines..=n_lines {
        let x = ix as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(grid_thick, grid_thick, full_len));
        let grid_line = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_grid.clone()),
                Transform::from_translation(Vec3::new(x, -half_size, 0.0)),
                layers.clone(),
            ))
            .id();
        commands.entity(root).add_child(grid_line);
    }

    // === XY back wall grid (z = -2.5) ===
    // Lines parallel to X
    for iy in -n_lines..=n_lines {
        let y = iy as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(full_len, grid_thick, grid_thick));
        let grid_line = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_grid.clone()),
                Transform::from_translation(Vec3::new(0.0, y, -half_size)),
                layers.clone(),
            ))
            .id();
        commands.entity(root).add_child(grid_line);
    }
    // Lines parallel to Y
    for ix in -n_lines..=n_lines {
        let x = ix as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(grid_thick, full_len, grid_thick));
        let grid_line = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_grid.clone()),
                Transform::from_translation(Vec3::new(x, 0.0, -half_size)),
                layers.clone(),
            ))
            .id();
        commands.entity(root).add_child(grid_line);
    }

    // === YZ side wall grid (x = -2.5) ===
    // Lines parallel to Z
    for iy in -n_lines..=n_lines {
        let y = iy as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(grid_thick, grid_thick, full_len));
        let grid_line = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_grid.clone()),
                Transform::from_translation(Vec3::new(-half_size, y, 0.0)),
                layers.clone(),
            ))
            .id();
        commands.entity(root).add_child(grid_line);
    }
    // Lines parallel to Y
    for iz in -n_lines..=n_lines {
        let z = iz as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(grid_thick, full_len, grid_thick));
        let grid_line = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_grid.clone()),
                Transform::from_translation(Vec3::new(-half_size, 0.0, z)),
                layers.clone(),
            ))
            .id();
        commands.entity(root).add_child(grid_line);
    }
}

/// Create a mesh for a 3D surface from a grid of points
fn create_surface_mesh(vertices: &[Vec3], grid: bevy_math::UVec2) -> Mesh {
    let w = grid.x as usize;
    let h = grid.y as usize;

    let positions: Vec<[f32; 3]> = vertices.iter().map(|v| [v.x, v.y, v.z]).collect();

    let mut indices = Vec::new();
    for y in 0..h.saturating_sub(1) {
        for x in 0..w.saturating_sub(1) {
            let i0 = (y * w + x) as u32;
            let i1 = i0 + 1;
            let i2 = i0 + w as u32;
            let i3 = i2 + 1;

            // Two triangles per quad
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    let normals = compute_surface_normals(&positions, &indices);

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

/// Compute smooth vertex normals for a surface mesh
fn compute_surface_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![Vec3::ZERO; positions.len()];

    let pos = |i: usize| Vec3::new(positions[i][0], positions[i][1], positions[i][2]);

    for tri in indices.chunks_exact(3) {
        let a = tri[0] as usize;
        let b = tri[1] as usize;
        let c = tri[2] as usize;

        let e1 = pos(b) - pos(a);
        let e2 = pos(c) - pos(a);
        let n = e1.cross(e2);

        normals[a] += n;
        normals[b] += n;
        normals[c] += n;
    }

    normals
        .into_iter()
        .map(|n| {
            let n = n.normalize_or_zero();
            [n.x, n.y, n.z]
        })
        .collect()
}
