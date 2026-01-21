#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]

use super::*;
use crate::render::PlotId;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_camera::visibility::RenderLayers;
use bevy_camera::{
    OrthographicProjection, PerspectiveProjection, Projection, ScalingMode, Viewport,
};
use bevy_math::UVec2;
use std::collections::HashSet;

// #[derive(Resource, Clone, Debug)]
// pub struct DashboardRes(pub crate::core::Dashboard);

/// Core system: Sync dashboard plots to tile entities
pub fn sync_plots_to_tiles(
    mut commands: Commands,
    dash: Res<DashboardRes>,
    mut registry: ResMut<TileRegistry>,
    existing: Query<(Entity, &PlotTile)>,
) {
    let active = dash.0.active_plots();

    // Generate plot IDs from active plots (respects tabs)
    let plot_ids: Vec<PlotId> = active
        .iter()
        .enumerate()
        .map(|(i, _)| PlotId(i as u64))
        .collect();

    // Remove tiles for plots that no longer exist
    for (entity, tile) in existing.iter() {
        if !plot_ids.contains(&tile.id) {
            cleanup_tile(&mut commands, &mut registry, entity, tile.id);
        }
    }

    // Create missing tiles
    for (i, plot) in active.iter().enumerate() {
        let id = PlotId(i as u64);

        if let std::collections::hash_map::Entry::Vacant(e) = registry.by_plot.entry(id) {
            let tile = spawn_tile(&mut commands, id, i, plot);
            e.insert(tile);
            registry.dirty.push_back(id);
        }
    }
}

