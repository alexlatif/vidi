use bevy::prelude::*;
use bevy_camera::Viewport;

use crate::render_gpt1::tile::{PlotTile, TileRegistry};

#[derive(Component, Clone, Copy, Debug)]
pub struct TileRect {
    pub vp_min: Vec2,
    pub vp_max: Vec2,
    pub viewport: Viewport, // physical pixels

    pub world_center: Vec2,
    pub world_size: Vec2,

    pub content_min: Vec2,
    pub content_max: Vec2,
    pub content_center: Vec2,
}

const OUTER: f32 = 20.0;
const GAP: f32 = 12.0;
const PAD: f32 = 12.0;

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
            let cols = (n as f32).sqrt().ceil().max(1.0) as usize;
            let rows = (n + cols - 1) / cols;
            (cols, rows.max(1))
        }
    }
}

pub fn compute_layout(
    mut commands: Commands,
    windows: Query<&Window>,
    reg: Res<TileRegistry>,
    tiles: Query<(Entity, &PlotTile)>,
) {
    let Ok(win) = windows.get_single() else {
        return;
    };
    let w = win.width();
    let h = win.height();
    let sf = win.resolution.scale_factor() as f32;

    let n = reg.order.len();
    if n == 0 {
        return;
    }

    let aspect = (w / h).max(0.01);
    let (cols, rows) = grid_dims(n, aspect);

    let usable_w = (w - 2.0 * OUTER).max(1.0);
    let usable_h = (h - 2.0 * OUTER).max(1.0);

    let tile_w = (usable_w - GAP * (cols.saturating_sub(1)) as f32).max(1.0) / cols as f32;
    let tile_h = (usable_h - GAP * (rows.saturating_sub(1)) as f32).max(1.0) / rows as f32;

    for (ord, id) in reg.order.iter().copied().enumerate() {
        let col = ord % cols;
        let row = ord / cols;

        let x0 = OUTER + col as f32 * (tile_w + GAP);
        let y0 = OUTER + row as f32 * (tile_h + GAP);
        let x1 = x0 + tile_w;
        let y1 = y0 + tile_h;

        let vp_min = Vec2::new(x0, y0);
        let vp_max = Vec2::new(x1, y1);

        let pos_px = (vp_min * sf).round().max(Vec2::ZERO).as_uvec2();
        let size_px = ((vp_max - vp_min) * sf).round().max(Vec2::ONE).as_uvec2();

        let world_center = Vec2::new((x0 + x1) * 0.5 - w * 0.5, h * 0.5 - (y0 + y1) * 0.5);
        let world_size = Vec2::new(tile_w, tile_h);

        let content_size = (world_size - Vec2::splat(2.0 * PAD)).max(Vec2::ONE);
        let content_center = world_center;
        let content_min = content_center - content_size * 0.5;
        let content_max = content_center + content_size * 0.5;

        let viewport = Viewport {
            physical_position: pos_px,
            physical_size: size_px,
            depth: 0.0..1.0,
        };

        // Write to matching tile entity
        for (e, t) in tiles.iter() {
            if t.id == id {
                commands.entity(e).insert(TileRect {
                    vp_min,
                    vp_max,
                    viewport,
                    world_center,
                    world_size,
                    content_min,
                    content_max,
                    content_center,
                });
                break;
            }
        }
    }
}
