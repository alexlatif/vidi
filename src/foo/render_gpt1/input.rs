use bevy::prelude::*;

use crate::render_gpt1::layout::TileRect;
use crate::render_gpt1::tile::{HoveredPlot, PlotKind, PlotTile, TileRegistry, View2D, View3D};

pub fn update_hover(
    windows: Query<&Window>,
    tiles: Query<(&PlotTile, &TileRect)>,
    mut hovered: ResMut<HoveredPlot>,
) {
    let Ok(win) = windows.get_single() else {
        hovered.0 = None;
        return;
    };
    let Some(p) = win.cursor_position() else {
        hovered.0 = None;
        return;
    };

    for (t, r) in tiles.iter() {
        if p.x >= r.vp_min.x && p.x <= r.vp_max.x && p.y >= r.vp_min.y && p.y <= r.vp_max.y {
            hovered.0 = Some(t.id);
            return;
        }
    }
    hovered.0 = None;
}

pub fn handle_input(
    hovered: Res<HoveredPlot>,
    reg: Res<TileRegistry>,
    mut q2: Query<(&PlotTile, &mut View2D)>,
    mut q3: Query<(&PlotTile, &mut View3D)>,
    mut wheel: EventReader<bevy::input::mouse::MouseWheel>,
    mut motion: EventReader<bevy::input::mouse::MouseMotion>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Some(id) = hovered.0 else {
        wheel.clear();
        motion.clear();
        return;
    };
    let Some(&tile_e) = reg.tile_of.get(&id) else {
        wheel.clear();
        motion.clear();
        return;
    };

    let mut scroll = 0.0;
    for e in wheel.read() {
        scroll += e.y;
    }

    let mut delta = Vec2::ZERO;
    for e in motion.read() {
        delta += e.delta;
    }

    if let Ok((tile, mut v)) = q2.get_mut(tile_e) {
        if matches!(tile.kind, PlotKind::TwoD | PlotKind::Placeholder) {
            if scroll != 0.0 {
                v.scale = (v.scale * (1.0 + 0.10 * scroll)).clamp(0.001, 200_000.0);
            }
            if mouse.pressed(MouseButton::Left) {
                v.offset_px.x += delta.x;
                v.offset_px.y -= delta.y;
            }
        }
        return;
    }

    if let Ok((_tile, mut v)) = q3.get_mut(tile_e) {
        if scroll != 0.0 {
            v.radius = (v.radius * (1.0 - 0.10 * scroll)).clamp(0.5, 5000.0);
        }
        if mouse.pressed(MouseButton::Left) {
            v.yaw -= delta.x * 0.01;
            v.pitch = (v.pitch - delta.y * 0.01).clamp(-1.5, 1.5);
        }
    }
}
