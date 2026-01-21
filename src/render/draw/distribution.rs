//! Distribution visualizations: histogram, PDF, boxplot, ECDF.

#![allow(clippy::too_many_arguments)]

use super::common::draw_tile_border;
use crate::render::{TileRect, TileView, UnitMeshes};
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_camera::visibility::RenderLayers;
use bevy_mesh::{Indices, PrimitiveTopology};

/// Draw a histogram.
pub fn draw_histogram(
    commands: &mut Commands,
    root: Entity,
    values: &[f32],
    bins: usize,
    style: &crate::core::Style,
    x_label: Option<&str>,
    y_label: Option<&str>,
    rect: &TileRect,
    _view: &TileView,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    if values.is_empty() || bins == 0 {
        return;
    }

    let min_val = values.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_val = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    if !min_val.is_finite() || !max_val.is_finite() || min_val >= max_val {
        return;
    }

    let bin_width = (max_val - min_val) / bins as f32;
    let mut counts = vec![0usize; bins];

    for &v in values {
        let idx = ((v - min_val) / bin_width).floor() as usize;
        let idx = idx.min(bins - 1);
        counts[idx] += 1;
    }

    let max_count = counts.iter().cloned().max().unwrap_or(1) as f32;

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

    let padding_left = 0.15;
    let padding_right = 0.08;
    let padding_bottom = 0.18;
    let padding_top = 0.08;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);

    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y =
        rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

    let bar_width_world = usable_width / bins as f32;
    let gap = bar_width_world * 0.1;

    let bar_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity);
    let bar_mat = materials.add(ColorMaterial::from(bar_color));

    for (i, &count) in counts.iter().enumerate() {
        if count == 0 {
            continue;
        }

        let bar_height = (count as f32 / max_count) * usable_height;
        let bar_x = left_x + (i as f32 + 0.5) * bar_width_world;
        let bar_y = bottom_y + bar_height * 0.5;

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(bar_mat.clone()),
                Transform {
                    translation: Vec3::new(bar_x, bar_y, 0.0),
                    scale: Vec3::new(bar_width_world - gap, bar_height, 1.0),
                    ..default()
                },
                layers.clone(),
            ));
        });
    }

    // X-axis tick labels
    let label_count = bins.min(8);
    let step = bins / label_count;
    for i in (0..=bins).step_by(step.max(1)) {
        let val = min_val + i as f32 * bin_width;
        let x = left_x + (i as f32 / bins as f32) * usable_width;

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.1}", val)),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(x, bottom_y - 14.0, 2.0)),
                layers.clone(),
            ));
        });
    }

    // Axis labels
    let x_label_text = x_label.unwrap_or("Value");
    let y_label_text = y_label.unwrap_or("Frequency");

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text2d::new(x_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform::from_translation(Vec3::new(
                left_x + usable_width * 0.5,
                rect.world_center.y - rect.world_size.y * 0.5 + 14.0,
                2.0,
            )),
            layers.clone(),
        ));

        parent.spawn((
            Text2d::new(y_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform {
                translation: Vec3::new(
                    rect.world_center.x - rect.world_size.x * 0.5 + 14.0,
                    bottom_y + usable_height * 0.5,
                    2.0,
                ),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            layers,
        ));
    });
}

