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

#[derive(Component, Clone, Copy, Default)]
pub struct TileView {
    pub offset: Vec2,
    pub scale: f32,
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