fn spawn_tile(
    commands: &mut Commands,
    id: PlotId,
    index: usize,
    plot: &crate::core::Plot,
) -> Entity {
    let kind = match plot {
        crate::core::Plot::Graph2D(_) => PlotKind::TwoD,
        crate::core::Plot::Graph3D(_) => PlotKind::ThreeD,
        crate::core::Plot::Distribution(_) => PlotKind::TwoD,
        crate::core::Plot::Candlestick(_) => PlotKind::TwoD,
        crate::core::Plot::Heatmap(_) => PlotKind::TwoD,
        crate::core::Plot::Radial(_) => PlotKind::TwoD,
        crate::core::Plot::Field(_) => PlotKind::TwoD,
    };

    let tile = commands
        .spawn((
            PlotTile { id, index, kind },
            kind, // Add PlotKind as separate component for queries
            TileView::default(),
            TileRect {
                world_center: Vec2::ZERO,
                world_size: Vec2::new(100.0, 100.0),
                content: Rect::from_center_size(Vec2::ZERO, Vec2::new(70.0, 70.0)),
                viewport: Viewport {
                    physical_position: UVec2::ZERO,
                    physical_size: UVec2::new(100, 100),
                    depth: 0.0..1.0,
                },
            },
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    // Add View3D for 3D plots
    if matches!(kind, PlotKind::ThreeD) {
        commands.entity(tile).insert(View3D::default());
    }

    // Create render root child with visibility
    let root = commands
        .spawn((TileRenderRoot, Transform::default(), Visibility::default()))
        .id();
    commands.entity(tile).add_child(root);

    tile
}

/// Update tile layout when window resizes
pub fn update_tile_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut registry: ResMut<TileRegistry>,
    mut tiles: Query<(&PlotTile, &mut TileRect)>,
    dash: Res<DashboardRes>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let plots = dash.0.active_plots();
    let n = plots.len();
    if n == 0 {
        return;
    }

    let (cols, rows) = grid_dims(n, window.width() / window.height(), dash.0.active_columns());

    let margin = 20.0;
    let gap = 10.0;
    // Reserve space for tab bar at top if using tabs
    let tab_bar_height = if dash.0.has_tabs() { 40.0 } else { 0.0 };

    let avail_w = window.width() - 2.0 * margin;
    let avail_h = window.height() - 2.0 * margin - tab_bar_height;

    let tile_w = (avail_w - (cols - 1) as f32 * gap) / cols as f32;
    let tile_h = (avail_h - (rows - 1) as f32 * gap) / rows as f32;

    for (tile, mut rect) in tiles.iter_mut() {
        let col = tile.index % cols;
        let row = tile.index / cols;

        // Viewport in physical pixels (CRITICAL FIX)
        let vp_x = margin + col as f32 * (tile_w + gap);
        let vp_y = margin + tab_bar_height + row as f32 * (tile_h + gap);

        let scale = window.resolution.scale_factor();
        let phys_pos = UVec2::new((vp_x * scale).round() as u32, (vp_y * scale).round() as u32);
        let phys_size = UVec2::new(
            (tile_w * scale).round() as u32,
            (tile_h * scale).round() as u32,
        );

        // World coordinates (centered origin)
        let world_center = Vec2::new(
            vp_x + tile_w * 0.5 - window.width() * 0.5,
            window.height() * 0.5 - vp_y - tile_h * 0.5,
        );

        let new_size = Vec2::new(tile_w, tile_h);

        // Only mark dirty if layout actually changed
        let changed = rect.world_center != world_center
            || rect.world_size != new_size
            || rect.viewport.physical_position != phys_pos
            || rect.viewport.physical_size != phys_size;

        if changed {
            rect.world_center = world_center;
            rect.world_size = new_size;
            rect.content =
                Rect::from_center_size(world_center, Vec2::new(tile_w - 30.0, tile_h - 30.0));
            rect.viewport = Viewport {
                physical_position: phys_pos,
                physical_size: phys_size,
                depth: 0.0..1.0,
            };

            registry.dirty.push_back(tile.id);
        }
    }
}

/// Create/update cameras for each tile
pub fn sync_tile_cameras(
    mut commands: Commands,
    mut registry: ResMut<TileRegistry>,
    tiles: Query<(Entity, &PlotTile, &TileRect, &PlotKind, Option<&View3D>)>,
    existing: Query<Entity, With<TileCamera>>,
    existing_overlay: Query<Entity, With<TileOverlayCamera>>,
) {
    let mut used = HashSet::new();
    let mut used_overlay = HashSet::new();

    for (_tile_entity, tile, rect, kind, view3d) in tiles.iter() {
        // One layer per tile index (0..31). This is a hard RenderLayers limitation.
        let layer = (tile.index % 32) as u8;
        let layers = RenderLayers::layer(layer.into());
        // Use a separate layer for overlay (offset by 16, wrapping within 0..31)
        let overlay_layer = ((tile.index + 16) % 32) as u8;
        let overlay_layers = RenderLayers::layer(overlay_layer.into());

        let cam_entity = if let Some(&cam) = registry.camera_of.get(&tile.id) {
            cam
        } else {
            let cam = commands.spawn((TileCamera, Transform::default())).id();
            registry.camera_of.insert(tile.id, cam);
            cam
        };

        used.insert(cam_entity);

        match kind {
            PlotKind::TwoD => {
                let mut ortho = OrthographicProjection::default_2d();
                ortho.scaling_mode = ScalingMode::FixedVertical {
                    viewport_height: rect.world_size.y,
                };

                commands.entity(cam_entity).insert((
                    Camera2d,
                    Camera {
                        viewport: Some(rect.viewport.clone()),
                        order: 10 + tile.index as isize,
                        ..default()
                    },
                    Projection::from(ortho),
                    Transform::from_translation(rect.world_center.extend(1000.0)),
                    layers,
                ));
            }
            PlotKind::ThreeD => {
                // Calculate camera position from View3D spherical coordinates
                let view = view3d.copied().unwrap_or_default();
                let cam_transform = compute_orbit_transform(&view);

                commands.entity(cam_entity).insert((
                    Camera3d::default(),
                    Camera {
                        viewport: Some(rect.viewport.clone()),
                        order: 10 + tile.index as isize,
                        ..default()
                    },
                    Projection::from(PerspectiveProjection {
                        fov: std::f32::consts::FRAC_PI_4, // 45 degrees
                        ..default()
                    }),
                    cam_transform,
                    layers,
                ));

                // Create/update overlay 2D camera for titles and borders
                let overlay_entity = if let Some(&overlay) = registry.overlay_of.get(&tile.id) {
                    overlay
                } else {
                    let overlay = commands
                        .spawn((TileOverlayCamera, Transform::default()))
                        .id();
                    registry.overlay_of.insert(tile.id, overlay);
                    overlay
                };
                used_overlay.insert(overlay_entity);

                let mut overlay_ortho = OrthographicProjection::default_2d();
                overlay_ortho.scaling_mode = ScalingMode::FixedVertical {
                    viewport_height: rect.world_size.y,
                };

                commands.entity(overlay_entity).insert((
                    Camera2d,
                    Camera {
                        viewport: Some(rect.viewport.clone()),
                        order: 50 + tile.index as isize, // Render after all 3D cameras (10+N)
                        clear_color: ClearColorConfig::None, // Don't clear, overlay on top
                        ..default()
                    },
                    Projection::from(overlay_ortho),
                    Transform::from_translation(rect.world_center.extend(1000.0)),
                    overlay_layers,
                ));
            }
            PlotKind::Placeholder => {
                // Keep camera if you want placeholders rendered; otherwise you can skip.
                let mut ortho = OrthographicProjection::default_2d();
                ortho.scaling_mode = ScalingMode::FixedVertical {
                    viewport_height: rect.world_size.y,
                };

                commands.entity(cam_entity).insert((
                    Camera2d,
                    Camera {
                        viewport: Some(rect.viewport.clone()),
                        order: 10 + tile.index as isize,
                        ..default()
                    },
                    Projection::from(ortho),
                    Transform::from_translation(rect.world_center.extend(1000.0)),
                    layers,
                ));
            }
        }
    }

    // Despawn cameras no longer used
    for cam_entity in existing.iter() {
        if !used.contains(&cam_entity) {
            commands.entity(cam_entity).despawn();
        }
    }

    // Despawn overlay cameras no longer used
    for overlay_entity in existing_overlay.iter() {
        if !used_overlay.contains(&overlay_entity) {
            commands.entity(overlay_entity).despawn();
        }
    }
}

/// Handle hover detection
pub fn update_hovered_tile(
    windows: Query<&Window>,
    tiles: Query<(&PlotTile, &TileRect)>,
    mut hovered: ResMut<HoveredTile>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    hovered.0 = tiles
        .iter()
        .find(|(_, rect)| {
            let half = rect.world_size * 0.5;
            let min = rect.world_center - half;
            let max = rect.world_center + half;

            let world_x = cursor.x - window.width() * 0.5;
            let world_y = window.height() * 0.5 - cursor.y;

            world_x >= min.x && world_x <= max.x && world_y >= min.y && world_y <= max.y
        })
        .map(|(tile, _)| tile.index);
}

/// Compute camera transform from View3D orbit parameters
fn compute_orbit_transform(view: &View3D) -> Transform {
    let cy = view.yaw.cos();
    let sy = view.yaw.sin();
    let cp = view.pitch.cos();
    let sp = view.pitch.sin();

    // Spherical to Cartesian: camera position orbiting target
    let dir = Vec3::new(sy * cp, sp, cy * cp);
    let pos = view.target + dir * view.radius;

    Transform::from_translation(pos).looking_at(view.target, Vec3::Y)
}

/// Handle user input for both 2D and 3D tiles
pub fn handle_input(
    mut tiles_2d: Query<(&PlotTile, &mut TileView), Without<View3D>>,
    mut tiles_3d: Query<(&PlotTile, &mut View3D)>,
    mut registry: ResMut<TileRegistry>,
    hovered: Res<HoveredTile>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut motion: MessageReader<MouseMotion>,
) {
    let Some(hovered_index) = hovered.0 else {
        return;
    };

    // Collect events first (they can only be read once)
    let mut zoom_delta = 0.0;
    for event in wheel.read() {
        zoom_delta += event.y;
    }

    let mut motion_delta = Vec2::ZERO;
    for event in motion.read() {
        motion_delta += event.delta;
    }

    // Handle 2D tiles
    for (tile, mut view) in tiles_2d.iter_mut() {
        if tile.index != hovered_index {
            continue;
        }

        let mut changed = false;

        // Zoom toward center with proper offset adjustment
        if zoom_delta != 0.0 {
            let old_scale = view.scale;
            let new_scale =
                (view.scale * (1.0 + zoom_delta * 0.05)).clamp(view.min_scale, view.max_scale);

            if new_scale != old_scale {
                view.offset = view.offset * new_scale / old_scale;
                view.scale = new_scale;
                changed = true;
            }
        }

        // Pan (offset is in world coordinates)
        if mouse.pressed(MouseButton::Left) && motion_delta != Vec2::ZERO {
            view.offset.x += motion_delta.x;
            view.offset.y -= motion_delta.y;
            changed = true;
        }

        if changed {
            registry.dirty.push_back(tile.id);
        }
    }

    // Handle 3D tiles
    for (tile, mut view3d) in tiles_3d.iter_mut() {
        if tile.index != hovered_index {
            continue;
        }

        let mut changed = false;
        let orbit_speed = 0.008;
        let pan_speed = 0.01;

        // Zoom (scroll wheel adjusts radius)
        if zoom_delta != 0.0 {
            view3d.radius = (view3d.radius - zoom_delta * 0.5).clamp(2.0, 50.0);
            changed = true;
        }

        // Orbit (left mouse button)
        if mouse.pressed(MouseButton::Left) && motion_delta != Vec2::ZERO {
            view3d.yaw -= motion_delta.x * orbit_speed;
            view3d.pitch = (view3d.pitch - motion_delta.y * orbit_speed).clamp(-1.5, 1.5);
            changed = true;
        }

        // Pan (right mouse button)
        if mouse.pressed(MouseButton::Right) && motion_delta != Vec2::ZERO {
            let right = Vec3::new(view3d.yaw.cos(), 0.0, -view3d.yaw.sin());
            let up = Vec3::Y;
            let radius = view3d.radius;
            view3d.target +=
                (-right * motion_delta.x + up * motion_delta.y) * pan_speed * radius * 0.1;
            changed = true;
        }

        if changed {
            // For 3D, we don't need to redraw - just update the camera
            // But we do need to mark dirty if we want the camera to update
            registry.dirty.push_back(tile.id);
        }
    }
}

/// Auto-fit tiles to their data bounds on first render
pub fn auto_fit_tiles(
    mut commands: Commands,
    mut registry: ResMut<TileRegistry>,
    mut tiles: Query<(Entity, &PlotTile, &TileRect, &mut TileView), Without<AutoFitted>>,
    dash: Res<DashboardRes>,
) {
    for (entity, tile, rect, mut view) in tiles.iter_mut() {
        // Get data bounds for this plot
        let Some(plot) = dash.0.active_plots().get(tile.index) else {
            continue;
        };

        // Compute data bounds based on plot type
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        match plot {
            crate::core::Plot::Graph2D(graph) => {
                for layer in &graph.layers {
                    for pt in &layer.xy {
                        min_x = min_x.min(pt.x);
                        max_x = max_x.max(pt.x);
                        min_y = min_y.min(pt.y);
                        max_y = max_y.max(pt.y);
                    }
                    // Also consider lower_line for FillBetween geometry
                    if let Some(lower_line) = &layer.lower_line {
                        for pt in lower_line {
                            min_x = min_x.min(pt.x);
                            max_x = max_x.max(pt.x);
                            min_y = min_y.min(pt.y);
                            max_y = max_y.max(pt.y);
                        }
                    }
                }
            }
            crate::core::Plot::Candlestick(candle) => {
                for c in &candle.candles {
                    min_x = min_x.min(c.x);
                    max_x = max_x.max(c.x);
                    min_y = min_y.min(c.low);
                    max_y = max_y.max(c.high);
                }
            }
            _ => {
                // Mark as fitted even if not a zoomable type
                commands.entity(entity).try_insert(AutoFitted);
                continue;
            }
        }

        // Skip if no valid bounds
        if !min_x.is_finite() || !max_x.is_finite() || !min_y.is_finite() || !max_y.is_finite() {
            commands.entity(entity).try_insert(AutoFitted);
            continue;
        }

        let data_width = (max_x - min_x).max(0.01);
        let data_height = (max_y - min_y).max(0.01);
        let data_center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);

        // Compute scale to fit data in viewport with some padding
        let padding = 0.85; // Use 85% of available space
        let available_size = rect.world_size * padding;
        let scale_x = available_size.x / data_width;
        let scale_y = available_size.y / data_height;
        let fit_scale = scale_x.min(scale_y);

        // Set view to center on data with zoom limits
        view.scale = fit_scale;
        view.offset = -data_center * fit_scale;
        view.min_scale = fit_scale * 0.5; // Can zoom out to 50% of fit
        view.max_scale = fit_scale * 4.0; // Can zoom in to 4x of fit

        // Mark as fitted and dirty
        commands.entity(entity).try_insert(AutoFitted);
        registry.dirty.push_back(tile.id);
    }
}