/// Draw a probability density function (smooth KDE curve).
pub fn draw_pdf(
    commands: &mut Commands,
    root: Entity,
    values: &[f32],
    style: &crate::core::Style,
    x_label: Option<&str>,
    y_label: Option<&str>,
    rect: &TileRect,
    _view: &TileView,
    unit: &UnitMeshes,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    if values.is_empty() {
        return;
    }

    let min_val = values.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_val = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    if !min_val.is_finite() || !max_val.is_finite() || min_val >= max_val {
        return;
    }

    // Silverman's rule of thumb for bandwidth
    let n = values.len() as f32;
    let std_dev = {
        let mean = values.iter().sum::<f32>() / n;
        let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
        variance.sqrt()
    };
    let bandwidth = (1.06 * std_dev * n.powf(-0.2)).max(0.01);

    // Sample the KDE
    let n_samples = 200;
    let range = max_val - min_val;
    let x_min = min_val - range * 0.1;
    let x_max = max_val + range * 0.1;

    let mut kde_points: Vec<(f32, f32)> = Vec::with_capacity(n_samples);
    let mut max_density = 0.0f32;

    for i in 0..n_samples {
        let x = x_min + (i as f32 / (n_samples - 1) as f32) * (x_max - x_min);
        let density: f32 = values
            .iter()
            .map(|&xi| {
                let u = (x - xi) / bandwidth;
                (-0.5 * u * u).exp() / (2.506628 * bandwidth)
            })
            .sum::<f32>()
            / n;

        kde_points.push((x, density));
        max_density = max_density.max(density);
    }

    if max_density <= 0.0 {
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

    let padding_left = 0.15;
    let padding_right = 0.08;
    let padding_bottom = 0.18;
    let padding_top = 0.08;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y =
        rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

    let upper: Vec<Vec2> = kde_points
        .iter()
        .map(|&(x, y)| {
            let wx = left_x + ((x - x_min) / (x_max - x_min)) * usable_width;
            let wy = bottom_y + (y / max_density) * usable_height;
            Vec2::new(wx, wy)
        })
        .collect();

    let lower: Vec<Vec2> = kde_points
        .iter()
        .map(|&(x, _)| {
            let wx = left_x + ((x - x_min) / (x_max - x_min)) * usable_width;
            Vec2::new(wx, bottom_y)
        })
        .collect();

    // Draw filled area under curve
    let n_pts = upper.len();
    if n_pts >= 2 {
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n_pts * 2);
        let mut indices: Vec<u32> = Vec::with_capacity((n_pts - 1) * 6);

        for pt in &upper {
            positions.push([pt.x, pt.y, 0.0]);
        }
        for pt in &lower {
            positions.push([pt.x, pt.y, 0.0]);
        }

        for i in 0..(n_pts - 1) {
            let u0 = i as u32;
            let u1 = (i + 1) as u32;
            let l0 = (n_pts + i) as u32;
            let l1 = (n_pts + i + 1) as u32;
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
                Transform::default(),
                layers.clone(),
            ));
        });
    }

    // Draw the PDF line on top
    let line_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity);
    let line_mat = materials.add(ColorMaterial::from(line_color));

    for window in upper.windows(2) {
        let a = window[0];
        let b = window[1];
        let length = a.distance(b);
        let angle = (b.y - a.y).atan2(b.x - a.x);

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(line_mat.clone()),
                Transform {
                    translation: ((a + b) * 0.5).extend(0.1),
                    rotation: Quat::from_rotation_z(angle),
                    scale: Vec3::new(length, style.size, 1.0),
                    ..default()
                },
                layers.clone(),
            ));
        });
    }

    // X-axis tick labels
    let n_ticks = 5;
    for i in 0..=n_ticks {
        let t = i as f32 / n_ticks as f32;
        let val = x_min + t * (x_max - x_min);
        let x = left_x + t * usable_width;

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.1}", val)),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(x, bottom_y - 12.0, 2.0)),
                layers.clone(),
            ));
        });
    }

    // Axis labels
    let x_label_text = x_label.unwrap_or("Value");
    let y_label_text = y_label.unwrap_or("Density");

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text2d::new(x_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform::from_translation(Vec3::new(
                left_x + usable_width * 0.5,
                rect.world_center.y - rect.world_size.y * 0.5 + 14.0,
                2.0,
            )),
            layers.clone(),
        ));

        parent.spawn((
            Text2d::new(y_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform {
                translation: Vec3::new(
                    rect.world_center.x - rect.world_size.x * 0.5 + 14.0,
                    bottom_y + usable_height * 0.5,
                    2.0,
                ),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            layers,
        ));
    });
}

