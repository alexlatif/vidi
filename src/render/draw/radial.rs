//! Radial visualizations: pie charts and radar/spider charts.

#![allow(clippy::too_many_arguments)]

use super::common::draw_tile_border;
use crate::render::{TileRect, UnitMeshes};
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_camera::visibility::RenderLayers;
use bevy_mesh::{Indices, PrimitiveTopology};

/// Draw a radial chart (pie or radar).
pub fn draw_radial(
    commands: &mut Commands,
    root: Entity,
    radial: &crate::core::Radial,
    rect: &TileRect,
    unit: &UnitMeshes,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    match radial {
        crate::core::Radial::Pie { slices, .. } => {
            draw_pie(
                commands, root, slices, rect, unit, meshes, materials, layers,
            );
        }
        crate::core::Radial::Radar {
            axes,
            values,
            style,
            ..
        } => {
            draw_radar(
                commands, root, axes, values, style, rect, unit, meshes, materials, layers,
            );
        }
    }
}

fn draw_pie(
    commands: &mut Commands,
    root: Entity,
    slices: &[(String, f32)],
    rect: &TileRect,
    unit: &UnitMeshes,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    if slices.is_empty() {
        return;
    }

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

    let total: f32 = slices.iter().map(|(_, v)| v.max(0.0)).sum();
    if total <= 0.0 {
        return;
    }

    let center = rect.world_center;
    let radius = (rect.world_size.x.min(rect.world_size.y) * 0.35).max(10.0);

    let colors = [
        Color::srgba(0.3, 0.6, 0.9, 0.9),
        Color::srgba(0.9, 0.4, 0.3, 0.9),
        Color::srgba(0.4, 0.8, 0.4, 0.9),
        Color::srgba(0.9, 0.7, 0.2, 0.9),
        Color::srgba(0.7, 0.4, 0.9, 0.9),
        Color::srgba(0.3, 0.8, 0.8, 0.9),
        Color::srgba(0.9, 0.5, 0.7, 0.9),
        Color::srgba(0.6, 0.6, 0.6, 0.9),
    ];

    let mut start_angle = -std::f32::consts::FRAC_PI_2;
    let segments_per_slice = 32;

    for (i, (label, value)) in slices.iter().enumerate() {
        if *value <= 0.0 {
            continue;
        }

        let sweep = (*value / total) * std::f32::consts::TAU;
        let color = colors[i % colors.len()];

        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        positions.push([center.x, center.y, 0.0]);

        for j in 0..=segments_per_slice {
            let angle = start_angle + (j as f32 / segments_per_slice as f32) * sweep;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            positions.push([x, y, 0.0]);
        }

        for j in 0..segments_per_slice {
            indices.push(0);
            indices.push((j + 1) as u32);
            indices.push((j + 2) as u32);
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

        let slice_mesh = meshes.add(mesh);
        let slice_mat = materials.add(ColorMaterial::from(color));

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(slice_mesh),
                MeshMaterial2d(slice_mat),
                Transform::from_translation(Vec3::ZERO),
                layers.clone(),
            ));
        });

        // Draw label
        let mid_angle = start_angle + sweep * 0.5;
        let label_radius = radius * 1.25;
        let label_x = center.x + label_radius * mid_angle.cos();
        let label_y = center.y + label_radius * mid_angle.sin();

        let pct = (*value / total) * 100.0;
        let label_text = format!("{}\n{:.1}%", label, pct);

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(label_text),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.9, 0.9, 1.0)),
                Transform::from_translation(Vec3::new(label_x, label_y, 2.0)),
                layers.clone(),
            ));
        });

        start_angle += sweep;
    }
}

