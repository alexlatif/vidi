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
    // Generate plot IDs
    let plot_ids: Vec<PlotId> = dash
        .0
        .plots
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
    for (i, plot) in dash.0.plots.iter().enumerate() {
        let id = PlotId(i as u64);

        if !registry.by_plot.contains_key(&id) {
            let tile = spawn_tile(&mut commands, id, i, plot);
            registry.by_plot.insert(id, tile);
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
        _ => PlotKind::Placeholder,
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

    let n = dash.0.plots.len();
    if n == 0 {
        return;
    }

    let (cols, rows) = grid_dims(n, window.width() / window.height());

    let margin = 20.0;
    let gap = 10.0;

    let avail_w = window.width() - 2.0 * margin;
    let avail_h = window.height() - 2.0 * margin;

    let tile_w = (avail_w - (cols - 1) as f32 * gap) / cols as f32;
    let tile_h = (avail_h - (rows - 1) as f32 * gap) / rows as f32;

    for (tile, mut rect) in tiles.iter_mut() {
        let col = tile.index % cols;
        let row = tile.index / cols;

        // Viewport in physical pixels (CRITICAL FIX)
        let vp_x = margin + col as f32 * (tile_w + gap);
        let vp_y = margin + row as f32 * (tile_h + gap);

        let scale = window.resolution.scale_factor() as f32;
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
    tiles: Query<(Entity, &PlotTile, &TileRect, &PlotKind)>,
    existing: Query<Entity, With<TileCamera>>,
) {
    let mut used = HashSet::new();

    for (_tile_entity, tile, rect, kind) in tiles.iter() {
        // One layer per tile index (0..31). This is a hard RenderLayers limitation.
        let layer = (tile.index % 32) as u8;
        let layers = RenderLayers::layer(layer.into());

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
                    Camera2d::default(),
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
                commands.entity(cam_entity).insert((
                    Camera3d::default(),
                    Camera {
                        viewport: Some(rect.viewport.clone()),
                        order: 10 + tile.index as isize,
                        ..default()
                    },
                    Projection::from(PerspectiveProjection::default()),
                    Transform::from_translation(rect.world_center.extend(50.0))
                        .looking_at(rect.world_center.extend(0.0), Vec3::Y),
                    layers,
                ));
            }
            PlotKind::Placeholder => {
                // Keep camera if you want placeholders rendered; otherwise you can skip.
                let mut ortho = OrthographicProjection::default_2d();
                ortho.scaling_mode = ScalingMode::FixedVertical {
                    viewport_height: rect.world_size.y,
                };

                commands.entity(cam_entity).insert((
                    Camera2d::default(),
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

/// Handle user input
pub fn handle_input(
    mut tiles: Query<(&PlotTile, &mut TileView)>,
    mut registry: ResMut<TileRegistry>,
    hovered: Res<HoveredTile>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut motion: MessageReader<MouseMotion>,
) {
    let Some(hovered_index) = hovered.0 else { return };

    // Collect events first (they can only be read once)
    let mut zoom_delta = 0.0;
    for event in wheel.read() {
        zoom_delta += event.y;
    }

    let mut pan_delta = Vec2::ZERO;
    if mouse.pressed(MouseButton::Left) {
        for event in motion.read() {
            pan_delta += event.delta;
        }
    }

    // Only modify the hovered tile
    for (tile, mut view) in tiles.iter_mut() {
        if tile.index != hovered_index {
            continue;
        }

        let mut changed = false;

        // Zoom with reduced sensitivity and limits
        if zoom_delta != 0.0 {
            view.scale *= 1.0 + zoom_delta * 0.05; // Reduced from 0.1
            view.scale = view.scale.clamp(view.min_scale, view.max_scale);
            changed = true;
        }

        // Pan (offset is in world coordinates, so don't divide by scale)
        if pan_delta != Vec2::ZERO {
            view.offset.x += pan_delta.x;
            view.offset.y -= pan_delta.y;
            changed = true;
        }

        if changed {
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
        let Some(plot) = dash.0.plots.get(tile.index) else {
            continue;
        };

        let crate::core::Plot::Graph2D(graph) = plot else {
            // Mark as fitted even if not 2D
            commands.entity(entity).insert(AutoFitted);
            continue;
        };

        // Compute data bounds from all layers
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

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

        // Skip if no valid bounds
        if !min_x.is_finite() || !max_x.is_finite() || !min_y.is_finite() || !max_y.is_finite() {
            commands.entity(entity).insert(AutoFitted);
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
        view.min_scale = fit_scale * 0.5;  // Can zoom out to 50% of fit
        view.max_scale = fit_scale * 4.0;  // Can zoom in to 4x of fit

        // Mark as fitted and dirty
        commands.entity(entity).insert(AutoFitted);
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
        if let Some(plot) = dash.0.plots.get(tile.index) {
            let layer = RenderLayers::layer(tile.index % 32);
            match plot {
                crate::core::Plot::Graph2D(graph) => {
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
                crate::core::Plot::Distribution(dist) => {
                    match dist {
                        crate::core::Distribution::Histogram { values, bins, style, x_label, y_label } => {
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
                        crate::core::Distribution::Pdf { values, style, x_label, y_label } => {
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
                    }
                }
                _ => {
                    draw_placeholder(&mut commands, root, rect, &unit, &mut materials, layer);
                }
            }
        }
    }
}

// Utility functions for grid layout
fn grid_dims(n: usize, aspect: f32) -> (usize, usize) {
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
            let rows = (n + cols - 1) / cols;
            (cols, rows)
        }
    }
}

fn cleanup_tile(commands: &mut Commands, registry: &mut TileRegistry, entity: Entity, id: PlotId) {
    commands.entity(entity).despawn();
    registry.by_plot.remove(&id);
    registry.camera_of.remove(&id);
}

/// Convert world coordinates back to data coordinates
fn world_to_data(world: Vec2, rect: &TileRect, view: &TileView) -> Vec2 {
    (world - rect.world_center - view.offset) / view.scale
}

/// Convert data coordinates to world coordinates
fn data_to_world_sys(data: Vec2, rect: &TileRect, view: &TileView) -> Vec2 {
    rect.world_center + view.offset + data * view.scale
}

/// Find nearest data point on any trace in the graph
fn find_nearest_point(
    cursor_data: Vec2,
    graph: &crate::core::Graph2D,
) -> Option<Vec2> {
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
    tiles: Query<(&PlotTile, &TileRect, &TileView)>,
    hovered: Res<HoveredTile>,
    dash: Res<DashboardRes>,
    mut cursor_pos: ResMut<CursorWorldPos>,
    crosshairs: Query<(Entity, &Crosshair)>,
    unit: Res<UnitMeshes>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    // Despawn all existing crosshairs first (we recreate each frame for simplicity)
    for (entity, _) in crosshairs.iter() {
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

    for (tile, rect, view) in tiles.iter() {
        if tile.index != hovered_index {
            continue;
        }

        // Get the graph data for this tile
        let Some(plot) = dash.0.plots.get(tile.index) else {
            continue;
        };

        match plot {
            crate::core::Plot::Graph2D(graph) => {
                // Convert cursor to data coordinates
                let cursor_data = world_to_data(cursor_world, rect, view);

                // Find nearest data point
                let snap_data = find_nearest_point(cursor_data, graph).unwrap_or(cursor_data);
                let snap_world = data_to_world_sys(snap_data, rect, view);

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
    let bottom_y = rect.world_center.y - rect.world_size.y * 0.5 + rect.world_size.y * padding_bottom;
    let right_x = left_x + usable_width;
    let top_y = bottom_y + usable_height;

    // Check if cursor is within the plot area
    if cursor_world.x < left_x || cursor_world.x > right_x
        || cursor_world.y < bottom_y || cursor_world.y > top_y
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
            let count = values.iter().filter(|&&v| {
                if bar_index == *bins - 1 {
                    v >= bin_start && v <= bin_end
                } else {
                    v >= bin_start && v < bin_end
                }
            }).count();

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

            let density: f32 = values.iter().map(|&xi| {
                let u = (data_x - xi) / bandwidth;
                (-0.5 * u * u).exp() / (2.506628 * bandwidth)
            }).sum::<f32>() / n;

            // Compute max density for normalization
            let max_density = {
                let mut max_d = 0.0f32;
                for i in 0..100 {
                    let x = x_min + (i as f32 / 99.0) * (x_max - x_min);
                    let d: f32 = values.iter().map(|&xi| {
                        let u = (x - xi) / bandwidth;
                        (-0.5 * u * u).exp() / (2.506628 * bandwidth)
                    }).sum::<f32>() / n;
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
                    let point_mat = materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.95)));
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
    }
}