/// Draw only dirty tiles
pub fn draw_dirty_tiles(
    mut commands: Commands,
    mut registry: ResMut<TileRegistry>,
    tiles: Query<(Entity, &PlotTile, &TileRect, &TileView)>,
    children_q: Query<&Children>,
    is_root_q: Query<(), With<TileRenderRoot>>,
    dash: Res<DashboardRes>,
    unit: Res<UnitMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut scatter_points: ResMut<ScatterPoints3D>,
    mut axis_info_store: ResMut<AxisInfo3DStore>,
) {
    while let Some(id) = registry.dirty.pop_front() {
        // PlotId -> tile entity
        let Some(&tile_entity) = registry.by_plot.get(&id) else {
            continue;
        };

        // Pull current tile state
        let Ok((_e, tile, rect, view)) = tiles.get(tile_entity) else {
            continue;
        };

        // 1) Remove previous render root(s) under this tile (but keep the tile!)
        if let Ok(children) = children_q.get(tile_entity) {
            for child in children.iter() {
                if is_root_q.get(child).is_ok() {
                    // In Bevy 0.17, despawning an entity removes its descendants via relationships.
                    // Use try_despawn to avoid errors if entity was already despawned.
                    commands.entity(child).try_despawn();
                }
            }
        }

        // 2) Create a fresh render root under the tile
        let root = commands
            .spawn((TileRenderRoot, Transform::default(), Visibility::default()))
            .id();
        commands.entity(tile_entity).add_child(root);

        // 3) Draw based on plot type
        if let Some(plot) = dash.0.active_plots().get(tile.index) {
            let layer = RenderLayers::layer(tile.index % 32);
            match plot {
                crate::core::Plot::Graph2D(graph) => {
                    draw_plot_title(&mut commands, root, &graph.meta, rect, layer.clone());
                    draw_2d_plot(
                        &mut commands,
                        root,
                        graph,
                        rect,
                        view,
                        &unit,
                        &mut meshes,
                        &mut materials,
                        layer.clone(),
                    );
                    // Draw axis ticks with value labels
                    draw_axis_ticks(
                        &mut commands,
                        root,
                        rect,
                        view,
                        &unit,
                        &mut materials,
                        layer,
                    );
                }
                crate::core::Plot::Distribution(dist) => match dist {
                    crate::core::Distribution::Histogram {
                        meta,
                        values,
                        bins,
                        style,
                        x_label,
                        y_label,
                    } => {
                        draw_plot_title(&mut commands, root, meta, rect, layer.clone());
                        draw_histogram(
                            &mut commands,
                            root,
                            values,
                            *bins,
                            style,
                            x_label.as_deref(),
                            y_label.as_deref(),
                            rect,
                            view,
                            &unit,
                            &mut materials,
                            layer,
                        );
                    }
                    crate::core::Distribution::Pdf {
                        meta,
                        values,
                        style,
                        x_label,
                        y_label,
                    } => {
                        draw_plot_title(&mut commands, root, meta, rect, layer.clone());
                        draw_pdf(
                            &mut commands,
                            root,
                            values,
                            style,
                            x_label.as_deref(),
                            y_label.as_deref(),
                            rect,
                            view,
                            &unit,
                            &mut meshes,
                            &mut materials,
                            layer,
                        );
                    }
                    crate::core::Distribution::BoxPlot {
                        meta,
                        groups,
                        style,
                        x_label,
                        y_label,
                    } => {
                        draw_plot_title(&mut commands, root, meta, rect, layer.clone());
                        draw_boxplot(
                            &mut commands,
                            root,
                            groups,
                            style,
                            x_label.as_deref(),
                            y_label.as_deref(),
                            rect,
                            view,
                            &unit,
                            &mut materials,
                            layer,
                        );
                    }
                    crate::core::Distribution::ECDF {
                        meta,
                        values,
                        style,
                        x_label,
                        y_label,
                    } => {
                        draw_plot_title(&mut commands, root, meta, rect, layer.clone());
                        draw_ecdf(
                            &mut commands,
                            root,
                            values,
                            style,
                            x_label.as_deref(),
                            y_label.as_deref(),
                            rect,
                            view,
                            &unit,
                            &mut meshes,
                            &mut materials,
                            layer,
                        );
                    }
                },
                crate::core::Plot::Candlestick(candle) => {
                    draw_plot_title(&mut commands, root, &candle.meta, rect, layer.clone());
                    draw_candlestick(
                        &mut commands,
                        root,
                        candle,
                        rect,
                        view,
                        &unit,
                        &mut materials,
                        layer,
                    );
                }
                crate::core::Plot::Heatmap(heatmap) => {
                    draw_plot_title(&mut commands, root, &heatmap.meta, rect, layer.clone());
                    draw_heatmap(
                        &mut commands,
                        root,
                        heatmap,
                        rect,
                        view,
                        &unit,
                        &mut materials,
                        layer,
                    );
                }
                crate::core::Plot::Radial(radial) => {
                    // Extract meta from radial variant
                    let meta = match radial {
                        crate::core::Radial::Pie { meta, .. } => meta,
                        crate::core::Radial::Radar { meta, .. } => meta,
                    };
                    draw_plot_title(&mut commands, root, meta, rect, layer.clone());
                    draw_radial(
                        &mut commands,
                        root,
                        radial,
                        rect,
                        &unit,
                        &mut meshes,
                        &mut materials,
                        layer,
                    );
                }
                crate::core::Plot::Graph3D(graph) => {
                    // Use overlay layer for title/border (offset by 16)
                    let overlay_layer = RenderLayers::layer((tile.index + 16) % 32);
                    draw_3d_plot(
                        &mut commands,
                        root,
                        graph,
                        rect,
                        tile.index,
                        &unit,
                        &mut meshes,
                        &mut materials,
                        &mut std_materials,
                        layer,
                        overlay_layer,
                        &mut scatter_points,
                        &mut axis_info_store,
                    );
                }
                crate::core::Plot::Field(_) => {
                    draw_placeholder(&mut commands, root, rect, &unit, &mut materials, layer);
                }
            }
        }
    }
}

// Utility functions for grid layout
fn grid_dims(n: usize, aspect: f32, configured_cols: Option<usize>) -> (usize, usize) {
    // If columns are explicitly configured, use that
    if let Some(cols) = configured_cols {
        let cols = cols.max(1);
        let rows = n.div_ceil(cols);
        return (cols, rows);
    }

    // Auto layout based on count and aspect ratio
    match n {
        0 => (0, 0),
        1 => (1, 1),
        2 => {
            if aspect > 1.35 {
                (2, 1)
            } else {
                (1, 2)
            }
        }
        3 => {
            if aspect > 1.35 {
                (3, 1)
            } else {
                (2, 2)
            }
        }
        _ => {
            let cols = (n as f32).sqrt().ceil() as usize;
            let rows = n.div_ceil(cols);
            (cols, rows)
        }
    }
}

fn cleanup_tile(commands: &mut Commands, registry: &mut TileRegistry, entity: Entity, id: PlotId) {
    commands.entity(entity).despawn();
    registry.by_plot.remove(&id);
    if let Some(cam) = registry.camera_of.remove(&id) {
        commands.entity(cam).try_despawn();
    }
    if let Some(overlay) = registry.overlay_of.remove(&id) {
        commands.entity(overlay).try_despawn();
    }
}

/// Find nearest data point on any trace in the graph
fn find_nearest_point(cursor_data: Vec2, graph: &crate::core::Graph2D) -> Option<Vec2> {
    let mut nearest: Option<(Vec2, f32)> = None;

    for layer in &graph.layers {
        // For FillBetween geometry, skip - we want to snap to actual trace points
        if matches!(layer.geometry, crate::core::Geometry2D::FillBetween) {
            continue;
        }

        for &pt in &layer.xy {
            let dist_sq = (pt.x - cursor_data.x).powi(2) + (pt.y - cursor_data.y).powi(2);
            let should_update = match &nearest {
                Some((_, best_dist)) => dist_sq < *best_dist,
                None => true,
            };
            if should_update {
                nearest = Some((pt, dist_sq));
            }
        }
    }

    nearest.map(|(pt, _)| pt)
}

