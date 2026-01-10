use bevy::prelude::*;
use bevy_camera::visibility::RenderLayers;

use crate::core::{Geometry3D, Layer3D};

use super::mesh::create_surface_mesh;

fn style_to_color(style: &crate::core::Style) -> Color {
    Color::srgba(style.color.r, style.color.g, style.color.b, style.color.a)
}

pub fn draw_3d_axes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    layer: RenderLayers,
) {
    let axis_len = 5.0;
    let axis_thick = 0.03;

    let mat_x = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.2, 0.2, 1.0),
        ..default()
    });
    let mesh_x = meshes.add(Cuboid::new(axis_len, axis_thick, axis_thick));
    commands.spawn((
        Mesh3d(mesh_x),
        MeshMaterial3d(mat_x),
        Transform::from_translation(Vec3::new(axis_len * 0.5, 0.0, 0.0)),
        layer.clone(),
        super::draw2d::Rendered,
    ));

    let mat_y = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.9, 0.2, 1.0),
        ..default()
    });
    let mesh_y = meshes.add(Cuboid::new(axis_thick, axis_len, axis_thick));
    commands.spawn((
        Mesh3d(mesh_y),
        MeshMaterial3d(mat_y),
        Transform::from_translation(Vec3::new(0.0, axis_len * 0.5, 0.0)),
        layer.clone(),
        super::draw2d::Rendered,
    ));

    let mat_z = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.4, 0.9, 1.0),
        ..default()
    });
    let mesh_z = meshes.add(Cuboid::new(axis_thick, axis_thick, axis_len));
    commands.spawn((
        Mesh3d(mesh_z),
        MeshMaterial3d(mat_z),
        Transform::from_translation(Vec3::new(0.0, 0.0, axis_len * 0.5)),
        layer,
        super::draw2d::Rendered,
    ));
}

pub fn draw_layer_3d(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    layer: RenderLayers,
    layer_data: &Layer3D,
) {
    let color = style_to_color(&layer_data.style);

    match layer_data.geometry {
        Geometry3D::Points => {
            let radius = (layer_data.style.size * 0.02).max(0.02);
            let sphere_mesh = meshes.add(Sphere::new(radius));
            let mat = materials.add(StandardMaterial {
                base_color: color,
                ..default()
            });

            for &pt in &layer_data.xyz {
                commands.spawn((
                    Mesh3d(sphere_mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(pt),
                    layer.clone(),
                    super::draw2d::Rendered,
                ));
            }
        }

        Geometry3D::Surface { grid } => {
            let mesh = create_surface_mesh(&layer_data.xyz, grid);
            let mesh_handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.9,
                cull_mode: None,
                ..default()
            });

            commands.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat),
                Transform::IDENTITY,
                layer,
                super::draw2d::Rendered,
            ));
        }
    }
}
