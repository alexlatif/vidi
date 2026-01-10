use super::components::PlotId;
use bevy::prelude::*;
use bevy_camera::visibility::RenderLayers;
use std::collections::{HashMap, VecDeque};

#[derive(Resource, Clone)]
pub struct DashboardRes(pub crate::core::Dashboard);

impl DashboardRes {
    pub fn new(dashboard: crate::core::Dashboard) -> Self {
        Self(dashboard)
    }
}

#[derive(Resource, Default)]
pub struct TileRegistry {
    pub by_plot: HashMap<PlotId, Entity>,
    pub camera_of: HashMap<PlotId, Entity>,
    /// Overlay 2D cameras for 3D tiles (used for titles/borders)
    pub overlay_of: HashMap<PlotId, Entity>,
    pub dirty: VecDeque<PlotId>,
}

#[derive(Resource, Default)]
pub struct HoveredTile(pub Option<usize>);

#[derive(Resource, Default)]
pub struct CursorWorldPos {
    /// World position of cursor (if over a tile)
    pub position: Option<Vec2>,
    /// Data coordinates (converted from world coords)
    pub data_coords: Option<Vec2>,
    /// Which tile the cursor is over
    pub tile_index: Option<usize>,
}

#[derive(Resource)]
pub struct UnitMeshes {
    pub quad: Handle<Mesh>,
    pub sphere: Handle<Mesh>,
}

// TODO: why below not in mod
pub fn setup_global_scene(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        brightness: 300.0,
        ..default()
    });

    // Spawn a main UI camera for tab bar and global UI elements (layer 0)
    commands.spawn((
        Camera2d::default(),
        Camera {
            order: 100, // Render after tile cameras (which use order 10+)
            ..default()
        },
        RenderLayers::layer(0),
    ));
}

pub fn setup_unit_meshes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let quad = meshes.add(Mesh::from(Rectangle::new(1.0, 1.0)));
    let sphere = meshes.add(Mesh::from(Sphere::new(0.5)));
    commands.insert_resource(UnitMeshes { quad, sphere });
}

/// Stores 3D scatter point data for tooltip lookup
/// Maps tile_index -> list of (original_coords, normalized_coords)
#[derive(Resource, Default)]
pub struct ScatterPoints3D {
    pub points: HashMap<usize, Vec<(Vec3, Vec3)>>,
}

/// Stores 3D plot axis info for rendering labels and ticks
#[derive(Clone, Default)]
pub struct AxisInfo3D {
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub z_label: Option<String>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

/// Maps tile_index -> axis info for 3D plots
#[derive(Resource, Default)]
pub struct AxisInfo3DStore {
    pub info: HashMap<usize, AxisInfo3D>,
}
