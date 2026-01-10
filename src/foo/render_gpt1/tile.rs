use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Clone, Debug)]
pub struct DashboardRes(pub crate::core::Dashboard);

#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct HoveredPlot(pub Option<PlotId>);

/// Must be stable across rebuilds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlotId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlotKind {
    TwoD,
    ThreeD,
    Placeholder,
}

#[derive(Component)]
pub struct PlotTile {
    pub id: PlotId,
    pub index: usize, // current dashboard order index
    pub kind: PlotKind,
}

#[derive(Component)]
pub struct RenderRoot;

#[derive(Component)]
pub struct View2D {
    pub offset_px: Vec2,
    pub scale: f32,
}
impl Default for View2D {
    fn default() -> Self {
        Self {
            offset_px: Vec2::ZERO,
            scale: 1.0,
        }
    }
}

#[derive(Component)]
pub struct View3D {
    pub target: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}
impl Default for View3D {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 8.0,
            yaw: 0.9,
            pitch: -0.5,
        }
    }
}

#[derive(Resource, Default)]
pub struct TileRegistry {
    pub tile_of: HashMap<PlotId, Entity>,
    pub root_of: HashMap<PlotId, Entity>,
    pub cam_of: HashMap<PlotId, Entity>,
    pub order: Vec<PlotId>,
}

pub struct DashRenderPlugin;

impl Plugin for DashRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileRegistry>()
            .init_resource::<HoveredPlot>()
            .add_systems(Startup, setup_world)
            .add_systems(
                Update,
                (
                    sync_dashboard_to_tiles,
                    crate::render_gpt1::layout::compute_layout.after(sync_dashboard_to_tiles),
                    crate::render_gpt1::camera::sync_tile_cameras
                        .after(crate::render_gpt1::layout::compute_layout),
                    crate::render_gpt1::input::update_hover.after(crate::render_gpt1::layout::compute_layout),
                    crate::render_gpt1::input::handle_input.after(crate::render_gpt1::input::update_hover),
                    crate::render_gpt1::draw::redraw_changed_tiles
                        .after(crate::render_gpt1::camera::sync_tile_cameras),
                ),
            )
            // unit meshes init in draw module
            .add_systems(Startup, crate::render_gpt1::draw::ensure_unit_meshes);
    }
}

fn setup_world(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        brightness: 300.0,
        ..default()
    });
}

/// ADAPTER: replace with your stable ID (recommended).
fn plot_id(dash: &crate::core::Dashboard, index: usize) -> PlotId {
    // Best: PlotId(dash.plots[index].id())
    // Fallback: hash stable-ish fingerprint
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::mem::discriminant(&dash.plots[index]).hash(&mut h);
    index.hash(&mut h);
    PlotId(h.finish())
}

fn plot_kind(plot: &crate::core::Plot) -> PlotKind {
    use crate::core::Plot::*;
    match plot {
        Graph2D(_) => PlotKind::TwoD,
        Graph3D(_) => PlotKind::ThreeD,
        _ => PlotKind::Placeholder,
    }
}

pub fn sync_dashboard_to_tiles(
    mut commands: Commands,
    dash: Option<Res<DashboardRes>>,
    mut reg: ResMut<TileRegistry>,
    existing: Query<(Entity, &PlotTile)>,
) {
    let Some(dash) = dash else {
        for (e, _) in existing.iter() {
            commands.entity(e).despawn_recursive();
        }
        reg.tile_of.clear();
        reg.root_of.clear();
        reg.cam_of.clear();
        reg.order.clear();
        return;
    };

    // Build desired list
    let mut desired: Vec<(PlotId, usize, PlotKind)> = Vec::with_capacity(dash.0.plots.len());
    for i in 0..dash.0.plots.len() {
        let id = plot_id(&dash.0, i);
        let kind = plot_kind(&dash.0.plots[i]);
        desired.push((id, i, kind));
    }

    // Remove stale tiles
    for (e, t) in existing.iter() {
        if !desired.iter().any(|(id, _, _)| *id == t.id) {
            commands.entity(e).despawn_recursive();
            reg.tile_of.remove(&t.id);
            reg.root_of.remove(&t.id);
            // camera will be removed by camera sync if orphaned, but okay to also remove mapping
            reg.cam_of.remove(&t.id);
        }
    }

    // Create or update tiles
    for (id, index, kind) in desired.iter().copied() {
        let tile_e = if let Some(&e) = reg.tile_of.get(&id) {
            e
        } else {
            let tile_e = commands
                .spawn((
                    Name::new(format!("tile_{:x}", id.0)),
                    PlotTile { id, index, kind },
                ))
                .id();

            let root_e = commands
                .spawn((
                    Name::new(format!("root_{:x}", id.0)),
                    RenderRoot,
                    SpatialBundle::default(),
                ))
                .id();

            commands.entity(tile_e).add_child(root_e);

            reg.tile_of.insert(id, tile_e);
            reg.root_of.insert(id, root_e);

            // insert correct view
            match kind {
                PlotKind::TwoD | PlotKind::Placeholder => {
                    commands.entity(tile_e).insert(View2D::default())
                }
                PlotKind::ThreeD => commands.entity(tile_e).insert(View3D::default()),
            };

            tile_e
        };

        // Keep up to date
        commands.entity(tile_e).insert(PlotTile { id, index, kind });

        // Ensure correct view component
        match kind {
            PlotKind::TwoD | PlotKind::Placeholder => {
                commands.entity(tile_e).remove::<View3D>();
                commands.entity(tile_e).insert(View2D::default());
            }
            PlotKind::ThreeD => {
                commands.entity(tile_e).remove::<View2D>();
                commands.entity(tile_e).insert(View3D::default());
            }
        }
    }

    reg.order = desired.into_iter().map(|(id, _, _)| id).collect();
}
