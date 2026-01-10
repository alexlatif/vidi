use crate::render::{AxisInfo3D, AxisInfo3DStore, ScatterPoints3D, TileRect, TileView, UnitMeshes};
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_camera::visibility::RenderLayers;
use bevy_mesh::{Indices, PrimitiveTopology};

/// Draw title and description for a plot
/// Returns the height used by title area (for adjusting plot content)
pub fn draw_plot_title(
    commands: &mut Commands,
    root: Entity,
    meta: &crate::core::PlotMeta,
    rect: &TileRect,
    layers: RenderLayers,
) -> f32 {
    let mut title_height = 0.0;

    if meta.title.is_none() && meta.description.is_none() {
        return title_height;
    }

    let title_y = rect.world_center.y + rect.world_size.y * 0.5 - 18.0;

    commands.entity(root).with_children(|parent| {
        if let Some(title) = &meta.title {
            parent.spawn((
                Text2d::new(title.clone()),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                Transform::from_translation(Vec3::new(
                    rect.world_center.x,
                    title_y,
                    3.0,
                )),
                layers.clone(),
            ));
            title_height += 22.0;
        }

        if let Some(desc) = &meta.description {
            let desc_y = title_y - if meta.title.is_some() { 16.0 } else { 0.0 };
            parent.spawn((
                Text2d::new(desc.clone()),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.85)),
                Transform::from_translation(Vec3::new(
                    rect.world_center.x,
                    desc_y,
                    3.0,
                )),
                layers,
            ));
            title_height += 14.0;
        }
    });

    title_height
}

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
                // Support for bubble charts with variable sizes
                let base_size = layer.style.size;
                for (i, &pt) in layer.xy.iter().enumerate() {
                    let world_pos = data_to_world(pt, rect, view);
                    let layers_pt = layers.clone();

                    // Use per-point size if available, otherwise use style.size
                    let point_size = layer.sizes.as_ref()
                        .and_then(|sizes| sizes.get(i).copied())
                        .unwrap_or(base_size);

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

/// Draw a candlestick chart (OHLC) with zoom/pan support
pub fn draw_candlestick(
    commands: &mut Commands,
    root: Entity,
    candle: &crate::core::Candlestick,
    rect: &TileRect,
    view: &TileView,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    if candle.candles.is_empty() {
        return;
    }

    // Draw border (static, not affected by zoom)
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

    // Transform data to world coordinates using view
    let data_to_world_candle = |data: Vec2| -> Vec2 {
        rect.world_center + view.offset + data * view.scale
    };

    // Calculate candle width in data units, then scale to world
    let n_candles = candle.candles.len();
    let x_min = candle.candles.iter().map(|c| c.x).fold(f32::INFINITY, f32::min);
    let x_max = candle.candles.iter().map(|c| c.x).fold(f32::NEG_INFINITY, f32::max);
    let x_range = (x_max - x_min).max(1.0);

    // Candle width in data units
    let candle_data_width = x_range / (n_candles as f32 * 1.5);
    let candle_world_width = candle_data_width * view.scale;
    let wick_world_width = candle_world_width * 0.15;

    // Prepare materials
    let up_color = Color::srgba(
        candle.up_color.r,
        candle.up_color.g,
        candle.up_color.b,
        candle.up_color.a,
    );
    let down_color = Color::srgba(
        candle.down_color.r,
        candle.down_color.g,
        candle.down_color.b,
        candle.down_color.a,
    );
    let up_mat = materials.add(ColorMaterial::from(up_color));
    let down_mat = materials.add(ColorMaterial::from(down_color));

    // Compute visible data range for culling (same pattern as axis ticks)
    let half_size = rect.world_size * 0.5;
    let visible_min = world_to_data(rect.world_center - half_size, rect, view);
    let visible_max = world_to_data(rect.world_center + half_size, rect, view);
    let candle_half_width = candle_data_width * 0.5;

    // Draw candles (only those in visible range)
    for c in &candle.candles {
        // Skip candles outside visible X range (with padding for partial visibility)
        if c.x + candle_half_width < visible_min.x || c.x - candle_half_width > visible_max.x {
            continue;
        }
        // Skip candles outside visible Y range
        if c.high < visible_min.y || c.low > visible_max.y {
            continue;
        }

        let is_up = c.close >= c.open;
        let mat = if is_up { up_mat.clone() } else { down_mat.clone() };

        // Transform to world coordinates
        let body_top_data = c.open.max(c.close);
        let body_bottom_data = c.open.min(c.close);

        let pos_top = data_to_world_candle(Vec2::new(c.x, body_top_data));
        let pos_bottom = data_to_world_candle(Vec2::new(c.x, body_bottom_data));
        let pos_high = data_to_world_candle(Vec2::new(c.x, c.high));
        let pos_low = data_to_world_candle(Vec2::new(c.x, c.low));

        let cx = pos_top.x;
        let body_height = (pos_top.y - pos_bottom.y).max(1.0);
        let body_center_y = (pos_top.y + pos_bottom.y) * 0.5;

        let layers_candle = layers.clone();

        commands.entity(root).with_children(|parent| {
            // Body
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(mat.clone()),
                Transform {
                    translation: Vec3::new(cx, body_center_y, 0.1),
                    scale: Vec3::new(candle_world_width, body_height, 1.0),
                    ..default()
                },
                layers_candle.clone(),
            ));

            // Upper wick
            let upper_wick_height = pos_high.y - pos_top.y;
            if upper_wick_height > 0.5 {
                parent.spawn((
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(mat.clone()),
                    Transform {
                        translation: Vec3::new(cx, (pos_high.y + pos_top.y) * 0.5, 0.0),
                        scale: Vec3::new(wick_world_width, upper_wick_height, 1.0),
                        ..default()
                    },
                    layers_candle.clone(),
                ));
            }

            // Lower wick
            let lower_wick_height = pos_bottom.y - pos_low.y;
            if lower_wick_height > 0.5 {
                parent.spawn((
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(mat),
                    Transform {
                        translation: Vec3::new(cx, (pos_bottom.y + pos_low.y) * 0.5, 0.0),
                        scale: Vec3::new(wick_world_width, lower_wick_height, 1.0),
                        ..default()
                    },
                    layers_candle,
                ));
            }
        });
    }

    // Draw dynamic axis ticks based on visible range
    let half_size = rect.world_size * 0.5;
    let visible_min = world_to_data(rect.world_center - half_size, rect, view);
    let visible_max = world_to_data(rect.world_center + half_size, rect, view);

    // Y-axis ticks
    let y_range_visible = visible_max.y - visible_min.y;
    let y_step = nice_step(y_range_visible, 6);
    let start_y = (visible_min.y / y_step).floor() as i32;
    let end_y = (visible_max.y / y_step).ceil() as i32;

    for i in start_y..=end_y {
        let y_data = i as f32 * y_step;
        let y_world = data_to_world_candle(Vec2::new(0.0, y_data)).y;

        // Skip if outside visible area
        if y_world < rect.world_center.y - half_size.y + 20.0
            || y_world > rect.world_center.y + half_size.y - 10.0
        {
            continue;
        }

        let layers_tick = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format_tick(y_data)),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(
                    rect.world_center.x - half_size.x + 25.0,
                    y_world,
                    2.0,
                )),
                layers_tick,
            ));
        });
    }

    // X-axis ticks
    let x_range_visible = visible_max.x - visible_min.x;
    let x_step = nice_step(x_range_visible, 8);
    let start_x = (visible_min.x / x_step).floor() as i32;
    let end_x = (visible_max.x / x_step).ceil() as i32;

    for i in start_x..=end_x {
        let x_data = i as f32 * x_step;
        let x_world = data_to_world_candle(Vec2::new(x_data, 0.0)).x;

        // Skip if outside visible area
        if x_world < rect.world_center.x - half_size.x + 50.0
            || x_world > rect.world_center.x + half_size.x - 20.0
        {
            continue;
        }

        let layers_tick = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.0}", x_data)),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(
                    x_world,
                    rect.world_center.y - half_size.y + 12.0,
                    2.0,
                )),
                layers_tick,
            ));
        });
    }

    // Draw axis labels (with proper padding from borders)
    let layers_axis = layers.clone();
    let x_label_text = candle.x_label.as_deref().unwrap_or("Time");
    let y_label_text = candle.y_label.as_deref().unwrap_or("Price");

    commands.entity(root).with_children(|parent| {
        // X-axis label at bottom with padding
        parent.spawn((
            Text2d::new(x_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform::from_translation(Vec3::new(
                rect.world_center.x,
                rect.world_center.y - half_size.y + 25.0,
                2.0,
            )),
            layers_axis.clone(),
        ));

        // Y-axis label on left with padding
        parent.spawn((
            Text2d::new(y_label_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            Transform {
                translation: Vec3::new(
                    rect.world_center.x - half_size.x + 15.0,
                    rect.world_center.y,
                    2.0,
                ),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            layers_axis,
        ));
    });
}

/// Draw a heatmap
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
    let vmin = heatmap.vmin.unwrap_or_else(|| {
        heatmap.values.iter().cloned().fold(f32::INFINITY, f32::min)
    });
    let vmax = heatmap.vmax.unwrap_or_else(|| {
        heatmap.values.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    });
    let vrange = (vmax - vmin).max(0.001);

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

    // Padding for labels - more balanced for centered look
    let has_row_labels = heatmap.row_labels.is_some();
    let has_col_labels = heatmap.col_labels.is_some();

    let padding_left = if has_row_labels { 0.12 } else { 0.06 };
    let padding_right = 0.06;
    let padding_bottom = if has_col_labels { 0.12 } else { 0.06 };
    let padding_top = 0.06;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y = rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

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
            // Flip y so row 0 is at top
            let cell_y = bottom_y + usable_height - (row as f32 + 0.5) * cell_height;

            let cell_color = Color::srgba(color.r, color.g, color.b, color.a);
            let cell_mat = materials.add(ColorMaterial::from(cell_color));
            let layers_cell = layers.clone();

            commands.entity(root).with_children(|parent| {
                parent.spawn((
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(cell_mat),
                    Transform {
                        translation: Vec3::new(cell_x, cell_y, 0.0),
                        scale: Vec3::new(cell_width - 1.0, cell_height - 1.0, 1.0),
                        ..default()
                    },
                    layers_cell.clone(),
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
                        layers_cell,
                    ));
                }
            });
        }
    }

    // Draw row labels (on the left, with proper spacing from grid and border)
    if let Some(ref row_labels) = heatmap.row_labels {
        let label_x = left_x - 12.0; // Space from grid edge
        for (row, label) in row_labels.iter().enumerate().take(rows) {
            let y = bottom_y + usable_height - (row as f32 + 0.5) * cell_height;
            let layers_label = layers.clone();

            commands.entity(root).with_children(|parent| {
                parent.spawn((
                    Text2d::new(label.clone()),
                    TextFont {
                        font_size: 9.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                    Transform::from_translation(Vec3::new(label_x, y, 2.0)),
                    layers_label,
                ));
            });
        }
    }

    // Draw column labels (at the bottom, with proper spacing from grid and border)
    if let Some(ref col_labels) = heatmap.col_labels {
        let label_y = bottom_y - 15.0; // Space from grid edge
        for (col, label) in col_labels.iter().enumerate().take(cols) {
            let x = left_x + (col as f32 + 0.5) * cell_width;
            let layers_label = layers.clone();

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
                        rotation: Quat::from_rotation_z(-0.4), // Slight angle for readability
                        ..default()
                    },
                    layers_label,
                ));
            });
        }
    }
}