/// Update crosshair position and visibility - snaps to nearest data point
pub fn update_crosshair(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    tiles: Query<(&PlotTile, &TileRect, &TileView, Option<&View3D>)>,
    hovered: Res<HoveredTile>,
    dash: Res<DashboardRes>,
    mut cursor_pos: ResMut<CursorWorldPos>,
    crosshairs: Query<(Entity, &Crosshair)>,
    tooltips_3d: Query<(Entity, &Tooltip3D)>,
    scatter_points: Res<ScatterPoints3D>,
    unit: Res<UnitMeshes>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    // Despawn all existing crosshairs and 3D tooltips first (we recreate each frame)
    for (entity, _) in crosshairs.iter() {
        commands.entity(entity).try_despawn();
    }
    for (entity, _) in tooltips_3d.iter() {
        commands.entity(entity).try_despawn();
    }

    let Some(cursor_screen) = window.cursor_position() else {
        cursor_pos.position = None;
        cursor_pos.data_coords = None;
        cursor_pos.tile_index = None;
        return;
    };

    // Convert screen coords to world coords
    let world_x = cursor_screen.x - window.width() * 0.5;
    let world_y = window.height() * 0.5 - cursor_screen.y;
    let cursor_world = Vec2::new(world_x, world_y);

    cursor_pos.position = Some(cursor_world);
    cursor_pos.tile_index = hovered.0;

    let Some(hovered_index) = hovered.0 else {
        return;
    };

    for (tile, rect, view, view_3d) in tiles.iter() {
        if tile.index != hovered_index {
            continue;
        }

        // Get the graph data for this tile
        let Some(plot) = dash.0.active_plots().get(tile.index) else {
            continue;
        };

        match plot {
            crate::core::Plot::Graph3D(_) => {
                // Handle 3D scatter tooltip
                if let Some(view3d) = view_3d {
                    if let Some(points) = scatter_points.points.get(&tile.index) {
                        spawn_3d_tooltip(
                            &mut commands,
                            tile.index,
                            rect,
                            cursor_world,
                            points,
                            view3d,
                            &unit,
                            &mut materials,
                        );
                    }
                }
            }
            crate::core::Plot::Graph2D(graph) => {
                // Convert cursor to data coordinates
                let cursor_data = world_to_data(cursor_world, rect, view);

                // Find nearest data point
                let snap_data = find_nearest_point(cursor_data, graph).unwrap_or(cursor_data);
                let snap_world = data_to_world(snap_data, rect, view);

                cursor_pos.data_coords = Some(snap_data);

                // Spawn crosshair with dashed lines
                spawn_dashed_crosshair(
                    &mut commands,
                    tile.index,
                    rect,
                    snap_world,
                    snap_data,
                    &unit,
                    &mut materials,
                    RenderLayers::layer(tile.index % 32),
                );
            }
            crate::core::Plot::Distribution(dist) => {
                // Spawn distribution crosshair with tooltip
                spawn_distribution_crosshair(
                    &mut commands,
                    tile.index,
                    rect,
                    cursor_world,
                    dist,
                    &unit,
                    &mut materials,
                    RenderLayers::layer(tile.index % 32),
                );
            }
            crate::core::Plot::Candlestick(candle) => {
                spawn_candlestick_tooltip(
                    &mut commands,
                    tile.index,
                    rect,
                    view,
                    cursor_world,
                    candle,
                    &unit,
                    &mut materials,
                    RenderLayers::layer(tile.index % 32),
                );
            }
            crate::core::Plot::Heatmap(heatmap) => {
                spawn_heatmap_tooltip(
                    &mut commands,
                    tile.index,
                    rect,
                    cursor_world,
                    heatmap,
                    &unit,
                    &mut materials,
                    RenderLayers::layer(tile.index % 32),
                );
            }
            crate::core::Plot::Radial(radial) => {
                spawn_radial_tooltip(
                    &mut commands,
                    tile.index,
                    rect,
                    cursor_world,
                    radial,
                    &unit,
                    &mut materials,
                    RenderLayers::layer(tile.index % 32),
                );
            }
            _ => {}
        }
    }
}

fn spawn_dashed_crosshair(
    commands: &mut Commands,
    tile_index: usize,
    rect: &TileRect,
    snap_world: Vec2,
    snap_data: Vec2,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    let crosshair_mat = materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.5)));
    let line_thickness = 1.0; // Thin but visible
    let dash_length = 4.0;
    let gap_length = 3.0;

    commands
        .spawn((
            Crosshair { tile_index },
            Transform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .with_children(|parent| {
            // Dashed vertical line
            let v_start = rect.world_center.y - rect.world_size.y * 0.5;
            let v_end = rect.world_center.y + rect.world_size.y * 0.5;
            let mut y = v_start;
            while y < v_end {
                let dash_end = (y + dash_length).min(v_end);
                let dash_center_y = (y + dash_end) / 2.0;
                let dash_height = dash_end - y;

                parent.spawn((
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(crosshair_mat.clone()),
                    Transform {
                        translation: Vec3::new(snap_world.x, dash_center_y, 5.0),
                        scale: Vec3::new(line_thickness, dash_height, 1.0),
                        ..default()
                    },
                    CrosshairVLine,
                    layers.clone(),
                ));

                y += dash_length + gap_length;
            }

            // Dashed horizontal line
            let h_start = rect.world_center.x - rect.world_size.x * 0.5;
            let h_end = rect.world_center.x + rect.world_size.x * 0.5;
            let mut x = h_start;
            while x < h_end {
                let dash_end = (x + dash_length).min(h_end);
                let dash_center_x = (x + dash_end) / 2.0;
                let dash_width = dash_end - x;

                parent.spawn((
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(crosshair_mat.clone()),
                    Transform {
                        translation: Vec3::new(dash_center_x, snap_world.y, 5.0),
                        scale: Vec3::new(dash_width, line_thickness, 1.0),
                        ..default()
                    },
                    CrosshairHLine,
                    layers.clone(),
                ));

                x += dash_length + gap_length;
            }

            // Small circle marker at intersection (snap point)
            let point_mat = materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.95)));
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(point_mat),
                Transform {
                    translation: Vec3::new(snap_world.x, snap_world.y, 5.5),
                    scale: Vec3::splat(5.0),
                    ..default()
                },
                layers.clone(),
            ));

            // Coordinate text near the snapped point (offset to avoid overlap)
            parent.spawn((
                Text2d::new(format!("({:.2}, {:.2})", snap_data.x, snap_data.y)),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                Transform::from_translation(Vec3::new(
                    snap_world.x + 10.0,
                    snap_world.y + 12.0,
                    6.0,
                )),
                CrosshairCoordText,
                layers,
            ));
        });
}

