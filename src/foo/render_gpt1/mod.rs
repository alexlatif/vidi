use bevy::prelude::*;

pub mod camera;
pub mod draw;
pub mod input;
pub mod layout;
pub mod tile;

pub use resources::DashboardRes;
pub use tile::{DashRenderPlugin, DashboardRes, HoveredPlot};
