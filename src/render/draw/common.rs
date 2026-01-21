//! Common drawing utilities shared across chart types.

use crate::render::{TileRect, TileView, UnitMeshes};
use bevy::prelude::*;
use bevy_camera::visibility::RenderLayers;

/// Draw a border around a tile rect.
pub fn draw_tile_border(
    commands: &mut Commands,
    root: Entity,
    rect: &TileRect,
    unit: &UnitMeshes,
    materials: &mut Assets<ColorMaterial>,
    layers: RenderLayers,
    color: Color,
    z: f32,
) {
    let border_mat = materials.add(ColorMaterial::from(color));
    let border_thickness = 2.0;

    commands.entity(root).with_children(|parent| {
        for (dx, dy) in [(0.0, 0.5), (0.0, -0.5), (-0.5, 0.0), (0.5, 0.0)] {
            parent.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(border_mat.clone()),
                Transform {
                    translation: Vec3::new(
                        rect.world_center.x + dx * rect.world_size.x,
                        rect.world_center.y + dy * rect.world_size.y,
                        z,
                    ),
                    scale: if dx == 0.0 {
                        Vec3::new(rect.world_size.x, border_thickness, 1.0)
                    } else {
                        Vec3::new(border_thickness, rect.world_size.y, 1.0)
                    },
                    ..default()
                },
                layers.clone(),
            ));
        }
    });
}

/// Draw title and description for a plot.
/// Returns the height used by title area (for adjusting plot content).
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
                Transform::from_translation(Vec3::new(rect.world_center.x, title_y, 3.0)),
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
                Transform::from_translation(Vec3::new(rect.world_center.x, desc_y, 3.0)),
                layers,
            ));
            title_height += 14.0;
        }
    });

    title_height
}

/// Draw placeholder for unimplemented plot types.
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

/// Convert data coordinates to world coordinates.
pub fn data_to_world(data: Vec2, rect: &TileRect, view: &TileView) -> Vec2 {
    rect.world_center + view.offset + data * view.scale
}

/// Convert world coordinates to data coordinates.
pub fn world_to_data(world: Vec2, rect: &TileRect, view: &TileView) -> Vec2 {
    (world - rect.world_center - view.offset) / view.scale
}

/// Calculate nice tick step for given range.
pub fn nice_step(range: f32, target_ticks: usize) -> f32 {
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

/// Format tick value for display.
pub fn format_tick(val: f32) -> String {
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

/// Draw axis tick marks and value labels.
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
