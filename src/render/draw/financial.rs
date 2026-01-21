//! Financial chart visualizations: candlestick (OHLC) charts.

#![allow(clippy::too_many_arguments)]

use super::common::{draw_tile_border, format_tick, nice_step, world_to_data};
use crate::render::{TileRect, TileView, UnitMeshes};
use bevy::prelude::*;
use bevy_camera::visibility::RenderLayers;

/// Draw a candlestick chart (OHLC) with zoom/pan support.
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

    // Transform data to world coordinates using view
    let data_to_world_candle =
        |data: Vec2| -> Vec2 { rect.world_center + view.offset + data * view.scale };

    // Calculate candle width
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

    // Compute visible data range for culling
    let half_size = rect.world_size * 0.5;
    let visible_min = world_to_data(rect.world_center - half_size, rect, view);
    let visible_max = world_to_data(rect.world_center + half_size, rect, view);
    let candle_half_width = candle_data_width * 0.5;

    // Draw candles
    for c in &candle.candles {
        // Skip candles outside visible range
        if c.x + candle_half_width < visible_min.x || c.x - candle_half_width > visible_max.x {
            continue;
        }
        if c.high < visible_min.y || c.low > visible_max.y {
            continue;
        }

        let is_up = c.close >= c.open;
        let mat = if is_up {
            up_mat.clone()
        } else {
            down_mat.clone()
        };

        let body_top_data = c.open.max(c.close);
        let body_bottom_data = c.open.min(c.close);

        let pos_top = data_to_world_candle(Vec2::new(c.x, body_top_data));
        let pos_bottom = data_to_world_candle(Vec2::new(c.x, body_bottom_data));
        let pos_high = data_to_world_candle(Vec2::new(c.x, c.high));
        let pos_low = data_to_world_candle(Vec2::new(c.x, c.low));

        let cx = pos_top.x;
        let body_height = (pos_top.y - pos_bottom.y).max(1.0);
        let body_center_y = (pos_top.y + pos_bottom.y) * 0.5;

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
                layers.clone(),
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
                    layers.clone(),
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
                    layers.clone(),
                ));
            }
        });
    }

    // Y-axis ticks
    let y_range_visible = visible_max.y - visible_min.y;
    let y_step = nice_step(y_range_visible, 6);
    let start_y = (visible_min.y / y_step).floor() as i32;
    let end_y = (visible_max.y / y_step).ceil() as i32;

    for i in start_y..=end_y {
        let y_data = i as f32 * y_step;
        let y_world = data_to_world_candle(Vec2::new(0.0, y_data)).y;

        if y_world < rect.world_center.y - half_size.y + 20.0
            || y_world > rect.world_center.y + half_size.y - 10.0
        {
            continue;
        }

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
                layers.clone(),
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

        if x_world < rect.world_center.x - half_size.x + 50.0
            || x_world > rect.world_center.x + half_size.x - 20.0
        {
            continue;
        }

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
                layers.clone(),
            ));
        });
    }

    // Axis labels
    let x_label_text = candle.x_label.as_deref().unwrap_or("Time");
    let y_label_text = candle.y_label.as_deref().unwrap_or("Price");

    commands.entity(root).with_children(|parent| {
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
                    rect.world_center.x - half_size.x + 15.0,
                    rect.world_center.y,
                    2.0,
                ),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            layers,
        ));
    });
}