fn draw_radar(
    commands: &mut Commands,
    root: Entity,
    axes: &[String],
    values: &[f32],
    style: &crate::core::Style,
    rect: &TileRect,
    unit: &UnitMeshes,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    if axes.is_empty() || values.is_empty() {
        return;
    }

    let n = axes.len().min(values.len());
    if n < 3 {
        return;
    }

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

    let center = rect.world_center;
    let radius = (rect.world_size.x.min(rect.world_size.y) * 0.35).max(10.0);
    let angle_step = std::f32::consts::TAU / n as f32;

    // Draw grid circles
    let grid_mat = materials.add(ColorMaterial::from(Color::srgba(0.4, 0.4, 0.5, 0.5)));

    for ring in 1..=4 {
        let r = radius * (ring as f32 / 4.0);
        let segments = 32;

        for j in 0..segments {
            let a1 = (j as f32 / segments as f32) * std::f32::consts::TAU;
            let a2 = ((j + 1) as f32 / segments as f32) * std::f32::consts::TAU;

            let p1 = Vec2::new(center.x + r * a1.cos(), center.y + r * a1.sin());
            let p2 = Vec2::new(center.x + r * a2.cos(), center.y + r * a2.sin());

            let length = p1.distance(p2);
            let angle = (p2.y - p1.y).atan2(p2.x - p1.x);
            let mid = (p1 + p2) * 0.5;

            commands.entity(root).with_children(|parent| {
                parent.spawn((
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(grid_mat.clone()),
                    Transform {
                        translation: mid.extend(0.0),
                        rotation: Quat::from_rotation_z(angle),
                        scale: Vec3::new(length, 1.0, 1.0),
                        ..default()
                    },
                    layers.clone(),
                ));
            });
        }
    }

    // Draw axis lines and labels
    let axis_mat = materials.add(ColorMaterial::from(Color::srgba(0.5, 0.5, 0.6, 0.7)));

    for i in 0..n {
        let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * angle_step;
        let mid = Vec2::new(
            center.x + radius * 0.5 * angle.cos(),
            center.y + radius * 0.5 * angle.sin(),
        );

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(axis_mat.clone()),
                Transform {
                    translation: mid.extend(0.1),
                    rotation: Quat::from_rotation_z(angle),
                    scale: Vec3::new(radius, 1.5, 1.0),
                    ..default()
                },
                layers.clone(),
            ));
        });

        let label_x = center.x + (radius + 20.0) * angle.cos();
        let label_y = center.y + (radius + 20.0) * angle.sin();

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(axes[i].clone()),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                Transform::from_translation(Vec3::new(label_x, label_y, 2.0)),
                layers.clone(),
            ));
        });
    }

    // Build data polygon mesh
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    positions.push([center.x, center.y, 0.0]);

    for i in 0..n {
        let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * angle_step;
        let r = radius * values[i].clamp(0.0, 1.0);
        let x = center.x + r * angle.cos();
        let y = center.y + r * angle.sin();
        positions.push([x, y, 0.0]);
    }

    for i in 0..n {
        let next = (i + 1) % n;
        indices.push(0);
        indices.push((i + 1) as u32);
        indices.push((next + 1) as u32);
    }

    let vertex_count = positions.len();
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; vertex_count];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; vertex_count];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    let fill_mesh = meshes.add(mesh);
    let fill_color = Color::srgba(
        style.color.r,
        style.color.g,
        style.color.b,
        style.opacity * 0.4,
    );
    let fill_mat = materials.add(ColorMaterial::from(fill_color));

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Mesh2d(fill_mesh),
            MeshMaterial2d(fill_mat),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.2)),
            layers.clone(),
        ));
    });

    // Draw data outline
    let line_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity);
    let line_mat = materials.add(ColorMaterial::from(line_color));

    for i in 0..n {
        let next = (i + 1) % n;
        let p1 = Vec2::new(positions[i + 1][0], positions[i + 1][1]);
        let p2 = Vec2::new(positions[next + 1][0], positions[next + 1][1]);

        let length = p1.distance(p2);
        let angle = (p2.y - p1.y).atan2(p2.x - p1.x);
        let mid = (p1 + p2) * 0.5;

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(line_mat.clone()),
                Transform {
                    translation: mid.extend(0.3),
                    rotation: Quat::from_rotation_z(angle),
                    scale: Vec3::new(length, style.size, 1.0),
                    ..default()
                },
                layers.clone(),
            ));
        });
    }

    // Draw data points
    let point_mat = materials.add(ColorMaterial::from(line_color));
    for i in 1..=n {
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(point_mat.clone()),
                Transform {
                    translation: Vec3::new(positions[i][0], positions[i][1], 0.4),
                    scale: Vec3::splat(6.0),
                    ..default()
                },
                layers.clone(),
            ));
        });
    }
}
