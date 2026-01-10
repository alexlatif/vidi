use bevy::prelude::*;
use bevy_camera::Viewport;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Component, Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct PlotId(pub u64);

impl PlotId {
    pub fn new() -> Self {
        static CTR: AtomicU32 = AtomicU32::new(1);
        Self(CTR.fetch_add(1, Ordering::Relaxed).into())
    }
}

#[derive(Component)]
pub struct PlotTile {
    pub id: PlotId,
    pub index: usize,
    pub kind: PlotKind,
}

#[derive(Component, Clone, Copy)]
pub enum PlotKind {
    TwoD,
    ThreeD,
    Placeholder,
}

#[derive(Component, Clone, Copy)]
pub struct TileView {
    pub offset: Vec2,
    pub scale: f32,
    pub min_scale: f32,
    pub max_scale: f32,
}

impl Default for TileView {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 1.0,
            min_scale: 0.1,
            max_scale: 100.0,
        }
    }
}

#[derive(Component)]
pub struct TileRect {
    pub world_center: Vec2,
    pub world_size: Vec2,
    pub content: Rect,
    pub viewport: Viewport,
}

#[derive(Component)]
pub struct TileRenderRoot;

#[derive(Component)]
pub struct TileCamera;

/// Marker for crosshair parent entity
#[derive(Component)]
pub struct Crosshair {
    pub tile_index: usize,
}

/// Marker for crosshair horizontal line
#[derive(Component)]
pub struct CrosshairHLine;

/// Marker for crosshair vertical line
#[derive(Component)]
pub struct CrosshairVLine;

/// Marker for coordinate text display
#[derive(Component)]
pub struct CrosshairCoordText;

/// Marker to track if a tile has been auto-fitted to its data
#[derive(Component)]
pub struct AutoFitted;
