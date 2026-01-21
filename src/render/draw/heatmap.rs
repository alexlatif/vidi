//! Heatmap visualization with colormaps.

#![allow(clippy::too_many_arguments)]

use super::common::draw_tile_border;
use crate::render::{TileRect, TileView, UnitMeshes};
use bevy::prelude::*;
use bevy_camera::visibility::RenderLayers;

/// Draw a heatmap with optional row/column labels and value annotations.
pub fn draw_heatmap(
    commands: &mut Commands,
    root: Entity,
    heatmap: &crate::core::Heatmap,
    rect: &TileRect,
    _view: &TileView,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    let rows = heatmap.dims.y as usize;
    let cols = heatmap.dims.x as usize;

    if rows == 0 || cols == 0 || heatmap.values.is_empty() {
        return;
    }

    // Compute value range
    let vmin = heatmap
        .vmin
        .unwrap_or_else(|| heatmap.values.iter().cloned().fold(f32::INFINITY, f32::min));
    let vmax = heatmap.vmax.unwrap_or_else(|| {
        heatmap
            .values
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max)
    });
    let vrange = (vmax - vmin).max(0.001);

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

    // Padding for labels
    let has_row_labels = heatmap.row_labels.is_some();
    let has_col_labels = heatmap.col_labels.is_some();

    let padding_left = if has_row_labels { 0.12 } else { 0.06 };
    let padding_right = 0.06;
    let padding_bottom = if has_col_labels { 0.12 } else { 0.06 };
    let padding_top = 0.06;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y =
        rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

    let cell_width = usable_width / cols as f32;
    let cell_height = usable_height / rows as f32;

    // Draw cells
    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            if idx >= heatmap.values.len() {
                continue;
            }

            let value = heatmap.values[idx];
            let t = ((value - vmin) / vrange).clamp(0.0, 1.0);
            let color = heatmap.colormap.sample(t);

            let cell_x = left_x + (col as f32 + 0.5) * cell_width;
            let cell_y = bottom_y + usable_height - (row as f32 + 0.5) * cell_height;

            let cell_color = Color::srgba(color.r, color.g, color.b, color.a);
            let cell_mat = materials.add(ColorMaterial::from(cell_color));

            commands.entity(root).with_children(|parent| {
                parent.spawn((
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(cell_mat),
                    Transform {
                        translation: Vec3::new(cell_x, cell_y, 0.0),
                        scale: Vec3::new(cell_width - 1.0, cell_height - 1.0, 1.0),
                        ..default()
                    },
                    layers.clone(),
                ));

                // Show value in cell if enabled
                if heatmap.show_values && cell_width > 20.0 && cell_height > 15.0 {
                    let text_color = if t > 0.5 {
                        Color::srgba(0.0, 0.0, 0.0, 0.9)
                    } else {
                        Color::srgba(1.0, 1.0, 1.0, 0.9)
                    };
                    parent.spawn((
                        Text2d::new(format!("{:.1}", value)),
                        TextFont {
                            font_size: (cell_height * 0.4).min(12.0).max(6.0),
                            ..default()
                        },
                        TextColor(text_color),
                        Transform::from_translation(Vec3::new(cell_x, cell_y, 0.5)),
                        layers.clone(),
                    ));
                }
            });
        }
    }

    // Draw row labels
    if let Some(ref row_labels) = heatmap.row_labels {
        let label_x = left_x - 12.0;
        for (row, label) in row_labels.iter().enumerate().take(rows) {
            let y = bottom_y + usable_height - (row as f32 + 0.5) * cell_height;

            commands.entity(root).with_children(|parent| {
                parent.spawn((
                    Text2d::new(label.clone()),
                    TextFont {
                        font_size: 9.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                    Transform::from_translation(Vec3::new(label_x, y, 2.0)),
                    layers.clone(),
                ));
            });
        }
    }

    // Draw column labels
    if let Some(ref col_labels) = heatmap.col_labels {
        let label_y = bottom_y - 15.0;
        for (col, label) in col_labels.iter().enumerate().take(cols) {
            let x = left_x + (col as f32 + 0.5) * cell_width;

            commands.entity(root).with_children(|parent| {
                parent.spawn((
                    Text2d::new(label.clone()),
                    TextFont {
                        font_size: 9.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                    Transform {
                        translation: Vec3::new(x, label_y, 2.0),
                        rotation: Quat::from_rotation_z(-0.4),
                        ..default()
                    },
                    layers.clone(),
                ));
            });
        }
    }
}
