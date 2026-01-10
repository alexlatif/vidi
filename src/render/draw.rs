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