fn spawn_distribution_crosshair(
    commands: &mut Commands,
    tile_index: usize,
    rect: &TileRect,
    cursor_world: Vec2,
    dist: &crate::core::Distribution,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    // Calculate padded area (must match draw_histogram/draw_pdf)
    let padding_left = 0.15;
    let padding_right = 0.08;
    let padding_bottom = 0.18;
    let padding_top = 0.08;

    let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
    let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
    let left_x = rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
    let bottom_y =
        rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;
    let right_x = left_x + usable_width;
    let top_y = bottom_y + usable_height;

    // Check if cursor is within the plot area
    if cursor_world.x < left_x
        || cursor_world.x > right_x
        || cursor_world.y < bottom_y
        || cursor_world.y > top_y
    {
        return;
    }

    let crosshair_mat = materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.5)));
    let highlight_mat = materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.3)));
    let line_thickness = 1.0;
    let dash_length = 4.0;
    let gap_length = 3.0;

    match dist {
        crate::core::Distribution::Histogram { values, bins, .. } => {
            // Histogram: just bar highlight + tooltip, no crosshair lines
            if values.is_empty() || *bins == 0 {
                return;
            }

            let min_val = values.iter().cloned().fold(f32::INFINITY, f32::min);
            let max_val = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            if !min_val.is_finite() || !max_val.is_finite() || min_val >= max_val {
                return;
            }

            let bin_width_data = (max_val - min_val) / *bins as f32;
            let bar_width_world = usable_width / *bins as f32;

            // Which bar is the cursor over?
            let bar_index = ((cursor_world.x - left_x) / bar_width_world).floor() as usize;
            let bar_index = bar_index.min(*bins - 1);

            // Count values in this bin
            let bin_start = min_val + bar_index as f32 * bin_width_data;
            let bin_end = bin_start + bin_width_data;
            let count = values
                .iter()
                .filter(|&&v| {
                    if bar_index == *bins - 1 {
                        v >= bin_start && v <= bin_end
                    } else {
                        v >= bin_start && v < bin_end
                    }
                })
                .count();

            // Calculate bar geometry for highlight
            let bar_center_x = left_x + (bar_index as f32 + 0.5) * bar_width_world;
            let max_count = {
                let mut counts = vec![0usize; *bins];
                for &v in values {
                    let idx = ((v - min_val) / bin_width_data).floor() as usize;
                    let idx = idx.min(*bins - 1);
                    counts[idx] += 1;
                }
                counts.iter().cloned().max().unwrap_or(1) as f32
            };
            let bar_height = (count as f32 / max_count) * usable_height;

            let tooltip_text = format!("[{:.2}, {:.2})\nCount: {}", bin_start, bin_end, count);

            commands
                .spawn((
                    Crosshair { tile_index },
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ))
                .with_children(|parent| {
                    // Highlight bar overlay
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(highlight_mat),
                        Transform {
                            translation: Vec3::new(bar_center_x, bottom_y + bar_height * 0.5, 4.0),
                            scale: Vec3::new(bar_width_world * 0.9, bar_height, 1.0),
                            ..default()
                        },
                        layers.clone(),
                    ));

                    // Tooltip text
                    parent.spawn((
                        Text2d::new(tooltip_text),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                        Transform::from_translation(Vec3::new(
                            bar_center_x + 15.0,
                            bottom_y + bar_height + 20.0,
                            6.0,
                        )),
                        CrosshairCoordText,
                        layers,
                    ));
                });
        }
        crate::core::Distribution::Pdf { values, .. } => {
            // PDF: crosshair snapped to curve
            if values.is_empty() {
                return;
            }

            let min_val = values.iter().cloned().fold(f32::INFINITY, f32::min);
            let max_val = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            if !min_val.is_finite() || !max_val.is_finite() || min_val >= max_val {
                return;
            }

            // Convert cursor x to data value
            let t = (cursor_world.x - left_x) / usable_width;
            let range = max_val - min_val;
            let x_min = min_val - range * 0.1;
            let x_max = max_val + range * 0.1;
            let data_x = x_min + t * (x_max - x_min);

            // Compute KDE at this point
            let n = values.len() as f32;
            let std_dev = {
                let mean = values.iter().sum::<f32>() / n;
                let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
                variance.sqrt()
            };
            let bandwidth = (1.06 * std_dev * n.powf(-0.2)).max(0.01);

            let density: f32 = values
                .iter()
                .map(|&xi| {
                    let u = (data_x - xi) / bandwidth;
                    (-0.5 * u * u).exp() / (2.506628 * bandwidth)
                })
                .sum::<f32>()
                / n;

            // Compute max density for normalization
            let max_density = {
                let mut max_d = 0.0f32;
                for i in 0..100 {
                    let x = x_min + (i as f32 / 99.0) * (x_max - x_min);
                    let d: f32 = values
                        .iter()
                        .map(|&xi| {
                            let u = (x - xi) / bandwidth;
                            (-0.5 * u * u).exp() / (2.506628 * bandwidth)
                        })
                        .sum::<f32>()
                        / n;
                    max_d = max_d.max(d);
                }
                max_d
            };

            // Snap to curve - y position on the PDF curve
            let snap_y = bottom_y + (density / max_density) * usable_height;
            let snap_world = Vec2::new(cursor_world.x, snap_y);

            let tooltip_text = format!("Value: {:.2}\nDensity: {:.4}", data_x, density);

            commands
                .spawn((
                    Crosshair { tile_index },
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ))
                .with_children(|parent| {
                    // Dashed vertical line
                    let mut y = bottom_y;
                    while y < snap_y {
                        let dash_end = (y + dash_length).min(snap_y);
                        let dash_center_y = (y + dash_end) / 2.0;
                        let dash_height = dash_end - y;

                        parent.spawn((
                            Mesh2d(unit.quad.clone()),
                            MeshMaterial2d(crosshair_mat.clone()),
                            Transform {
                                translation: Vec3::new(cursor_world.x, dash_center_y, 5.0),
                                scale: Vec3::new(line_thickness, dash_height, 1.0),
                                ..default()
                            },
                            CrosshairVLine,
                            layers.clone(),
                        ));

                        y += dash_length + gap_length;
                    }

                    // Dashed horizontal line to left edge
                    let mut x = left_x;
                    while x < cursor_world.x {
                        let dash_end = (x + dash_length).min(cursor_world.x);
                        let dash_center_x = (x + dash_end) / 2.0;
                        let dash_width = dash_end - x;

                        parent.spawn((
                            Mesh2d(unit.quad.clone()),
                            MeshMaterial2d(crosshair_mat.clone()),
                            Transform {
                                translation: Vec3::new(dash_center_x, snap_y, 5.0),
                                scale: Vec3::new(dash_width, line_thickness, 1.0),
                                ..default()
                            },
                            CrosshairHLine,
                            layers.clone(),
                        ));

                        x += dash_length + gap_length;
                    }

                    // Point marker at curve intersection
                    let point_mat =
                        materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.95)));
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(point_mat),
                        Transform {
                            translation: Vec3::new(snap_world.x, snap_world.y, 5.5),
                            scale: Vec3::splat(6.0),
                            ..default()
                        },
                        layers.clone(),
                    ));

                    // Tooltip text
                    parent.spawn((
                        Text2d::new(tooltip_text),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                        Transform::from_translation(Vec3::new(
                            snap_world.x + 15.0,
                            snap_world.y + 15.0,
                            6.0,
                        )),
                        CrosshairCoordText,
                        layers,
                    ));
                });
        }
        crate::core::Distribution::BoxPlot { groups, .. } => {
            // BoxPlot: highlight hovered box and show stats tooltip
            if groups.is_empty() {
                return;
            }

            let padding_left = 0.12;
            let padding_right = 0.05;
            let padding_bottom = 0.18;
            let padding_top = 0.08;

            let usable_width = rect.world_size.x * (1.0 - padding_left - padding_right);
            let usable_height = rect.world_size.y * (1.0 - padding_bottom - padding_top);
            let left_x =
                rect.world_center.x - rect.world_size.x * 0.5 + rect.world_size.x * padding_left;
            let bottom_y =
                rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;
            let right_x = left_x + usable_width;
            let top_y = bottom_y + usable_height;

            if cursor_world.x < left_x
                || cursor_world.x > right_x
                || cursor_world.y < bottom_y
                || cursor_world.y > top_y
            {
                return;
            }

            let n_groups = groups.len();
            let group_width = usable_width / n_groups as f32;
            let box_index = ((cursor_world.x - left_x) / group_width).floor() as usize;
            let box_index = box_index.min(n_groups - 1);

            let (name, values) = &groups[box_index];

            // Compute stats
            if values.is_empty() {
                return;
            }
            let mut sorted: Vec<f32> = values.iter().cloned().filter(|x| x.is_finite()).collect();
            if sorted.is_empty() {
                return;
            }
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = sorted.len();
            let median = if n % 2 == 0 {
                (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
            } else {
                sorted[n / 2]
            };
            let q1 = sorted[n / 4];
            let q3 = sorted[(3 * n / 4).min(n - 1)];
            let min_val = sorted[0];
            let max_val = sorted[n - 1];

            let box_center_x = left_x + (box_index as f32 + 0.5) * group_width;
            let box_width = group_width * 0.6;

            // Highlight the box
            let highlight_mat =
                materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.15)));
            commands
                .spawn((
                    Crosshair { tile_index },
                    Transform::default(),
                    Visibility::Visible,
                    layers.clone(),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(highlight_mat),
                        Transform {
                            translation: Vec3::new(box_center_x, (bottom_y + top_y) * 0.5, 4.5),
                            scale: Vec3::new(box_width + 10.0, usable_height, 1.0),
                            ..default()
                        },
                        layers.clone(),
                    ));

                    // Tooltip
                    let tooltip = format!(
                        "{}\nMedian: {:.2}\nQ1: {:.2}  Q3: {:.2}\nMin: {:.2}  Max: {:.2}",
                        name, median, q1, q3, min_val, max_val
                    );
                    parent.spawn((
                        Text2d::new(tooltip),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                        Transform::from_translation(Vec3::new(
                            cursor_world.x + 15.0,
                            cursor_world.y + 25.0,
                            6.0,
                        )),
                        CrosshairCoordText,
                        layers,
                    ));
                });
        }
        crate::core::Distribution::ECDF { values, .. } => {
            // ECDF: show F(x) value at cursor position
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

            // Convert cursor x to data value
            let range = max_val - min_val;
            let x_min = min_val - range * 0.05;
            let x_max = max_val + range * 0.05;

            let t = (cursor_world.x - left_x) / usable_width;
            let data_x = x_min + t * (x_max - x_min);

            // Compute ECDF value at this point
            let count = sorted.iter().filter(|&&v| v <= data_x).count();
            let ecdf_y = count as f32 / sorted.len() as f32;

            // Snap to curve
            let snap_world_y = bottom_y + ecdf_y * usable_height;

            let tooltip_text = format!("x: {:.2}\nF(x): {:.3}", data_x, ecdf_y);

            commands
                .spawn((
                    Crosshair { tile_index },
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ))
                .with_children(|parent| {
                    // Dashed vertical line
                    let mut y = bottom_y;
                    while y < snap_world_y {
                        let dash_end = (y + dash_length).min(snap_world_y);
                        let dash_center_y = (y + dash_end) / 2.0;
                        let dash_height = dash_end - y;

                        parent.spawn((
                            Mesh2d(unit.quad.clone()),
                            MeshMaterial2d(crosshair_mat.clone()),
                            Transform {
                                translation: Vec3::new(cursor_world.x, dash_center_y, 5.0),
                                scale: Vec3::new(line_thickness, dash_height, 1.0),
                                ..default()
                            },
                            CrosshairVLine,
                            layers.clone(),
                        ));

                        y += dash_length + gap_length;
                    }

                    // Point marker
                    let point_mat =
                        materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.95)));
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(point_mat),
                        Transform {
                            translation: Vec3::new(cursor_world.x, snap_world_y, 5.5),
                            scale: Vec3::splat(6.0),
                            ..default()
                        },
                        layers.clone(),
                    ));

                    // Tooltip text
                    parent.spawn((
                        Text2d::new(tooltip_text),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                        Transform::from_translation(Vec3::new(
                            cursor_world.x + 15.0,
                            snap_world_y + 15.0,
                            6.0,
                        )),
                        CrosshairCoordText,
                        layers,
                    ));
                });
        }
    }
}

