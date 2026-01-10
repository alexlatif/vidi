use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Indices, PrimitiveTopology};

use super::layout::{FRAME_THICKNESS, Tile};

pub fn create_rect_mesh(width: f32, height: f32) -> Mesh {
    let hw = width * 0.5;
    let hh = height * 0.5;

    let positions = vec![
        [-hw, -hh, 0.0],
        [hw, -hh, 0.0],
        [-hw, hh, 0.0],
        [hw, hh, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
    let indices = Indices::U32(vec![0, 2, 1, 1, 2, 3]);

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(indices)
}

fn clip_segment(mut a: Vec2, mut b: Vec2, min: Vec2, max: Vec2) -> Option<(Vec2, Vec2)> {
    const INSIDE: u8 = 0;
    const LEFT: u8 = 1;
    const RIGHT: u8 = 2;
    const BOTTOM: u8 = 4;
    const TOP: u8 = 8;

    let outcode = |p: Vec2| -> u8 {
        let mut c = INSIDE;
        if p.x < min.x {
            c |= LEFT;
        } else if p.x > max.x {
            c |= RIGHT;
        }
        if p.y < min.y {
            c |= BOTTOM;
        } else if p.y > max.y {
            c |= TOP;
        }
        c
    };

    let mut code_a = outcode(a);
    let mut code_b = outcode(b);

    loop {
        if (code_a | code_b) == 0 {
            return Some((a, b));
        }
        if (code_a & code_b) != 0 {
            return None;
        }

        let code_out = if code_a != 0 { code_a } else { code_b };

        let (x, y) = if (code_out & TOP) != 0 {
            let x = a.x + (b.x - a.x) * (max.y - a.y) / (b.y - a.y);
            (x, max.y)
        } else if (code_out & BOTTOM) != 0 {
            let x = a.x + (b.x - a.x) * (min.y - a.y) / (b.y - a.y);
            (x, min.y)
        } else if (code_out & RIGHT) != 0 {
            let y = a.y + (b.y - a.y) * (max.x - a.x) / (b.x - a.x);
            (max.x, y)
        } else {
            let y = a.y + (b.y - a.y) * (min.x - a.x) / (b.x - a.x);
            (min.x, y)
        };

        if code_out == code_a {
            a = Vec2::new(x, y);
            code_a = outcode(a);
        } else {
            b = Vec2::new(x, y);
            code_b = outcode(b);
        }
    }
}

pub fn create_line_mesh_clipped(points: &[Vec2], width: f32, tile: &Tile) -> Mesh {
    let half_w = width * 0.5;
    let mut positions: Vec<[f32; 3]> = Vec::new();

    // Clip to *content* rect so plots respect inner bounds
    let clip_min = tile.content_min;
    let clip_max = tile.content_max;

    for seg in points.windows(2) {
        let p0 = seg[0];
        let p1 = seg[1];

        let Some((c0, c1)) = clip_segment(p0, p1, clip_min, clip_max) else {
            continue;
        };

        let dir = c1 - c0;
        let len = dir.length();
        if len < 0.001 {
            continue;
        }

        let normal = Vec2::new(-dir.y, dir.x) / len * half_w;

        let v0 = c0 + normal;
        let v1 = c1 + normal;
        let v2 = c0 - normal;
        let v3 = c1 - normal;

        positions.extend_from_slice(&[
            [v0.x, v0.y, 0.0],
            [v1.x, v1.y, 0.0],
            [v2.x, v2.y, 0.0],
            [v2.x, v2.y, 0.0],
            [v1.x, v1.y, 0.0],
            [v3.x, v3.y, 0.0],
        ]);
    }

    let vertex_count = positions.len();
    let normals = vec![[0.0, 0.0, 1.0]; vertex_count];

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}

pub fn create_surface_mesh(vertices: &[Vec3], grid: UVec2) -> Mesh {
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
