use super::components::PlotId;
use bevy::prelude::*;
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
    pub dirty: VecDeque<PlotId>,
}

#[derive(Resource, Default)]
pub struct HoveredTile(pub Option<usize>);

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
}

pub fn setup_unit_meshes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let quad = meshes.add(Mesh::from(Rectangle::new(1.0, 1.0)));
    let sphere = meshes.add(Mesh::from(Sphere::new(0.5)));
    commands.insert_resource(UnitMeshes { quad, sphere });
}