fn spawn_candlestick_tooltip(
    commands: &mut Commands,
    tile_index: usize,
    rect: &TileRect,
    view: &TileView,
    cursor_world: Vec2,
    candle: &crate::core::Candlestick,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    if candle.candles.is_empty() {
        return;
    }

    // Convert cursor to data coordinates using view transformation
    let cursor_data = world_to_data(cursor_world, rect, view);

    // Calculate candle width in data units (same as draw_candlestick)
    let n_candles = candle.candles.len();
    let x_min = candle
        .candles
        .iter()
        .map(|c| c.x)
        .fold(f32::INFINITY, f32::min);
    let x_max = candle
        .candles
        .iter()
        .map(|c| c.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let x_range = (x_max - x_min).max(1.0);
    let candle_data_width = x_range / (n_candles as f32 * 1.5);

    // Find nearest candle in data space
    let mut nearest_idx = 0;
    let mut nearest_dist = f32::INFINITY;
    for (i, c) in candle.candles.iter().enumerate() {
        let dist = (c.x - cursor_data.x).abs();
        if dist < nearest_dist {
            nearest_dist = dist;
            nearest_idx = i;
        }
    }

    let c = &candle.candles[nearest_idx];

    // Check if cursor is close enough to this candle (in data space)
    if (cursor_data.x - c.x).abs() > candle_data_width * 0.8 {
        return;
    }

    // Transform candle position to world coordinates using view
    let candle_world = data_to_world(Vec2::new(c.x, (c.high + c.low) * 0.5), rect, view);
    let candle_high_world = data_to_world(Vec2::new(c.x, c.high), rect, view);
    let candle_low_world = data_to_world(Vec2::new(c.x, c.low), rect, view);

    // Candle width in world space
    let candle_world_width = candle_data_width * view.scale;

    let is_up = c.close >= c.open;
    let highlight_color = if is_up {
        Color::srgba(0.3, 1.0, 0.4, 0.3)
    } else {
        Color::srgba(1.0, 0.3, 0.3, 0.3)
    };
    let highlight_mat = materials.add(ColorMaterial::from(highlight_color));

    commands
        .spawn((
            Crosshair { tile_index },
            Transform::default(),
            Visibility::Visible,
            layers.clone(),
        ))
        .with_children(|parent| {
            // Highlight column
            let wick_height = (candle_high_world.y - candle_low_world.y).abs();
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(highlight_mat),
                Transform {
                    translation: Vec3::new(candle_world.x, candle_world.y, 4.5),
                    scale: Vec3::new(candle_world_width + 4.0, wick_height + 4.0, 1.0),
                    ..default()
                },
                layers.clone(),
            ));

            // Tooltip
            let change = c.close - c.open;
            let change_pct = if c.open != 0.0 {
                (change / c.open) * 100.0
            } else {
                0.0
            };
            let change_str = if change >= 0.0 {
                format!("+{:.2} (+{:.1}%)", change, change_pct)
            } else {
                format!("{:.2} ({:.1}%)", change, change_pct)
            };

            let tooltip = format!(
                "O: {:.2}  H: {:.2}\nL: {:.2}  C: {:.2}\n{}",
                c.open, c.high, c.low, c.close, change_str
            );

            parent.spawn((
                Text2d::new(tooltip),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                Transform::from_translation(Vec3::new(
                    cursor_world.x + 15.0,
                    cursor_world.y + 20.0,
                    6.0,
                )),
                CrosshairCoordText,
                layers,
            ));
        });
}

fn spawn_heatmap_tooltip(
    commands: &mut Commands,
    tile_index: usize,
    rect: &TileRect,
    cursor_world: Vec2,
    heatmap: &crate::core::Heatmap,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    let rows = heatmap.dims.y as usize;
    let cols = heatmap.dims.x as usize;

    if rows == 0 || cols == 0 || heatmap.values.is_empty() {
        return;
    }

    // Match padding from draw_heatmap exactly
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
    let right_x = left_x + usable_width;
    let top_y = bottom_y + usable_height;

    if cursor_world.x < left_x
        || cursor_world.x > right_x
        || cursor_world.y < bottom_y
        || cursor_world.y > top_y
    {
        return;
    }

    let cell_width = usable_width / cols as f32;
    let cell_height = usable_height / rows as f32;

    // Find which cell
    let col = ((cursor_world.x - left_x) / cell_width).floor() as usize;
    let col = col.min(cols - 1);

    // Flip y (row 0 is at top)
    let row_from_bottom = ((cursor_world.y - bottom_y) / cell_height).floor() as usize;
    let row = rows.saturating_sub(1).saturating_sub(row_from_bottom);
    let row = row.min(rows - 1);

    let idx = row * cols + col;
    if idx >= heatmap.values.len() {
        return;
    }

    let value = heatmap.values[idx];

    // Cell center
    let cell_x = left_x + (col as f32 + 0.5) * cell_width;
    let cell_y = bottom_y + usable_height - (row as f32 + 0.5) * cell_height;

    // Highlight color based on value
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
    let _t = ((value - vmin) / (vmax - vmin).max(0.001)).clamp(0.0, 1.0);

    let highlight_mat = materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.25)));

    commands
        .spawn((
            Crosshair { tile_index },
            Transform::default(),
            Visibility::Visible,
            layers.clone(),
        ))
        .with_children(|parent| {
            // Highlight cell border
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(highlight_mat),
                Transform {
                    translation: Vec3::new(cell_x, cell_y, 4.5),
                    scale: Vec3::new(cell_width + 2.0, cell_height + 2.0, 1.0),
                    ..default()
                },
                layers.clone(),
            ));

            // Build tooltip
            let default_row = format!("Row {}", row);
            let default_col = format!("Col {}", col);
            let row_label = heatmap
                .row_labels
                .as_ref()
                .and_then(|l| l.get(row))
                .map(|s| s.as_str())
                .unwrap_or(&default_row);
            let col_label = heatmap
                .col_labels
                .as_ref()
                .and_then(|l| l.get(col))
                .map(|s| s.as_str())
                .unwrap_or(&default_col);

            let tooltip = format!("{} x {}\nValue: {:.3}", row_label, col_label, value);

            parent.spawn((
                Text2d::new(tooltip),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                Transform::from_translation(Vec3::new(
                    cursor_world.x + 15.0,
                    cursor_world.y + 15.0,
                    6.0,
                )),
                CrosshairCoordText,
                layers,
            ));
        });
}

fn spawn_radial_tooltip(
    commands: &mut Commands,
    tile_index: usize,
    rect: &TileRect,
    cursor_world: Vec2,
    radial: &crate::core::Radial,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
) {
    let center = rect.world_center;
    let radius = (rect.world_size.x.min(rect.world_size.y) * 0.35).max(10.0);

    // Distance from center
    let dx = cursor_world.x - center.x;
    let dy = cursor_world.y - center.y;
    let dist = (dx * dx + dy * dy).sqrt();

    // Only show tooltip when cursor is near the chart
    if dist > radius * 1.5 {
        return;
    }

    match radial {
        crate::core::Radial::Pie { slices, .. } => {
            if slices.is_empty() {
                return;
            }

            let total: f32 = slices.iter().map(|(_, v)| v.max(0.0)).sum();
            if total <= 0.0 {
                return;
            }

            // Find which slice the cursor is in
            let angle = dy.atan2(dx);
            // Normalize angle to start from top (-PI/2) going clockwise
            let mut normalized = angle + std::f32::consts::FRAC_PI_2;
            if normalized < 0.0 {
                normalized += std::f32::consts::TAU;
            }

            let mut cumulative = 0.0;
            let mut found_slice: Option<(usize, &str, f32, f32)> = None;

            for (i, (label, value)) in slices.iter().enumerate() {
                if *value <= 0.0 {
                    continue;
                }
                let sweep = (*value / total) * std::f32::consts::TAU;
                if normalized >= cumulative && normalized < cumulative + sweep {
                    let pct = (*value / total) * 100.0;
                    found_slice = Some((i, label, *value, pct));
                    break;
                }
                cumulative += sweep;
            }

            if let Some((_idx, label, value, pct)) = found_slice {
                let tooltip = format!("{}\nValue: {:.1}\n{:.1}%", label, value, pct);

                let highlight_mat =
                    materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.3)));

                commands
                    .spawn((
                        Crosshair { tile_index },
                        Transform::default(),
                        Visibility::Visible,
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                    ))
                    .with_children(|parent| {
                        // Small highlight at cursor
                        parent.spawn((
                            Mesh2d(unit.quad.clone()),
                            MeshMaterial2d(highlight_mat),
                            Transform {
                                translation: Vec3::new(cursor_world.x, cursor_world.y, 4.5),
                                scale: Vec3::splat(8.0),
                                ..default()
                            },
                            layers.clone(),
                        ));

                        // Tooltip
                        parent.spawn((
                            Text2d::new(tooltip),
                            TextFont {
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                            Transform::from_translation(Vec3::new(
                                cursor_world.x + 15.0,
                                cursor_world.y + 15.0,
                                6.0,
                            )),
                            CrosshairCoordText,
                            layers,
                        ));
                    });
            }
        }
        crate::core::Radial::Radar { axes, values, .. } => {
            if axes.is_empty() || values.is_empty() {
                return;
            }

            let n = axes.len().min(values.len());
            if n < 3 {
                return;
            }

            // Find which axis the cursor is closest to
            let angle = dy.atan2(dx);
            let angle_step = std::f32::consts::TAU / n as f32;

            let mut best_idx = 0;
            let mut best_diff = f32::MAX;

            for i in 0..n {
                let axis_angle = -std::f32::consts::FRAC_PI_2 + i as f32 * angle_step;
                let mut diff = (angle - axis_angle).abs();
                if diff > std::f32::consts::PI {
                    diff = std::f32::consts::TAU - diff;
                }
                if diff < best_diff {
                    best_diff = diff;
                    best_idx = i;
                }
            }

            let axis_name = &axes[best_idx];
            let axis_value = values[best_idx];

            let tooltip = format!("{}\nValue: {:.2}", axis_name, axis_value);

            let highlight_mat =
                materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.3)));

            // Calculate point position on the radar
            let axis_angle = -std::f32::consts::FRAC_PI_2 + best_idx as f32 * angle_step;
            let r = radius * axis_value.clamp(0.0, 1.0);
            let point_x = center.x + r * axis_angle.cos();
            let point_y = center.y + r * axis_angle.sin();

            commands
                .spawn((
                    Crosshair { tile_index },
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ))
                .with_children(|parent| {
                    // Highlight the data point
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(highlight_mat),
                        Transform {
                            translation: Vec3::new(point_x, point_y, 4.5),
                            scale: Vec3::splat(12.0),
                            ..default()
                        },
                        layers.clone(),
                    ));

                    // Tooltip
                    parent.spawn((
                        Text2d::new(tooltip),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                        Transform::from_translation(Vec3::new(
                            cursor_world.x + 15.0,
                            cursor_world.y + 15.0,
                            6.0,
                        )),
                        CrosshairCoordText,
                        layers,
                    ));
                });
        }
    }
}

