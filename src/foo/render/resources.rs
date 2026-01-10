use bevy::prelude::*;

use crate::core::Dashboard;

#[derive(Resource, Clone, Debug)]
pub struct DashboardRes(pub Dashboard);

impl DashboardRes {
    pub fn new(d: Dashboard) -> Self {
        Self(d)
    }
}

#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct HoveredTile(pub Option<usize>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View2D {
    pub offset: Vec2,
    pub scale: f32,
}

impl Default for View2D {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 1.0,
        }
    }
}

impl View2D {
    pub fn fit_bounds(&mut self, bounds: ([f32; 2], [f32; 2]), tile_size: Vec2) {
        // Ensure min/max are in correct order
        let x_min = bounds.0[0].min(bounds.0[1]);
        let x_max = bounds.0[0].max(bounds.0[1]);
        let y_min = bounds.1[0].min(bounds.1[1]);
        let y_max = bounds.1[0].max(bounds.1[1]);

        let data_width = x_max - x_min;
        let data_height = y_max - y_min;

        if data_width <= 0.0 || data_height <= 0.0 {
            return;
        }

        // Leave small margin for borders (frame thickness is 1px on each side)
        let margin = 10.0; // pixels
        let available_width = (tile_size.x - 2.0 * margin).max(1.0);
        let available_height = (tile_size.y - 2.0 * margin).max(1.0);

        let scale_x = available_width / data_width;
        let scale_y = available_height / data_height;

        self.scale = scale_x.min(scale_y);

        // Calculate the data center
        let data_center_x = (x_min + x_max) * 0.5;
        let data_center_y = (y_min + y_max) * 0.5;

        // offset should move the scaled data center to (0, 0)
        self.offset = Vec2::new(-data_center_x * self.scale, -data_center_y * self.scale);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct View3D {
    pub target: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub pan_speed: f32,
    pub orbit_speed: f32,
}

impl Default for View3D {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 8.0,
            yaw: 0.9,
            pitch: -0.5,
            pan_speed: 0.005,
            orbit_speed: 0.01,
        }
    }
}

#[derive(Resource, Default, Clone, Debug)]
pub struct TileViews {
    pub v2: Vec<View2D>,
    pub v3: Vec<View3D>,
}

impl TileViews {
    pub fn ensure_len(&mut self, n: usize) {
        if self.v2.len() < n {
            self.v2.resize(n, View2D::default());
        }
        if self.v3.len() < n {
            self.v3.resize(n, View3D::default());
        }
    }
}

pub fn setup_global_scene(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        brightness: 300.0,
        ..default()
    });
}
