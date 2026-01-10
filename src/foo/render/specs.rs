use bevy::prelude::*;

use crate::core::{Dashboard, Graph2D, Graph3D, Plot};

use super::layout::compute_tiles;
use super::resources::{DashboardRes, HoveredTile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    TwoD { g2_idx: usize },
    ThreeD { g3_idx: usize },
    Placeholder2D,
}

#[derive(Clone, Copy, Debug)]
pub struct TileSpec {
    pub tile_idx: usize,
    pub kind: TileKind,
}

pub fn build_tile_specs(dash: &Dashboard) -> (Vec<TileSpec>, Vec<&Graph2D>, Vec<&Graph3D>) {
    let mut specs = Vec::with_capacity(dash.plots.len());
    let mut g2_list: Vec<&Graph2D> = Vec::new();
    let mut g3_list: Vec<&Graph3D> = Vec::new();

    for (i, plot) in dash.plots.iter().enumerate() {
        let kind = match plot {
            Plot::Graph2D(g) => {
                let idx = g2_list.len();
                g2_list.push(g);
                TileKind::TwoD { g2_idx: idx }
            }
            Plot::Graph3D(g) => {
                let idx = g3_list.len();
                g3_list.push(g);
                TileKind::ThreeD { g3_idx: idx }
            }
            Plot::Distribution(_) | Plot::Field(_) | Plot::Radial(_) => TileKind::Placeholder2D,
        };
        specs.push(TileSpec { tile_idx: i, kind });
    }

    (specs, g2_list, g3_list)
}

pub fn update_hovered_tile(
    dash: Option<Res<DashboardRes>>,
    windows: Query<&Window>,
    mut hovered: ResMut<HoveredTile>,
) {
    let Some(dash) = dash else {
        hovered.0 = None;
        return;
    };

    let Ok(window) = windows.single() else {
        hovered.0 = None;
        return;
    };

    let (specs, _, _) = build_tile_specs(&dash.0);
    if specs.is_empty() {
        hovered.0 = None;
        return;
    }

    let Some(cursor) = window.cursor_position() else {
        hovered.0 = None;
        return;
    };

    let tiles = compute_tiles(window, specs.len());

    hovered.0 = tiles.iter().position(|t| {
        cursor.x >= t.vp_min.x
            && cursor.x <= t.vp_max.x
            && cursor.y >= t.vp_min.y
            && cursor.y <= t.vp_max.y
    });
}

#[derive(Component)]
pub struct TileCam {
    pub tile_idx: usize,
    pub kind: TileKind,
}

#[derive(Component)]
pub struct TileCam3D;