/// Draw a box plot
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

    // Compute statistics for each group
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

        // Whiskers extend to the most extreme values within the fences
        let min = sorted.iter().cloned().find(|&x| x >= lower_fence).unwrap_or(q1);
        let max = sorted.iter().rev().cloned().find(|&x| x <= upper_fence).unwrap_or(q3);

        // Outliers
        let outliers: Vec<f32> = sorted.iter()
            .cloned()
            .filter(|&x| x < lower_fence || x > upper_fence)
            .collect();

        Some(BoxStats { min, q1, median, q3, max, outliers })
    }

    let stats: Vec<Option<BoxStats>> = groups.iter().map(|(_, v)| compute_box_stats(v)).collect();

    // Find global y range
    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;
    for s in &stats {
        if let Some(s) = s {
            y_min = y_min.min(s.min);
            y_max = y_max.max(s.max);
            for &o in &s.outliers {
                y_min = y_min.min(o);
                y_max = y_max.max(o);
            }
        }
    }

    if !y_min.is_finite() || !y_max.is_finite() {
        return;
    }

    // Add padding
    let y_range = y_max - y_min;
    let y_min = y_min - y_range * 0.1;
    let y_max = y_max + y_range * 0.1;

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

    // Padding
    let padding_left = 0.12;
    let padding_right = 0.05;
    let padding_bottom = 0.18;
    let padding_top = 0.08;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y = rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

    let to_world_y = |y: f32| -> f32 {
        bottom_y + ((y - y_min) / (y_max - y_min)) * usable_height
    };

    let n_groups = groups.len();
    let group_width = usable_width / n_groups as f32;
    let box_width = group_width * 0.6;

    let box_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity);
    let box_mat = materials.add(ColorMaterial::from(box_color));
    let median_color = Color::srgba(1.0, 0.8, 0.2, 1.0);
    let median_mat = materials.add(ColorMaterial::from(median_color));
    let whisker_color = Color::srgba(0.7, 0.7, 0.7, 0.9);
    let whisker_mat = materials.add(ColorMaterial::from(whisker_color));
    let outlier_color = Color::srgba(0.9, 0.3, 0.3, 0.8);
    let outlier_mat = materials.add(ColorMaterial::from(outlier_color));

    // Draw boxes
    for (i, (label, _)) in groups.iter().enumerate() {
        let cx = left_x + (i as f32 + 0.5) * group_width;

        if let Some(ref s) = stats[i] {
            let y_q1 = to_world_y(s.q1);
            let y_q3 = to_world_y(s.q3);
            let y_med = to_world_y(s.median);
            let y_min_w = to_world_y(s.min);
            let y_max_w = to_world_y(s.max);

            let layers_box = layers.clone();

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
                    layers_box.clone(),
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
                    layers_box.clone(),
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
                        layers_box.clone(),
                    ));
                    // Whisker cap
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(whisker_mat.clone()),
                        Transform {
                            translation: Vec3::new(cx, y_min_w, 0.0),
                            scale: Vec3::new(box_width * 0.5, 2.0, 1.0),
                            ..default()
                        },
                        layers_box.clone(),
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
                        layers_box.clone(),
                    ));
                    // Whisker cap
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(whisker_mat.clone()),
                        Transform {
                            translation: Vec3::new(cx, y_max_w, 0.0),
                            scale: Vec3::new(box_width * 0.5, 2.0, 1.0),
                            ..default()
                        },
                        layers_box.clone(),
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
                        layers_box.clone(),
                    ));
                }
            });
        }

        // Group label
        let layers_label = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(label.clone()),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                Transform::from_translation(Vec3::new(cx, bottom_y - 12.0, 2.0)),
                layers_label,
            ));
        });
    }

    // Y-axis tick labels
    let n_ticks = 5;
    for i in 0..=n_ticks {
        let t = i as f32 / n_ticks as f32;
        let val = y_min + t * (y_max - y_min);
        let y = to_world_y(val);
        let layers_tick = layers.clone();

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.1}", val)),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(left_x - 20.0, y, 2.0)),
                layers_tick,
            ));
        });
    }

    // Axis labels
    let layers_axis = layers.clone();
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
            layers_axis.clone(),
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
            layers_axis,
        ));
    });
}

