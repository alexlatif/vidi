use crate::render::{TileRect, TileView, UnitMeshes};
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_camera::visibility::RenderLayers;
use bevy_mesh::{Indices, PrimitiveTopology};

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
    // Draw border around tile
    let border_mat = materials.add(ColorMaterial::from(Color::srgb(0.3, 0.3, 0.4)));
    let border_thickness = 2.0;
    let layers_border = layers.clone();

    commands.entity(root).with_children(|parent| {
        // Top border
        parent.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(border_mat.clone()),
            Transform {
                translation: Vec3::new(
                    rect.world_center.x,
                    rect.world_center.y + rect.world_size.y * 0.5,
                    1.0,
                ),
                scale: Vec3::new(rect.world_size.x, border_thickness, 1.0),
                ..default()
            },
            layers_border.clone(),
        ));
        // Bottom border
        parent.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(border_mat.clone()),
            Transform {
                translation: Vec3::new(
                    rect.world_center.x,
                    rect.world_center.y - rect.world_size.y * 0.5,
                    1.0,
                ),
                scale: Vec3::new(rect.world_size.x, border_thickness, 1.0),
                ..default()
            },
            layers_border.clone(),
        ));
        // Left border
        parent.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(border_mat.clone()),
            Transform {
                translation: Vec3::new(
                    rect.world_center.x - rect.world_size.x * 0.5,
                    rect.world_center.y,
                    1.0,
                ),
                scale: Vec3::new(border_thickness, rect.world_size.y, 1.0),
                ..default()
            },
            layers_border.clone(),
        ));
        // Right border
        parent.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(border_mat),
            Transform {
                translation: Vec3::new(
                    rect.world_center.x + rect.world_size.x * 0.5,
                    rect.world_center.y,
                    1.0,
                ),
                scale: Vec3::new(border_thickness, rect.world_size.y, 1.0),
                ..default()
            },
            layers_border,
        ));
    });

    // Draw axis at data origin (0,0) - moves with pan/zoom
    let axis_mat = materials.add(ColorMaterial::from(Color::srgb(0.5, 0.5, 0.5)));
    let axis_origin = data_to_world(Vec2::ZERO, rect, view);
    let layers_axis = layers.clone();

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
            layers_axis.clone(),
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
            layers_axis,
        ));
    });

    // Draw axis labels
    let layers_label = layers.clone();
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
                layers_label.clone(),
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
                layers_label.clone(),
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
                if layer.xy.len() >= 2 {
                    for window in layer.xy.windows(2) {
                        let a = data_to_world(window[0], rect, view);
                        let b = data_to_world(window[1], rect, view);

                        let length = a.distance(b);
                        let angle = (b.y - a.y).atan2(b.x - a.x);

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
            crate::core::Geometry2D::Points => {
                // Fixed pixel size - do NOT scale with view.scale
                let point_size = layer.style.size;
                for &pt in &layer.xy {
                    let world_pos = data_to_world(pt, rect, view);
                    let layers_pt = layers.clone();

                    commands.entity(root).with_children(|parent| {
                        parent.spawn((
                            Mesh2d(unit.quad.clone()),
                            MeshMaterial2d(mat.clone()),
                            Transform {
                                translation: world_pos.extend(0.1),
                                scale: Vec3::splat(point_size),
                                ..default()
                            },
                            layers_pt,
                        ));
                    });
                }
            }
            crate::core::Geometry2D::FillBetween => {
                if let Some(lower) = &layer.lower_line {
                    let upper = &layer.xy;
                    let n = upper.len().min(lower.len());

                    if n >= 2 {
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
                        // upper[i] is at index i, lower[i] is at index n+i
                        for i in 0..(n - 1) {
                            let u0 = i as u32;
                            let u1 = (i + 1) as u32;
                            let l0 = (n + i) as u32;
                            let l1 = (n + i + 1) as u32;

                            // Two triangles per segment (CCW winding for front-facing)
                            indices.extend_from_slice(&[u0, l1, l0]); // lower-left triangle
                            indices.extend_from_slice(&[u0, u1, l1]); // upper-right triangle
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

                        let layers_fill = layers.clone();
                        commands.entity(root).with_children(|parent| {
                            parent.spawn((
                                Mesh2d(fill_mesh),
                                MeshMaterial2d(fill_mat),
                                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                                layers_fill,
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

/// Convert world coordinates to data coordinates
fn world_to_data(world: Vec2, rect: &TileRect, view: &TileView) -> Vec2 {
    (world - rect.world_center - view.offset) / view.scale
}

/// Calculate nice tick step for given range
fn nice_step(range: f32, target_ticks: usize) -> f32 {
    if range <= 0.0 || !range.is_finite() {
        return 1.0;
    }
    let rough = range / target_ticks as f32;
    let exp = rough.log10().floor();
    let base = 10f32.powf(exp);

    let normalized = rough / base;
    let nice = if normalized <= 1.5 {
        1.0
    } else if normalized <= 3.0 {
        2.0
    } else if normalized <= 7.0 {
        5.0
    } else {
        10.0
    };

    (nice * base).max(0.001)
}

/// Format tick value for display
fn format_tick(val: f32) -> String {
    if val.abs() < 0.001 && val != 0.0 {
        format!("{:.1e}", val)
    } else if val.abs() >= 1000.0 {
        format!("{:.1e}", val)
    } else if val.fract().abs() < 0.001 {
        format!("{:.0}", val)
    } else if val.abs() < 1.0 {
        format!("{:.2}", val)
    } else {
        format!("{:.1}", val)
    }
}

/// Draw axis tick marks and value labels
pub fn draw_axis_ticks(
    commands: &mut Commands,
    root: Entity,
    rect: &TileRect,
    view: &TileView,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    let tick_color = Color::srgba(0.5, 0.5, 0.55, 0.6);
    let tick_mat = materials.add(ColorMaterial::from(tick_color));
    let tick_length = 6.0;
    let tick_width = 1.0;

    // Calculate visible data range
    let half_size = rect.world_size * 0.5;
    let data_min = world_to_data(rect.world_center - half_size, rect, view);
    let data_max = world_to_data(rect.world_center + half_size, rect, view);

    // Compute nice tick spacing
    let x_step = nice_step(data_max.x - data_min.x, 8);
    let y_step = nice_step(data_max.y - data_min.y, 6);

    // Get axis origin in world coords (clamped to visible area)
    let origin_data = Vec2::ZERO;
    let origin_world = data_to_world(origin_data, rect, view);
    let clamped_origin_y = origin_world.y.clamp(
        rect.world_center.y - half_size.y + 20.0,
        rect.world_center.y + half_size.y - 20.0,
    );
    let clamped_origin_x = origin_world.x.clamp(
        rect.world_center.x - half_size.x + 30.0,
        rect.world_center.x + half_size.x - 30.0,
    );

    // Draw X-axis ticks and labels
    let start_x = (data_min.x / x_step).floor() as i32;
    let end_x = (data_max.x / x_step).ceil() as i32;

    for i in start_x..=end_x {
        let x_data = i as f32 * x_step;
        let x_world = data_to_world(Vec2::new(x_data, 0.0), rect, view).x;

        // Skip if outside visible area
        if x_world < rect.world_center.x - half_size.x + 10.0
            || x_world > rect.world_center.x + half_size.x - 10.0
        {
            continue;
        }

        let layers_tick = layers.clone();

        commands.entity(root).with_children(|parent| {
            // Tick mark
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(tick_mat.clone()),
                Transform {
                    translation: Vec3::new(x_world, clamped_origin_y, 0.6),
                    scale: Vec3::new(tick_width, tick_length, 1.0),
                    ..default()
                },
                layers_tick.clone(),
            ));

            // Tick label
            parent.spawn((
                Text2d::new(format_tick(x_data)),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(x_world, clamped_origin_y - 14.0, 2.0)),
                layers_tick,
            ));
        });
    }

    // Draw Y-axis ticks and labels
    let start_y = (data_min.y / y_step).floor() as i32;
    let end_y = (data_max.y / y_step).ceil() as i32;

    for i in start_y..=end_y {
        let y_data = i as f32 * y_step;
        let y_world = data_to_world(Vec2::new(0.0, y_data), rect, view).y;

        // Skip if outside visible area
        if y_world < rect.world_center.y - half_size.y + 10.0
            || y_world > rect.world_center.y + half_size.y - 10.0
        {
            continue;
        }

        let layers_tick = layers.clone();

        commands.entity(root).with_children(|parent| {
            // Tick mark
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(tick_mat.clone()),
                Transform {
                    translation: Vec3::new(clamped_origin_x, y_world, 0.6),
                    scale: Vec3::new(tick_length, tick_width, 1.0),
                    ..default()
                },
                layers_tick.clone(),
            ));

            // Tick label
            parent.spawn((
                Text2d::new(format_tick(y_data)),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(clamped_origin_x - 20.0, y_world, 2.0)),
                layers_tick,
            ));
        });
    }
}

/// Draw a histogram
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

    // Compute histogram bins
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

    // Draw border
    let border_mat = materials.add(ColorMaterial::from(Color::srgb(0.3, 0.3, 0.4)));
    let border_thickness = 2.0;
    let layers_border = layers.clone();

    commands.entity(root).with_children(|parent| {
        for (dx, dy) in [(0.0, 0.5), (0.0, -0.5), (-0.5, 0.0), (0.5, 0.0)] {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(border_mat.clone()),
                Transform {
                    translation: Vec3::new(
                        rect.world_center.x + dx * rect.world_size.x,
                        rect.world_center.y + dy * rect.world_size.y,
                        1.0,
                    ),
                    scale: if dx == 0.0 {
                        Vec3::new(rect.world_size.x, border_thickness, 1.0)
                    } else {
                        Vec3::new(border_thickness, rect.world_size.y, 1.0)
                    },
                    ..default()
                },
                layers_border.clone(),
            ));
        }
    });

    // Increased padding for labels
    let padding_left = 0.15;
    let padding_right = 0.08;
    let padding_bottom = 0.18;
    let padding_top = 0.08;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);

    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y = rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

    let bar_width_world = usable_width / bins as f32;
    let gap = bar_width_world * 0.1;

    // Draw bars
    let bar_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity);
    let bar_mat = materials.add(ColorMaterial::from(bar_color));

    for (i, &count) in counts.iter().enumerate() {
        if count == 0 {
            continue;
        }

        let bar_height = (count as f32 / max_count) * usable_height;
        let bar_x = left_x + (i as f32 + 0.5) * bar_width_world;
        let bar_y = bottom_y + bar_height * 0.5;

        let layers_bar = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(bar_mat.clone()),
                Transform {
                    translation: Vec3::new(bar_x, bar_y, 0.0),
                    scale: Vec3::new(bar_width_world - gap, bar_height, 1.0),
                    ..default()
                },
                layers_bar,
            ));
        });
    }

    // Draw x-axis tick labels
    let label_count = bins.min(8);
    let step = bins / label_count;
    for i in (0..=bins).step_by(step.max(1)) {
        let val = min_val + i as f32 * bin_width;
        let x = left_x + (i as f32 / bins as f32) * usable_width;
        let layers_label = layers.clone();

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.1}", val)),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(x, bottom_y - 14.0, 2.0)),
                layers_label,
            ));
        });
    }

    // Draw axis labels
    let x_label_text = x_label.unwrap_or("Value");
    let y_label_text = y_label.unwrap_or("Frequency");

    let layers_axis = layers.clone();
    commands.entity(root).with_children(|parent| {
        // X-axis label
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
            layers_axis.clone(),
        ));

        // Y-axis label
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
            layers_axis,
        ));
    });
}