/// Box plot statistics.
struct BoxStats {
    min: f32,
    q1: f32,
    median: f32,
    q3: f32,
    max: f32,
    outliers: Vec<f32>,
}

fn compute_box_stats(values: &[f32]) -> Option<BoxStats> {
    if values.is_empty() {
        return None;
    }

    let mut sorted: Vec<f32> = values.iter().cloned().filter(|x| x.is_finite()).collect();
    if sorted.is_empty() {
        return None;
    }

    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len();

    let median = if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };

    let q1_idx = n / 4;
    let q3_idx = 3 * n / 4;
    let q1 = sorted[q1_idx];
    let q3 = sorted[q3_idx.min(n - 1)];

    let iqr = q3 - q1;
    let lower_fence = q1 - 1.5 * iqr;
    let upper_fence = q3 + 1.5 * iqr;

    let min = sorted
        .iter()
        .cloned()
        .find(|&x| x >= lower_fence)
        .unwrap_or(q1);
    let max = sorted
        .iter()
        .rev()
        .cloned()
        .find(|&x| x <= upper_fence)
        .unwrap_or(q3);

    let outliers: Vec<f32> = sorted
        .iter()
        .cloned()
        .filter(|&x| x < lower_fence || x > upper_fence)
        .collect();

    Some(BoxStats {
        min,
        q1,
        median,
        q3,
        max,
        outliers,
    })
}