/// Draw an ECDF (Empirical Cumulative Distribution Function) - step function
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

    // Sort values for ECDF computation
    let mut sorted: Vec<f32> = values.iter().cloned().filter(|x| x.is_finite()).collect();
    if sorted.is_empty() {
        return;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let n = sorted.len();
    let min_val = sorted[0];
    let max_val = sorted[n - 1];

    if min_val >= max_val {
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

    // Padding for labels
    let padding_left = 0.15;
    let padding_right = 0.08;
    let padding_bottom = 0.18;
    let padding_top = 0.08;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y = rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;

    // Add some padding to x range
    let range = max_val - min_val;
    let x_min = min_val - range * 0.05;
    let x_max = max_val + range * 0.05;

    // Convert data to world coordinates
    let to_world = |x: f32, y: f32| -> Vec2 {
        let wx = left_x + ((x - x_min) / (x_max - x_min)) * usable_width;
        let wy = bottom_y + y * usable_height;
        Vec2::new(wx, wy)
    };

    // Build ECDF step points
    // ECDF(x) = (number of values <= x) / n
    let mut step_points: Vec<Vec2> = Vec::new();

    // Start at (x_min, 0)
    step_points.push(to_world(x_min, 0.0));

    // Add step for each unique value
    let mut i = 0;
    while i < n {
        let x = sorted[i];
        // Count how many values equal to this one
        let mut count = 1;
        while i + count < n && sorted[i + count] == x {
            count += 1;
        }

        // Horizontal line to this x value at previous y
        let prev_y = i as f32 / n as f32;
        step_points.push(to_world(x, prev_y));

        // Vertical jump to new y
        let new_y = (i + count) as f32 / n as f32;
        step_points.push(to_world(x, new_y));

        i += count;
    }

    // End at (x_max, 1)
    step_points.push(to_world(x_max, 1.0));

    // Build fill mesh under the step function
    if step_points.len() >= 2 {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        // For each segment, create a filled rectangle
        for window in step_points.windows(2) {
            let a = window[0];
            let b = window[1];

            // Only fill horizontal segments (where y is the same)
            if (a.y - b.y).abs() < 0.001 && (b.x - a.x).abs() > 0.001 {
                let base_idx = positions.len() as u32;

                // Rectangle: bottom-left, bottom-right, top-right, top-left
                positions.push([a.x, bottom_y, 0.0]);
                positions.push([b.x, bottom_y, 0.0]);
                positions.push([b.x, a.y, 0.0]);
                positions.push([a.x, a.y, 0.0]);

                // Two triangles
                indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);
                indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 3]);
            }
        }

        if !positions.is_empty() {
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
            let fill_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity * 0.3);
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
    }

    // Draw the step line
    let line_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity);
    let line_mat = materials.add(ColorMaterial::from(line_color));

    for window in step_points.windows(2) {
        let a = window[0];
        let b = window[1];
        let length = a.distance(b);
        if length < 0.1 {
            continue;
        }

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

    // Draw y-axis tick labels (0.0, 0.25, 0.5, 0.75, 1.0)
    for i in 0..=4 {
        let y_val = i as f32 * 0.25;
        let y = bottom_y + y_val * usable_height;
        let layers_tick = layers.clone();

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(format!("{:.2}", y_val)),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                Transform::from_translation(Vec3::new(left_x - 20.0, y, 2.0)),
                layers_tick,
            ));
        });
    }

    // Draw axis labels
    let x_label_text = x_label.unwrap_or("Value");
    let y_label_text = y_label.unwrap_or("F(x)");

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
                    rect.world_center.x - rect.world_size.x * 0.5 + 10.0,
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

