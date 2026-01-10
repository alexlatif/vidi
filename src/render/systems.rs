use super::*;
use crate::render::PlotId;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_camera::visibility::RenderLayers;
use bevy_camera::{
    OrthographicProjection, PerspectiveProjection, Projection, ScalingMode, Viewport,
};
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
            TileView::default(),
            Transform::default(),
        ))
        .id();

    // Create render root child
    let root = commands.spawn((TileRenderRoot, Transform::default())).id();
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

        rect.world_center = world_center;
        rect.world_size = Vec2::new(tile_w, tile_h);
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
    mut tiles: Query<(&mut TileView, &mut Transform), With<PlotKind>>,
    hovered: Res<HoveredTile>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut motion: MessageReader<MouseMotion>,
) {
    let Some(_index) = hovered.0 else { return };

    for (mut view, _) in tiles.iter_mut() {
        // Zoom
        for event in wheel.read() {
            view.scale *= 1.0 + event.y * 0.1;
        }

        // Pan
        if mouse.pressed(MouseButton::Left) {
            let mut delta = Vec2::ZERO;
            for event in motion.read() {
                delta += event.delta;
            }
            view.offset.x += delta.x / view.scale;
            view.offset.y -= delta.y / view.scale;
        }
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
                    commands.entity(child).despawn();
                }
            }
        }

        // 2) Create a fresh render root under the tile
        let root = commands.spawn((TileRenderRoot, Transform::default())).id();
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
                        &mut materials,
                        layer,
                    );
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