/// Draw a boxplot for grouped data.
pub fn draw_boxplot(
    commands: &mut Commands,
    root: Entity,
    groups: &[(String, Vec<f32>)],
    style: &crate::core::Style,
    x_label: Option<&str>,
    y_label: Option<&str>,
    rect: &TileRect,
    _view: &TileView,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    if groups.is_empty() {
        return;
    }

    let stats: Vec<Option<BoxStats>> = groups.iter().map(|(_, v)| compute_box_stats(v)).collect();

    // Find global y range
    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;
    for s in stats.iter().flatten() {
        y_min = y_min.min(s.min);
        y_max = y_max.max(s.max);
        for &o in &s.outliers {
            y_min = y_min.min(o);
            y_max = y_max.max(o);
        }
    }

    if !y_min.is_finite() || !y_max.is_finite() {
        return;
    }

    let y_range = y_max - y_min;
    let y_min = y_min - y_range * 0.1;
    let y_max = y_max + y_range * 0.1;

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

    let padding_left = 0.12;
    let padding_right = 0.05;
    let padding_bottom = 0.18;
    let padding_top = 0.08;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y =
        rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

    let to_world_y = |y: f32| -> f32 { bottom_y + ((y - y_min) / (y_max - y_min)) * usable_height };

    let n_groups = groups.len();
    let group_width = usable_width / n_groups as f32;
    let box_width = group_width * 0.6;

    let box_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity);
    let box_mat = materials.add(ColorMaterial::from(box_color));
    let median_mat = materials.add(ColorMaterial::from(Color::srgba(1.0, 0.8, 0.2, 1.0)));
    let whisker_mat = materials.add(ColorMaterial::from(Color::srgba(0.7, 0.7, 0.7, 0.9)));
    let outlier_mat = materials.add(ColorMaterial::from(Color::srgba(0.9, 0.3, 0.3, 0.8)));

    for (i, (label, _)) in groups.iter().enumerate() {
        let cx = left_x + (i as f32 + 0.5) * group_width;

        if let Some(ref s) = stats[i] {
            let y_q1 = to_world_y(s.q1);
            let y_q3 = to_world_y(s.q3);
            let y_med = to_world_y(s.median);
            let y_min_w = to_world_y(s.min);
            let y_max_w = to_world_y(s.max);

            commands.entity(root).with_children(|parent| {
                // Box (Q1 to Q3)
                let box_height = (y_q3 - y_q1).max(2.0);
                parent.spawn((
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(box_mat.clone()),
                    Transform {
                        translation: Vec3::new(cx, (y_q1 + y_q3) * 0.5, 0.0),
                        scale: Vec3::new(box_width, box_height, 1.0),
                        ..default()
                    },
                    layers.clone(),
                ));

                // Median line
                parent.spawn((
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(median_mat.clone()),
                    Transform {
                        translation: Vec3::new(cx, y_med, 0.1),
                        scale: Vec3::new(box_width, 3.0, 1.0),
                        ..default()
                    },
                    layers.clone(),
                ));

                // Lower whisker
                let lower_whisker_height = y_q1 - y_min_w;
                if lower_whisker_height > 1.0 {
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(whisker_mat.clone()),
                        Transform {
                            translation: Vec3::new(cx, (y_q1 + y_min_w) * 0.5, 0.0),
                            scale: Vec3::new(2.0, lower_whisker_height, 1.0),
                            ..default()
                        },
                        layers.clone(),
                    ));
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(whisker_mat.clone()),
                        Transform {
                            translation: Vec3::new(cx, y_min_w, 0.0),
                            scale: Vec3::new(box_width * 0.5, 2.0, 1.0),
                            ..default()
                        },
                        layers.clone(),
                    ));
                }

                // Upper whisker
                let upper_whisker_height = y_max_w - y_q3;
                if upper_whisker_height > 1.0 {
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(whisker_mat.clone()),
                        Transform {
                            translation: Vec3::new(cx, (y_q3 + y_max_w) * 0.5, 0.0),
                            scale: Vec3::new(2.0, upper_whisker_height, 1.0),
                            ..default()
                        },
                        layers.clone(),
                    ));
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(whisker_mat.clone()),
                        Transform {
                            translation: Vec3::new(cx, y_max_w, 0.0),
                            scale: Vec3::new(box_width * 0.5, 2.0, 1.0),
                            ..default()
                        },
                        layers.clone(),
                    ));
                }

                // Outliers
                for &o in &s.outliers {
                    let oy = to_world_y(o);
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(outlier_mat.clone()),
                        Transform {
                            translation: Vec3::new(cx, oy, 0.2),
                            scale: Vec3::splat(5.0),
                            ..default()
                        },
                        layers.clone(),
                    ));
                }
            });
        }

        // Group label
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(label.clone()),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                Transform::from_translation(Vec3::new(cx, bottom_y - 12.0, 2.0)),
                layers.clone(),
            ));
        });
    }

    // Y-axis tick labels
    let n_ticks = 5;
    for i in 0..=n_ticks {
        let t = i as f32 / n_ticks as f32;
        let val = y_min + t * (y_max - y_min);
        let y = to_world_y(val);

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.1}", val)),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(left_x - 20.0, y, 2.0)),
                layers.clone(),
            ));
        });
    }

    // Axis labels
    let x_label_text = x_label.unwrap_or("Group");
    let y_label_text = y_label.unwrap_or("Value");

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text2d::new(x_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform::from_translation(Vec3::new(
                left_x + usable_width * 0.5,
                rect.world_center.y - rect.world_size.y * 0.5 + 14.0,
                2.0,
            )),
            layers.clone(),
        ));

        parent.spawn((
            Text2d::new(y_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform {
                translation: Vec3::new(
                    rect.world_center.x - rect.world_size.x * 0.5 + 10.0,
                    bottom_y + usable_height * 0.5,
                    2.0,
                ),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            layers,
        ));
    });
}