/// Draw radial charts (Pie, Radar)
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
            draw_pie(commands, root, slices, rect, unit, meshes, materials, layers);
        }
        crate::core::Radial::Radar { axes, values, style, .. } => {
            draw_radar(commands, root, axes, values, style, rect, unit, meshes, materials, layers);
        }
    }
}

/// Draw a pie chart
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

    // Calculate total
    let total: f32 = slices.iter().map(|(_, v)| v.max(0.0)).sum();
    if total <= 0.0 {
        return;
    }

    // Pie chart center and radius
    let center = rect.world_center;
    let radius = (rect.world_size.x.min(rect.world_size.y) * 0.35).max(10.0);

    // Color palette
    let colors = [
        Color::srgba(0.3, 0.6, 0.9, 0.9),  // Blue
        Color::srgba(0.9, 0.4, 0.3, 0.9),  // Red
        Color::srgba(0.4, 0.8, 0.4, 0.9),  // Green
        Color::srgba(0.9, 0.7, 0.2, 0.9),  // Yellow
        Color::srgba(0.7, 0.4, 0.9, 0.9),  // Purple
        Color::srgba(0.3, 0.8, 0.8, 0.9),  // Cyan
        Color::srgba(0.9, 0.5, 0.7, 0.9),  // Pink
        Color::srgba(0.6, 0.6, 0.6, 0.9),  // Gray
    ];

    let mut start_angle = -std::f32::consts::FRAC_PI_2; // Start at top
    let segments_per_slice = 32;

    for (i, (label, value)) in slices.iter().enumerate() {
        if *value <= 0.0 {
            continue;
        }

        let sweep = (*value / total) * std::f32::consts::TAU;
        let color = colors[i % colors.len()];

        // Build pie slice mesh
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        // Center vertex
        positions.push([center.x, center.y, 0.0]);

        // Arc vertices
        for j in 0..=segments_per_slice {
            let angle = start_angle + (j as f32 / segments_per_slice as f32) * sweep;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            positions.push([x, y, 0.0]);
        }

        // Create triangles (fan from center)
        for j in 0..segments_per_slice {
            indices.push(0); // center
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

        let layers_slice = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(slice_mesh),
                MeshMaterial2d(slice_mat),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                layers_slice,
            ));
        });

        // Draw label at slice center
        let mid_angle = start_angle + sweep * 0.5;
        let label_radius = radius * 1.25;
        let label_x = center.x + label_radius * mid_angle.cos();
        let label_y = center.y + label_radius * mid_angle.sin();

        let pct = (*value / total) * 100.0;
        let label_text = format!("{}\n{:.1}%", label, pct);

        let layers_label = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(label_text),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.9, 0.9, 1.0)),
                Transform::from_translation(Vec3::new(label_x, label_y, 2.0)),
                layers_label,
            ));
        });

        start_angle += sweep;
    }
}