/// Spawn 3D scatter point tooltip
fn spawn_3d_tooltip(
    commands: &mut Commands,
    tile_index: usize,
    rect: &TileRect,
    cursor_world: Vec2,
    points: &[(Vec3, Vec3)], // (original, normalized)
    view: &View3D,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
) {
    // Project 3D points to 2D screen space and find nearest to cursor
    // We use a simplified projection based on the View3D camera

    // Get camera transform
    let cam_transform = compute_orbit_transform(view);
    let cam_pos = cam_transform.translation;
    let cam_forward = cam_transform.forward();
    let cam_right = cam_transform.right();
    let cam_up = cam_transform.up();

    // Viewport dimensions (use tile size as approximate)
    let vp_width = rect.world_size.x;
    let vp_height = rect.world_size.y;
    let fov = std::f32::consts::FRAC_PI_4; // 45 degrees
    let aspect = vp_width / vp_height;

    // Convert cursor from world to viewport-relative coords
    let cursor_vp = Vec2::new(
        (cursor_world.x - rect.world_center.x) / vp_width + 0.5,
        (cursor_world.y - rect.world_center.y) / vp_height + 0.5,
    );

    // If cursor is outside viewport, skip
    if cursor_vp.x < 0.0 || cursor_vp.x > 1.0 || cursor_vp.y < 0.0 || cursor_vp.y > 1.0 {
        return;
    }

    // Find nearest point by projecting to screen
    let mut nearest: Option<(Vec3, Vec2, f32)> = None; // (original_coords, screen_pos, dist_sq)
    let tan_half_fov = (fov * 0.5).tan();

    for &(original, normalized) in points {
        // Vector from camera to point
        let to_point = normalized - cam_pos;
        let depth = to_point.dot(*cam_forward);

        if depth <= 0.1 {
            continue; // Point is behind camera
        }

        // Project to normalized device coords
        let right_dist = to_point.dot(*cam_right);
        let up_dist = to_point.dot(*cam_up);

        let ndc_x = right_dist / (depth * tan_half_fov * aspect);
        let ndc_y = up_dist / (depth * tan_half_fov);

        // Convert to viewport coords [0, 1]
        let screen_x = (ndc_x + 1.0) * 0.5;
        let screen_y = (ndc_y + 1.0) * 0.5;

        // Distance from cursor
        let dx = screen_x - cursor_vp.x;
        let dy = screen_y - cursor_vp.y;
        let dist_sq = dx * dx + dy * dy;

        let should_update = match &nearest {
            Some((_, _, best_dist)) => dist_sq < *best_dist,
            None => true,
        };
        if should_update {
            let world_screen = Vec2::new(
                rect.world_center.x + (screen_x - 0.5) * vp_width,
                rect.world_center.y + (screen_y - 0.5) * vp_height,
            );
            nearest = Some((original, world_screen, dist_sq));
        }
    }

    // Check if nearest point is close enough (within ~5% of viewport)
    let threshold = 0.05 * 0.05; // 5% squared
    if let Some((original, screen_pos, dist_sq)) = nearest {
        if dist_sq < threshold {
            // Use overlay layer for tooltip
            let overlay_layer = RenderLayers::layer((tile_index + 16) % 32);
            let tooltip_text = format!("({:.2}, {:.2}, {:.2})", original.x, original.y, original.z);

            let highlight_mat =
                materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.6)));

            commands
                .spawn((
                    Tooltip3D { tile_index },
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ))
                .with_children(|parent| {
                    // Point marker at projected location
                    parent.spawn((
                        Mesh2d(unit.quad.clone()),
                        MeshMaterial2d(highlight_mat),
                        Transform {
                            translation: Vec3::new(screen_pos.x, screen_pos.y, 5.0),
                            scale: Vec3::splat(8.0),
                            ..default()
                        },
                        overlay_layer.clone(),
                    ));

                    // Tooltip text
                    parent.spawn((
                        Text2d::new(tooltip_text),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
                        Transform::from_translation(Vec3::new(
                            screen_pos.x + 15.0,
                            screen_pos.y + 15.0,
                            6.0,
                        )),
                        CrosshairCoordText,
                        overlay_layer,
                    ));
                });
        }
    }
}