/// Draw an ECDF (Empirical Cumulative Distribution Function).
pub fn draw_ecdf(
    commands: &mut Commands,
    root: Entity,
    values: &[f32],
    style: &crate::core::Style,
    x_label: Option<&str>,
    y_label: Option<&str>,
    rect: &TileRect,
    _view: &TileView,
    unit: &UnitMeshes,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    if values.is_empty() {
        return;
    }

    let mut sorted: Vec<f32> = values.iter().cloned().filter(|x| x.is_finite()).collect();
    if sorted.is_empty() {
        return;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let min_val = sorted[0];
    let max_val = sorted[sorted.len() - 1];

    if min_val >= max_val {
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

    let padding_left = 0.15;
    let padding_right = 0.08;
    let padding_bottom = 0.18;
    let padding_top = 0.08;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y =
        rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

    let n = sorted.len();

    // Build step function points
    let mut step_points: Vec<Vec2> = Vec::with_capacity(n * 2 + 2);

    // Start at (min_val, 0)
    let start_x = left_x;
    step_points.push(Vec2::new(start_x, bottom_y));

    for (i, &val) in sorted.iter().enumerate() {
        let x = left_x + ((val - min_val) / (max_val - min_val)) * usable_width;
        let prev_y = (i as f32 / n as f32) * usable_height + bottom_y;
        let next_y = ((i + 1) as f32 / n as f32) * usable_height + bottom_y;

        // Horizontal to current x at previous y
        step_points.push(Vec2::new(x, prev_y));
        // Vertical step up
        step_points.push(Vec2::new(x, next_y));
    }

    // Draw filled area under step function
    if step_points.len() >= 2 {
        let n_pts = step_points.len();
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n_pts * 2);
        let mut indices: Vec<u32> = Vec::with_capacity((n_pts - 1) * 6);

        for pt in &step_points {
            positions.push([pt.x, pt.y, 0.0]);
        }
        for pt in &step_points {
            positions.push([pt.x, bottom_y, 0.0]);
        }

        for i in 0..(n_pts - 1) {
            let u0 = i as u32;
            let u1 = (i + 1) as u32;
            let l0 = (n_pts + i) as u32;
            let l1 = (n_pts + i + 1) as u32;
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
            style.color.r,
            style.color.g,
            style.color.b,
            style.opacity * 0.3,
        );
        let fill_mat = materials.add(ColorMaterial::from(fill_color));

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(fill_mesh),
                MeshMaterial2d(fill_mat),
                Transform::default(),
                layers.clone(),
            ));
        });
    }

    // Draw step line
    let line_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity);
    let line_mat = materials.add(ColorMaterial::from(line_color));

    for window in step_points.windows(2) {
        let a = window[0];
        let b = window[1];
        let length = a.distance(b);
        if length < 0.5 {
            continue;
        }
        let angle = (b.y - a.y).atan2(b.x - a.x);

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(line_mat.clone()),
                Transform {
                    translation: ((a + b) * 0.5).extend(0.1),
                    rotation: Quat::from_rotation_z(angle),
                    scale: Vec3::new(length, style.size, 1.0),
                    ..default()
                },
                layers.clone(),
            ));
        });
    }

    // X-axis tick labels
    let n_ticks = 5;
    for i in 0..=n_ticks {
        let t = i as f32 / n_ticks as f32;
        let val = min_val + t * (max_val - min_val);
        let x = left_x + t * usable_width;

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.1}", val)),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(x, bottom_y - 12.0, 2.0)),
                layers.clone(),
            ));
        });
    }

    // Y-axis tick labels (0 to 1)
    for i in 0..=4 {
        let t = i as f32 / 4.0;
        let y = bottom_y + t * usable_height;

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.1}", t)),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(left_x - 18.0, y, 2.0)),
                layers.clone(),
            ));
        });
    }

    // Axis labels
    let x_label_text = x_label.unwrap_or("Value");
    let y_label_text = y_label.unwrap_or("Cumulative Probability");

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text2d::new(x_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform::from_translation(Vec3::new(
                left_x + usable_width * 0.5,
                rect.world_center.y - rect.world_size.y * 0.5 + 14.0,
                2.0,
            )),
            layers.clone(),
        ));

        parent.spawn((
            Text2d::new(y_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform {
                translation: Vec3::new(
                    rect.world_center.x - rect.world_size.x * 0.5 + 10.0,
                    bottom_y + usable_height * 0.5,
                    2.0,
                ),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            layers,
        ));
    });
}
