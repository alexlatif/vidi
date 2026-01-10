use bevy::prelude::*;
use bevy_camera::Viewport;

pub const OUTER_MARGIN: f32 = 2.0;
pub const TILE_GAP: f32 = 2.0;
pub const FRAME_THICKNESS: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub idx: usize,
    pub vp_min: Vec2,
    pub vp_max: Vec2,
    pub world_min: Vec2,
    pub world_max: Vec2,
    pub world_center: Vec2,
    pub world_size: Vec2,
    pub content_min: Vec2,
    pub content_max: Vec2,
    pub content_center: Vec2,
    pub content_size: Vec2,
}

fn grid_dims(n: usize) -> (usize, usize) {
    match n {
        0 => (0, 0),
        1 => (1, 1),
        2 => (2, 1),
        _ => {
            let cols = (n as f32).sqrt().ceil().max(1.0) as usize;
            let rows = ((n + cols - 1) / cols).max(1);
            (cols, rows)
        }
    }
}

pub fn compute_tiles(window: &Window, n: usize) -> Vec<Tile> {
    if n == 0 {
        return Vec::new();
    }

    let (cols, rows) = grid_dims(n);
    let w = window.width();
    let h = window.height();

    let outer = OUTER_MARGIN.min(w * 0.05).min(h * 0.05);
    let gap = TILE_GAP.min(w * 0.02).min(h * 0.02);

    let usable_w = (w - 2.0 * outer).max(1.0);
    let usable_h = (h - 2.0 * outer).max(1.0);

    let gap_x = gap * (cols.saturating_sub(1)) as f32;
    let gap_y = gap * (rows.saturating_sub(1)) as f32;

    let tile_w = ((usable_w - gap_x) / cols as f32).max(1.0);
    let tile_h = ((usable_h - gap_y) / rows as f32).max(1.0);

    let mut tiles = Vec::with_capacity(n);

    for i in 0..n {
        let col = i % cols;
        let row = i / cols;

        let vp_x0 = outer + col as f32 * (tile_w + gap);
        let vp_y0 = outer + row as f32 * (tile_h + gap);
        let vp_x1 = vp_x0 + tile_w;
        let vp_y1 = vp_y0 + tile_h;

        let vp_min = Vec2::new(vp_x0, vp_y0);
        let vp_max = Vec2::new(vp_x1, vp_y1);

        // 1:1 pixel to world mapping
        let world_min = Vec2::new(vp_min.x - w * 0.5, h * 0.5 - vp_max.y);
        let world_max = Vec2::new(vp_max.x - w * 0.5, h * 0.5 - vp_min.y);
        let world_center = (world_min + world_max) * 0.5;
        let world_size = (world_max - world_min).max(Vec2::ONE);

        // Content area is world area minus frame
        let inset = FRAME_THICKNESS;
        let content_min = world_min + Vec2::splat(inset);
        let content_max = world_max - Vec2::splat(inset);
        let content_center = (content_min + content_max) * 0.5;
        let content_size = (content_max - content_min).max(Vec2::ONE);

        tiles.push(Tile {
            idx: i,
            vp_min,
            vp_max,
            world_min,
            world_max,
            world_center,
            world_size,
            content_min,
            content_max,
            content_center,
            content_size,
        });
    }

    tiles
}

pub fn tile_to_viewport(window: &Window, tile: &Tile) -> Viewport {
    let sf = window.scale_factor() as f32;
    let phys_pos = (tile.vp_min * sf).round().as_uvec2();
    let phys_size = ((tile.vp_max - tile.vp_min) * sf)
        .round()
        .max(Vec2::ONE)
        .as_uvec2();

    Viewport {
        physical_position: phys_pos,
        physical_size: phys_size,
        depth: 0.0..1.0,
    }
}