/// Draw a radar (spider) chart
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
        return; // Need at least 3 axes for a radar chart
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

    let center = rect.world_center;
    let radius = (rect.world_size.x.min(rect.world_size.y) * 0.35).max(10.0);

    // Angle between each axis
    let angle_step = std::f32::consts::TAU / n as f32;

    // Draw grid circles (at 25%, 50%, 75%, 100%)
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

            let layers_grid = layers.clone();
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
                    layers_grid,
                ));
            });
        }
    }

    // Draw axis lines and labels
    let axis_mat = materials.add(ColorMaterial::from(Color::srgba(0.5, 0.5, 0.6, 0.7)));

    for i in 0..n {
        let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * angle_step;
        let _end_x = center.x + radius * angle.cos();
        let _end_y = center.y + radius * angle.sin();

        // Axis line
        let length = radius;
        let mid = Vec2::new(
            center.x + radius * 0.5 * angle.cos(),
            center.y + radius * 0.5 * angle.sin(),
        );

        let layers_axis = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(axis_mat.clone()),
                Transform {
                    translation: mid.extend(0.1),
                    rotation: Quat::from_rotation_z(angle),
                    scale: Vec3::new(length, 1.5, 1.0),
                    ..default()
                },
                layers_axis,
            ));
        });

        // Axis label
        let label_x = center.x + (radius + 20.0) * angle.cos();
        let label_y = center.y + (radius + 20.0) * angle.sin();

        let layers_label = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Text2d::new(axes[i].clone()),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                Transform::from_translation(Vec3::new(label_x, label_y, 2.0)),
                layers_label,
            ));
        });
    }

    // Build data polygon mesh
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Center vertex
    positions.push([center.x, center.y, 0.0]);

    // Data vertices
    for i in 0..n {
        let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * angle_step;
        let r = radius * values[i].clamp(0.0, 1.0);
        let x = center.x + r * angle.cos();
        let y = center.y + r * angle.sin();
        positions.push([x, y, 0.0]);
    }

    // Create triangles (fan from center)
    for i in 0..n {
        let next = (i + 1) % n;
        indices.push(0); // center
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
    let fill_color = Color::srgba(style.color.r, style.color.g, style.color.b, style.opacity * 0.4);
    let fill_mat = materials.add(ColorMaterial::from(fill_color));

    let layers_fill = layers.clone();
    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Mesh2d(fill_mesh),
            MeshMaterial2d(fill_mat),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.2)),
            layers_fill,
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

        let layers_line = layers.clone();
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
                layers_line,
            ));
        });
    }

    // Draw data points
    let point_mat = materials.add(ColorMaterial::from(line_color));
    for i in 1..=n {
        let layers_point = layers.clone();
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(point_mat.clone()),
                Transform {
                    translation: Vec3::new(positions[i][0], positions[i][1], 0.4),
                    scale: Vec3::splat(6.0),
                    ..default()
                },
                layers_point,
            ));
        });
    }
}