/// Update 3D axis labels - projects axis endpoints to screen and shows axis names + tick values
pub fn update_3d_axis_labels(
    mut commands: Commands,
    tiles: Query<(&PlotTile, &TileRect, &PlotKind, Option<&View3D>)>,
    existing_labels: Query<(Entity, &AxisLabel3D)>,
    axis_info_store: Res<AxisInfo3DStore>,
) {
    // Despawn existing labels (we recreate each frame)
    for (entity, _) in existing_labels.iter() {
        commands.entity(entity).try_despawn();
    }

    // The normalized volume spans [-2.5, 2.5]
    let half_size = 2.5;
    let axis_len = 5.5;
    let origin = Vec3::new(-half_size, -half_size, -half_size);
    let x_tip = origin + Vec3::new(axis_len + 0.3, 0.0, 0.0);
    let y_tip = origin + Vec3::new(0.0, axis_len + 0.3, 0.0);
    let z_tip = origin + Vec3::new(0.0, 0.0, axis_len + 0.3);

    for (tile, rect, kind, view_3d) in tiles.iter() {
        // Only for 3D tiles
        if !matches!(kind, PlotKind::ThreeD) {
            continue;
        }

        let Some(view) = view_3d else {
            continue;
        };

        // Get axis info for this tile
        let axis_info = axis_info_store.info.get(&tile.index);

        // Get camera transform
        let cam_transform = compute_orbit_transform(view);
        let cam_pos = cam_transform.translation;
        let cam_forward = cam_transform.forward();
        let cam_right = cam_transform.right();
        let cam_up = cam_transform.up();

        let vp_width = rect.world_size.x;
        let vp_height = rect.world_size.y;
        let fov = std::f32::consts::FRAC_PI_4;
        let aspect = vp_width / vp_height;
        let tan_half_fov = (fov * 0.5).tan();

        // Project 3D point to screen
        let project_to_screen = |point: Vec3| -> Option<Vec2> {
            let to_point = point - cam_pos;
            let depth = to_point.dot(*cam_forward);
            if depth <= 0.1 {
                return None;
            }

            let right_dist = to_point.dot(*cam_right);
            let up_dist = to_point.dot(*cam_up);

            let ndc_x = right_dist / (depth * tan_half_fov * aspect);
            let ndc_y = up_dist / (depth * tan_half_fov);

            let screen_x = rect.world_center.x + ndc_x * vp_width * 0.5;
            let screen_y = rect.world_center.y + ndc_y * vp_height * 0.5;

            Some(Vec2::new(screen_x, screen_y))
        };

        // Use overlay layer for labels
        let overlay_layer = RenderLayers::layer((tile.index + 16) % 32);

        // Get axis labels (use default if not set)
        let x_label = axis_info
            .and_then(|i| i.x_label.clone())
            .unwrap_or_else(|| "X".to_string());
        let y_label = axis_info
            .and_then(|i| i.y_label.clone())
            .unwrap_or_else(|| "Y".to_string());
        let z_label = axis_info
            .and_then(|i| i.z_label.clone())
            .unwrap_or_else(|| "Z".to_string());

        // Spawn X axis label
        if let Some(screen_pos) = project_to_screen(x_tip) {
            commands.spawn((
                AxisLabel3D {
                    tile_index: tile.index,
                },
                Text2d::new(x_label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.5, 0.5)),
                Transform::from_translation(Vec3::new(screen_pos.x, screen_pos.y, 10.0)),
                overlay_layer.clone(),
            ));
        }

        // Spawn Y axis label
        if let Some(screen_pos) = project_to_screen(y_tip) {
            commands.spawn((
                AxisLabel3D {
                    tile_index: tile.index,
                },
                Text2d::new(y_label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 1.0, 0.5)),
                Transform::from_translation(Vec3::new(screen_pos.x, screen_pos.y, 10.0)),
                overlay_layer.clone(),
            ));
        }

        // Spawn Z axis label
        if let Some(screen_pos) = project_to_screen(z_tip) {
            commands.spawn((
                AxisLabel3D {
                    tile_index: tile.index,
                },
                Text2d::new(z_label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.7, 1.0)),
                Transform::from_translation(Vec3::new(screen_pos.x, screen_pos.y, 10.0)),
                overlay_layer.clone(),
            ));
        }

        // Add tick values along axes if we have axis info
        if let Some(info) = axis_info {
            let tick_color = Color::srgba(0.8, 0.8, 0.8, 0.8);
            let num_ticks = 5;

            // X-axis ticks (along bottom edge at y=-2.5, z=-2.5)
            for i in 0..=num_ticks {
                let t = i as f32 / num_ticks as f32;
                let normalized_x = -half_size + t * (half_size * 2.0);
                let data_x = info.bounds_min[0] + t * (info.bounds_max[0] - info.bounds_min[0]);
                let tick_pos = Vec3::new(normalized_x, -half_size - 0.3, -half_size);

                if let Some(screen_pos) = project_to_screen(tick_pos) {
                    commands.spawn((
                        AxisLabel3D {
                            tile_index: tile.index,
                        },
                        Text2d::new(format!("{:.1}", data_x)),
                        TextFont {
                            font_size: 9.0,
                            ..default()
                        },
                        TextColor(tick_color),
                        Transform::from_translation(Vec3::new(screen_pos.x, screen_pos.y, 10.0)),
                        overlay_layer.clone(),
                    ));
                }
            }

            // Y-axis ticks (along left edge at x=-2.5, z=-2.5)
            for i in 0..=num_ticks {
                let t = i as f32 / num_ticks as f32;
                let normalized_y = -half_size + t * (half_size * 2.0);
                let data_y = info.bounds_min[1] + t * (info.bounds_max[1] - info.bounds_min[1]);
                let tick_pos = Vec3::new(-half_size - 0.3, normalized_y, -half_size);

                if let Some(screen_pos) = project_to_screen(tick_pos) {
                    commands.spawn((
                        AxisLabel3D {
                            tile_index: tile.index,
                        },
                        Text2d::new(format!("{:.1}", data_y)),
                        TextFont {
                            font_size: 9.0,
                            ..default()
                        },
                        TextColor(tick_color),
                        Transform::from_translation(Vec3::new(screen_pos.x, screen_pos.y, 10.0)),
                        overlay_layer.clone(),
                    ));
                }
            }

            // Z-axis ticks (along bottom edge at x=-2.5, y=-2.5)
            for i in 0..=num_ticks {
                let t = i as f32 / num_ticks as f32;
                let normalized_z = -half_size + t * (half_size * 2.0);
                let data_z = info.bounds_min[2] + t * (info.bounds_max[2] - info.bounds_min[2]);
                let tick_pos = Vec3::new(-half_size, -half_size - 0.3, normalized_z);

                if let Some(screen_pos) = project_to_screen(tick_pos) {
                    commands.spawn((
                        AxisLabel3D {
                            tile_index: tile.index,
                        },
                        Text2d::new(format!("{:.1}", data_z)),
                        TextFont {
                            font_size: 9.0,
                            ..default()
                        },
                        TextColor(tick_color),
                        Transform::from_translation(Vec3::new(screen_pos.x, screen_pos.y, 10.0)),
                        overlay_layer.clone(),
                    ));
                }
            }
        }
    }
}

/// Spawn or update the tab bar UI
pub fn update_tab_bar(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    dash: Res<DashboardRes>,
    existing_tabs: Query<Entity, With<TabBar>>,
    unit: Res<UnitMeshes>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Remove existing tab bar
    for entity in existing_tabs.iter() {
        commands.entity(entity).despawn();
    }

    // Only render if we have tabs
    if !dash.0.has_tabs() {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let tab_names = dash.0.tab_names();
    let active_tab = dash.0.active_tab;
    let tab_bar_height = 35.0;
    let tab_width = 100.0;
    let tab_gap = 5.0;
    let margin = 20.0;

    // Tab bar background
    let bar_mat = materials.add(ColorMaterial::from(Color::srgba(0.15, 0.15, 0.2, 0.95)));

    let layers = RenderLayers::layer(0);

    commands
        .spawn((
            TabBar,
            Transform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            layers.clone(),
        ))
        .with_children(|parent| {
            // Background bar
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(bar_mat),
                Transform {
                    translation: Vec3::new(
                        0.0,
                        window.height() * 0.5 - margin - tab_bar_height * 0.5,
                        0.0,
                    ),
                    scale: Vec3::new(window.width() - margin * 2.0, tab_bar_height, 1.0),
                    ..default()
                },
                layers.clone(),
            ));

            // Tab buttons
            let total_tabs_width = tab_names.len() as f32 * (tab_width + tab_gap) - tab_gap;
            let start_x = -total_tabs_width * 0.5 + tab_width * 0.5;

            for (i, name) in tab_names.iter().enumerate() {
                let is_active = i == active_tab;
                let tab_x = start_x + i as f32 * (tab_width + tab_gap);
                let tab_y = window.height() * 0.5 - margin - tab_bar_height * 0.5;

                // Tab button background
                let tab_color = if is_active {
                    Color::srgba(0.3, 0.5, 0.8, 1.0)
                } else {
                    Color::srgba(0.25, 0.25, 0.3, 0.9)
                };
                let tab_mat = materials.add(ColorMaterial::from(tab_color));

                parent.spawn((
                    TabButton { index: i },
                    Mesh2d(unit.quad.clone()),
                    MeshMaterial2d(tab_mat),
                    Transform {
                        translation: Vec3::new(tab_x, tab_y, 1.0),
                        scale: Vec3::new(tab_width, tab_bar_height - 6.0, 1.0),
                        ..default()
                    },
                    layers.clone(),
                ));

                // Tab label
                parent.spawn((
                    Text2d::new(*name),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(if is_active {
                        Color::srgba(1.0, 1.0, 1.0, 1.0)
                    } else {
                        Color::srgba(0.7, 0.7, 0.7, 0.9)
                    }),
                    Transform::from_translation(Vec3::new(tab_x, tab_y, 2.0)),
                    layers.clone(),
                ));
            }
        });
}

/// Handle tab button clicks
pub fn handle_tab_clicks(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut dash: ResMut<DashboardRes>,
    tab_buttons: Query<(&TabButton, &Transform)>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    if !dash.0.has_tabs() {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // Convert to world coordinates (centered origin)
    let cursor_world = Vec2::new(
        cursor.x - window.width() * 0.5,
        window.height() * 0.5 - cursor.y,
    );

    let tab_width = 100.0;
    let tab_height = 29.0;

    for (tab_btn, transform) in tab_buttons.iter() {
        let tab_pos = transform.translation.truncate();
        let half_size = Vec2::new(tab_width * 0.5, tab_height * 0.5);

        if cursor_world.x >= tab_pos.x - half_size.x
            && cursor_world.x <= tab_pos.x + half_size.x
            && cursor_world.y >= tab_pos.y - half_size.y
            && cursor_world.y <= tab_pos.y + half_size.y
        {
            if dash.0.active_tab != tab_btn.index {
                dash.0.active_tab = tab_btn.index;
            }
            break;
        }
    }
}

/// Detect tab changes and refresh tiles
pub fn detect_tab_change(
    mut commands: Commands,
    dash: Res<DashboardRes>,
    mut prev_tab: ResMut<PreviousActiveTab>,
    mut registry: ResMut<TileRegistry>,
    existing: Query<(Entity, &PlotTile)>,
) {
    if !dash.0.has_tabs() {
        return;
    }

    if dash.0.active_tab != prev_tab.0 {
        prev_tab.0 = dash.0.active_tab;

        // Clear all existing tiles to force respawn
        for (entity, tile) in existing.iter() {
            cleanup_tile(&mut commands, &mut registry, entity, tile.id);
        }
    }
}