/// Draw a probability density function (smooth curve)
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

    // Compute KDE (Kernel Density Estimation)
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
    let bandwidth = 1.06 * std_dev * n.powf(-0.2);
    let bandwidth = bandwidth.max(0.01); // Minimum bandwidth

    // Sample the KDE at many points
    let n_samples = 200;
    let range = max_val - min_val;
    let x_min = min_val - range * 0.1;
    let x_max = max_val + range * 0.1;

    let mut kde_points: Vec<(f32, f32)> = Vec::with_capacity(n_samples);
    let mut max_density = 0.0f32;

    for i in 0..n_samples {
        let x = x_min + (i as f32 / (n_samples - 1) as f32) * (x_max - x_min);

        // Gaussian kernel
        let density: f32 = values.iter().map(|&xi| {
            let u = (x - xi) / bandwidth;
            (-0.5 * u * u).exp() / (2.506628 * bandwidth) // 2.506628 ≈ sqrt(2π)
        }).sum::<f32>() / n;

        kde_points.push((x, density));
        max_density = max_density.max(density);
    }

    if max_density <= 0.0 {
        return;
    }

    // Draw border
    let border_mat = materials.add(ColorMaterial::from(Color::srgb(0.3, 0.3, 0.4)));
    let border_thickness = 2.0;
    let layers_border = layers.clone();

    commands.entity(root).with_children(|parent| {
        for (dx, dy) in [(0.0, 0.5), (0.0, -0.5), (-0.5, 0.0), (0.5, 0.0)] {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(border_mat.clone()),
                Transform {
                    translation: Vec3::new(
                        rect.world_center.x + dx * rect.world_size.x,
                        rect.world_center.y + dy * rect.world_size.y,
                        1.0,
                    ),
                    scale: if dx == 0.0 {
                        Vec3::new(rect.world_size.x, border_thickness, 1.0)
                    } else {
                        Vec3::new(border_thickness, rect.world_size.y, 1.0)
                    },
                    ..default()
                },
                layers_border.clone(),
            ));
        }
    });

    // Convert KDE to world coordinates with better padding
    let padding_left = 0.15;
    let padding_right = 0.08;
    let padding_bottom = 0.18;
    let padding_top = 0.08;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y = rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

    // Build the upper curve (the PDF line)
    let upper: Vec<Vec2> = kde_points.iter().map(|&(x, y)| {
        let wx = left_x + ((x - x_min) / (x_max - x_min)) * usable_width;
        let wy = bottom_y + (y / max_density) * usable_height;
        Vec2::new(wx, wy)
    }).collect();

    // Build the lower curve (baseline at y=0)
    let lower: Vec<Vec2> = kde_points.iter().map(|&(x, _)| {
        let wx = left_x + ((x - x_min) / (x_max - x_min)) * usable_width;
        Vec2::new(wx, bottom_y)
    }).collect();

    // Draw filled area under curve
    let n = upper.len();
    if n >= 2 {
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n * 2);
        let mut indices: Vec<u32> = Vec::with_capacity((n - 1) * 6);

        for pt in &upper {
            positions.push([pt.x, pt.y, 0.0]);
        }
        for pt in &lower {
            positions.push([pt.x, pt.y, 0.0]);
        }

        for i in 0..(n - 1) {
            let u0 = i as u32;
            let u1 = (i + 1) as u32;
            let l0 = (n + i) as u32;
            let l1 = (n + i + 1) as u32;
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
        let fill_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity * 0.4);
        let fill_mat = materials.add(ColorMaterial::from(fill_color));

        let layers_fill = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(fill_mesh),
                MeshMaterial2d(fill_mat),
                Transform::default(),
                layers_fill,
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

        let layers_line = layers.clone();
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
                layers_line,
            ));
        });
    }

    // Draw x-axis tick labels
    let n_ticks = 5;
    for i in 0..=n_ticks {
        let t = i as f32 / n_ticks as f32;
        let val = x_min + t * (x_max - x_min);
        let x = left_x + t * usable_width;
        let layers_tick = layers.clone();

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.1}", val)),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(x, bottom_y - 12.0, 2.0)),
                layers_tick,
            ));
        });
    }

    // Draw axis labels
    let layers_axis = layers.clone();
    // Draw axis labels
    let x_label_text = x_label.unwrap_or("Value");
    let y_label_text = y_label.unwrap_or("Density");

    commands.entity(root).with_children(|parent| {
        // X-axis label
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
            layers_axis.clone(),
        ));

        // Y-axis label
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
            layers_axis,
        ));
    });
}