// ============================================================================
// 3D PLOTTING FUNCTIONS
// ============================================================================

/// Draw a 3D plot (scatter points or surface)
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
    axis_info_store.info.insert(tile_index, AxisInfo3D {
        x_label: graph.x_label.clone(),
        y_label: graph.y_label.clone(),
        z_label: graph.z_label.clone(),
        bounds_min: bounds.0,
        bounds_max: bounds.1,
    });

    // Clear previous scatter points for this tile
    scatter_points.points.remove(&tile_index);

    // Multiple lights for better visibility
    // Key light (main, from top-front-right) - brighter for better surface visibility
    let light1 = commands.spawn((
        PointLight {
            intensity: 800000.0,
            range: 100.0,
            color: Color::srgb(1.0, 0.98, 0.95),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(8.0, 12.0, 8.0),
        layers.clone(),
    )).id();
    commands.entity(root).add_child(light1);

    // Fill light (softer, from opposite side)
    let light2 = commands.spawn((
        PointLight {
            intensity: 350000.0,
            range: 100.0,
            color: Color::srgb(0.9, 0.95, 1.0),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-6.0, 8.0, -6.0),
        layers.clone(),
    )).id();
    commands.entity(root).add_child(light2);

    // Rim light (from behind for edge definition)
    let light3 = commands.spawn((
        PointLight {
            intensity: 200000.0,
            range: 100.0,
            color: Color::srgb(1.0, 1.0, 1.0),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, -5.0, -10.0),
        layers.clone(),
    )).id();
    commands.entity(root).add_child(light3);

    // Collect scatter points for tooltip
    let mut all_points = Vec::new();

    // Draw each layer
    for layer_data in &graph.layers {
        if let Some(points) = draw_layer_3d(commands, root, meshes, materials, layers.clone(), layer_data, &bounds) {
            all_points.extend(points);
        }
    }

    // Store scatter points for tooltip lookup
    if !all_points.is_empty() {
        scatter_points.points.insert(tile_index, all_points);
    }

    // Draw 3D axes with labels (scaled to data)
    draw_3d_axes(commands, root, meshes, materials, layers, &bounds);

    // Draw title and border on overlay layer (2D elements)
    draw_3d_title_and_border(commands, root, &graph.meta, rect, unit, color_materials, overlay_layers);
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
    // Border
    let border_color = Color::srgba(0.3, 0.3, 0.35, 0.8);
    let border_mat = materials.add(ColorMaterial::from(border_color));
    let border_thickness = 2.0;

    // Top border
    let top = commands.spawn((
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
        layers.clone(),
    )).id();
    commands.entity(root).add_child(top);

    // Bottom border
    let bottom = commands.spawn((
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
        layers.clone(),
    )).id();
    commands.entity(root).add_child(bottom);

    // Left border
    let left = commands.spawn((
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
        layers.clone(),
    )).id();
    commands.entity(root).add_child(left);

    // Right border
    let right = commands.spawn((
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
        layers.clone(),
    )).id();
    commands.entity(root).add_child(right);

    // Title and description
    let title_y = rect.world_center.y + rect.world_size.y * 0.5 - 18.0;

    if let Some(title) = &meta.title {
        let title_entity = commands.spawn((
            Text2d::new(title.clone()),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
            Transform::from_translation(Vec3::new(rect.world_center.x, title_y, 3.0)),
            layers.clone(),
        )).id();
        commands.entity(root).add_child(title_entity);
    }

    if let Some(desc) = &meta.description {
        let desc_y = title_y - if meta.title.is_some() { 16.0 } else { 0.0 };
        let desc_entity = commands.spawn((
            Text2d::new(desc.clone()),
            TextFont {
                font_size: 10.0,
                ..default()
            },
            TextColor(Color::srgba(0.7, 0.7, 0.7, 0.85)),
            Transform::from_translation(Vec3::new(rect.world_center.x, desc_y, 3.0)),
            layers,
        )).id();
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
                let entity = commands.spawn((
                    Mesh3d(sphere_mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(normalized),
                    layers.clone(),
                )).id();
                commands.entity(root).add_child(entity);
            }

            Some(point_pairs)
        }

        crate::core::Geometry3D::Surface { grid } => {
            // Normalize all points for the surface
            let normalized_xyz: Vec<Vec3> = layer_data.xyz
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
            let entity = commands.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat),
                Transform::IDENTITY,
                layers.clone(),
            )).id();
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
    _bounds: &([f32; 3], [f32; 3]),
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
    let x_axis = commands.spawn((
        Mesh3d(mesh_x),
        MeshMaterial3d(mat_x.clone()),
        Transform::from_translation(origin + Vec3::new(axis_len * 0.5, 0.0, 0.0)),
        layers.clone(),
    )).id();
    commands.entity(root).add_child(x_axis);

    // X axis label (cone/arrow at tip)
    let x_tip = meshes.add(Sphere::new(0.12));
    let x_label = commands.spawn((
        Mesh3d(x_tip),
        MeshMaterial3d(mat_x),
        Transform::from_translation(origin + Vec3::new(axis_len + 0.15, 0.0, 0.0)),
        layers.clone(),
    )).id();
    commands.entity(root).add_child(x_label);

    // Y axis (green)
    let mesh_y = meshes.add(Cuboid::new(thickness, axis_len, thickness));
    let y_axis = commands.spawn((
        Mesh3d(mesh_y),
        MeshMaterial3d(mat_y.clone()),
        Transform::from_translation(origin + Vec3::new(0.0, axis_len * 0.5, 0.0)),
        layers.clone(),
    )).id();
    commands.entity(root).add_child(y_axis);

    // Y axis label
    let y_tip = meshes.add(Sphere::new(0.12));
    let y_label = commands.spawn((
        Mesh3d(y_tip),
        MeshMaterial3d(mat_y),
        Transform::from_translation(origin + Vec3::new(0.0, axis_len + 0.15, 0.0)),
        layers.clone(),
    )).id();
    commands.entity(root).add_child(y_label);

    // Z axis (blue)
    let mesh_z = meshes.add(Cuboid::new(thickness, thickness, axis_len));
    let z_axis = commands.spawn((
        Mesh3d(mesh_z),
        MeshMaterial3d(mat_z.clone()),
        Transform::from_translation(origin + Vec3::new(0.0, 0.0, axis_len * 0.5)),
        layers.clone(),
    )).id();
    commands.entity(root).add_child(z_axis);

    // Z axis label
    let z_tip = meshes.add(Sphere::new(0.12));
    let z_label = commands.spawn((
        Mesh3d(z_tip),
        MeshMaterial3d(mat_z),
        Transform::from_translation(origin + Vec3::new(0.0, 0.0, axis_len + 0.15)),
        layers.clone(),
    )).id();
    commands.entity(root).add_child(z_label);

    // Origin marker (white sphere)
    let origin_mesh = meshes.add(Sphere::new(0.08));
    let origin_marker = commands.spawn((
        Mesh3d(origin_mesh),
        MeshMaterial3d(mat_label),
        Transform::from_translation(origin),
        layers.clone(),
    )).id();
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
        let grid_line = commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat_grid.clone()),
            Transform::from_translation(Vec3::new(0.0, -half_size, z)),
            layers.clone(),
        )).id();
        commands.entity(root).add_child(grid_line);
    }
    // Lines parallel to Z
    for ix in -n_lines..=n_lines {
        let x = ix as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(grid_thick, grid_thick, full_len));
        let grid_line = commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat_grid.clone()),
            Transform::from_translation(Vec3::new(x, -half_size, 0.0)),
            layers.clone(),
        )).id();
        commands.entity(root).add_child(grid_line);
    }

    // === XY back wall grid (z = -2.5) ===
    // Lines parallel to X
    for iy in -n_lines..=n_lines {
        let y = iy as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(full_len, grid_thick, grid_thick));
        let grid_line = commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat_grid.clone()),
            Transform::from_translation(Vec3::new(0.0, y, -half_size)),
            layers.clone(),
        )).id();
        commands.entity(root).add_child(grid_line);
    }
    // Lines parallel to Y
    for ix in -n_lines..=n_lines {
        let x = ix as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(grid_thick, full_len, grid_thick));
        let grid_line = commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat_grid.clone()),
            Transform::from_translation(Vec3::new(x, 0.0, -half_size)),
            layers.clone(),
        )).id();
        commands.entity(root).add_child(grid_line);
    }

    // === YZ side wall grid (x = -2.5) ===
    // Lines parallel to Z
    for iy in -n_lines..=n_lines {
        let y = iy as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(grid_thick, grid_thick, full_len));
        let grid_line = commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat_grid.clone()),
            Transform::from_translation(Vec3::new(-half_size, y, 0.0)),
            layers.clone(),
        )).id();
        commands.entity(root).add_child(grid_line);
    }
    // Lines parallel to Y
    for iz in -n_lines..=n_lines {
        let z = iz as f32 * grid_step;
        let mesh = meshes.add(Cuboid::new(grid_thick, full_len, grid_thick));
        let grid_line = commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat_grid.clone()),
            Transform::from_translation(Vec3::new(-half_size, 0.0, z)),
            layers.clone(),
        )).id();
        commands.entity(root).add_child(grid_line);
    }
}

/// Create a mesh for a 3D surface from a grid of points
pub fn create_surface_mesh(vertices: &[Vec3], grid: bevy_math::UVec2) -> Mesh {
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
