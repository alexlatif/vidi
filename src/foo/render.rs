//! src/render.rs — Bevy 0.17.3 Production Renderer
//!
//! NON-NEGOTIABLES:
//! - 2D and 3D plots are EQUAL peers in the same tile world
//! - Each tile has its own camera with viewport scissoring (hard clip)
//! - Each tile has its own RenderLayers mask (complete isolation)
//! - No visual artifacts or memory of old renders
//! - Proper tile layout that fills the dashboard

use bevy::prelude::*;

// Bevy 0.17.3 correct import paths
use bevy_asset::RenderAssetUsages;
use bevy_camera::visibility::RenderLayers;
use bevy_camera::{ClearColorConfig, Viewport};
use bevy_mesh::{Indices, PrimitiveTopology};

use crate::core::{
    Dashboard, Geometry2D, Geometry3D, Graph2D, Graph3D, Layer2D, Layer3D, Plot, Style,
};

/* ========================================================================== */
/*                              CONFIGURATION                                  */
/* ========================================================================== */

const OUTER_MARGIN: f32 = 8.0;
const TILE_GAP: f32 = 6.0;
const FRAME_THICKNESS: f32 = 1.5;

/* ========================================================================== */
/*                              RESOURCES                                      */
/* ========================================================================== */

#[derive(Resource, Clone, Debug)]
pub struct DashboardRes(pub Dashboard);

impl DashboardRes {
    pub fn new(d: Dashboard) -> Self {
        Self(d)
    }
}

#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct HoveredTile(pub Option<usize>);

#[derive(Clone, Copy, Debug)]
pub struct View2D {
    pub offset: Vec2,
    pub scale: f32,
}

impl Default for View2D {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 80.0,
        }
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
    fn ensure_len(&mut self, n: usize) {
        if self.v2.len() < n {
            self.v2.resize(n, View2D::default());
        }
        if self.v3.len() < n {
            self.v3.resize(n, View3D::default());
        }
    }
}

/* ========================================================================== */
/*                              TILE MODEL                                     */
/* ========================================================================== */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TileKind {
    TwoD { g2_idx: usize },
    ThreeD { g3_idx: usize },
    Placeholder2D,
}

#[derive(Clone, Copy, Debug)]
struct TileSpec {
    tile_idx: usize,
    kind: TileKind,
}

fn build_tile_specs(dash: &Dashboard) -> (Vec<TileSpec>, Vec<&Graph2D>, Vec<&Graph3D>) {
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

#[derive(Clone, Copy, Debug)]
struct Tile {
    idx: usize,
    vp_min: Vec2,
    vp_max: Vec2,
    world_min: Vec2,
    world_max: Vec2,
    world_center: Vec2,
    world_size: Vec2,
}

fn grid_dims(n: usize) -> (usize, usize) {
    if n == 0 {
        return (0, 0);
    }
    let cols = (n as f32).sqrt().ceil().max(1.0) as usize;
    let rows = ((n + cols - 1) / cols).max(1);
    (cols, rows)
}

fn compute_tiles(window: &Window, n: usize) -> Vec<Tile> {
    if n == 0 {
        return Vec::new();
    }

    let (cols, rows) = grid_dims(n);
    let w = window.width();
    let h = window.height();

    let usable_x = w - 2.0 * OUTER_MARGIN;
    let usable_y = h - 2.0 * OUTER_MARGIN;

    let gap_x = TILE_GAP * (cols.saturating_sub(1)) as f32;
    let gap_y = TILE_GAP * (rows.saturating_sub(1)) as f32;

    let tile_w = (usable_x - gap_x) / cols as f32;
    let tile_h = (usable_y - gap_y) / rows as f32;

    let mut tiles = Vec::with_capacity(n);

    for i in 0..n {
        let col = i % cols;
        let row = i / cols;

        let vp_x0 = OUTER_MARGIN + col as f32 * (tile_w + TILE_GAP);
        let vp_y0 = OUTER_MARGIN + row as f32 * (tile_h + TILE_GAP);
        let vp_x1 = vp_x0 + tile_w;
        let vp_y1 = vp_y0 + tile_h;

        let vp_min = Vec2::new(vp_x0, vp_y0);
        let vp_max = Vec2::new(vp_x1, vp_y1);

        let world_min = Vec2::new(vp_min.x - w * 0.5, h * 0.5 - vp_max.y);
        let world_max = Vec2::new(vp_max.x - w * 0.5, h * 0.5 - vp_min.y);
        let world_center = (world_min + world_max) * 0.5;
        let world_size = world_max - world_min;

        tiles.push(Tile {
            idx: i,
            vp_min,
            vp_max,
            world_min,
            world_max,
            world_center,
            world_size,
        });
    }

    tiles
}

fn tile_to_viewport(window: &Window, tile: &Tile) -> Viewport {
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

/* ========================================================================== */
/*                              ECS COMPONENTS                                 */
/* ========================================================================== */

#[derive(Component)]
struct Rendered;

#[derive(Component)]
struct TileCam {
    tile_idx: usize,
    kind: TileKind,
}

#[derive(Component)]
struct TileCam3D;

/* ========================================================================== */
/*                              PLUGIN                                         */
/* ========================================================================== */

pub struct DashRenderPlugin;

impl Plugin for DashRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredTile>()
            .init_resource::<TileViews>()
            .add_systems(Startup, setup_global_scene)
            .add_systems(
                Update,
                (
                    update_hovered_tile,
                    sync_tile_cameras,
                    handle_gestures.after(update_hovered_tile),
                    redraw_all_tiles.after(sync_tile_cameras),
                ),
            );
    }
}

fn setup_global_scene(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        brightness: 300.0,
        ..default()
    });
}

/* ========================================================================== */
/*                              HOVER DETECTION                                */
/* ========================================================================== */

fn update_hovered_tile(
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

/* ========================================================================== */
/*                              CAMERA MANAGEMENT                              */
/* ========================================================================== */

fn sync_tile_cameras(
    mut commands: Commands,
    dash: Option<Res<DashboardRes>>,
    windows: Query<&Window>,
    mut views: ResMut<TileViews>,
    existing_cams: Query<(Entity, &TileCam)>,
) {
    let Some(dash) = dash else {
        for (entity, _) in existing_cams.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    let (specs, _, _) = build_tile_specs(&dash.0);
    let n = specs.len();
    views.ensure_len(n);

    for (entity, tc) in existing_cams.iter() {
        if tc.tile_idx >= n {
            commands.entity(entity).despawn();
        }
    }

    if n == 0 {
        return;
    }

    let tiles = compute_tiles(window, n);

    let mut cam_map: Vec<Option<(Entity, TileKind)>> = vec![None; n];
    for (entity, tc) in existing_cams.iter() {
        if tc.tile_idx < n {
            cam_map[tc.tile_idx] = Some((entity, tc.kind));
        }
    }

    for spec in &specs {
        let ti = spec.tile_idx;
        let tile = &tiles[ti];
        let vp = tile_to_viewport(window, tile);
        let layer = RenderLayers::layer(ti % 32);

        match cam_map[ti] {
            Some((entity, existing_kind)) if existing_kind == spec.kind => {
                commands.entity(entity).insert(Camera {
                    viewport: Some(vp),
                    order: ti as isize,
                    clear_color: ClearColorConfig::Custom(Color::srgba(0.08, 0.08, 0.10, 1.0)),
                    ..default()
                });
            }
            Some((entity, _wrong_kind)) => {
                commands.entity(entity).despawn();
                spawn_camera(&mut commands, ti, spec.kind, vp, layer, &views);
            }
            None => {
                spawn_camera(&mut commands, ti, spec.kind, vp, layer, &views);
            }
        }
    }
}

fn spawn_camera(
    commands: &mut Commands,
    tile_idx: usize,
    kind: TileKind,
    viewport: Viewport,
    layer: RenderLayers,
    views: &TileViews,
) {
    match kind {
        TileKind::TwoD { .. } | TileKind::Placeholder2D => {
            commands.spawn((
                Camera2d,
                Camera {
                    viewport: Some(viewport),
                    order: tile_idx as isize,
                    clear_color: ClearColorConfig::Custom(Color::srgba(0.08, 0.08, 0.10, 1.0)),
                    ..default()
                },
                layer,
                TileCam { tile_idx, kind },
                Name::new(format!("Cam2D_{}", tile_idx)),
            ));
        }
        TileKind::ThreeD { .. } => {
            let v = views.v3.get(tile_idx).copied().unwrap_or_default();
            let transform = compute_3d_transform(&v);

            commands.spawn((
                Camera3d::default(),
                Camera {
                    viewport: Some(viewport),
                    order: tile_idx as isize,
                    clear_color: ClearColorConfig::Custom(Color::srgba(0.08, 0.08, 0.10, 1.0)),
                    ..default()
                },
                transform,
                layer.clone(),
                TileCam { tile_idx, kind },
                TileCam3D,
                Name::new(format!("Cam3D_{}", tile_idx)),
            ));

            commands.spawn((
                PointLight {
                    intensity: 4000.0,
                    ..default()
                },
                Transform::from_xyz(3.0, 4.0, 3.0),
                layer,
                Rendered,
                Name::new(format!("Light3D_{}", tile_idx)),
            ));
        }
    }
}

fn compute_3d_transform(v: &View3D) -> Transform {
    let cy = v.yaw.cos();
    let sy = v.yaw.sin();
    let cp = v.pitch.cos();
    let sp = v.pitch.sin();

    let dir = Vec3::new(sy * cp, sp, cy * cp);
    let pos = v.target + dir * v.radius;

    Transform::from_translation(pos).looking_at(v.target, Vec3::Y)
}

/* ========================================================================== */
/*                              GESTURE HANDLING                               */
/* ========================================================================== */

#[allow(deprecated)]
fn handle_gestures(
    mut wheel_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    dash: Option<Res<DashboardRes>>,
    hovered: Res<HoveredTile>,
    mut views: ResMut<TileViews>,
    mut cam3_query: Query<(&TileCam, &mut Transform), With<TileCam3D>>,
) {
    let Some(dash) = dash else {
        wheel_events.clear();
        motion_events.clear();
        return;
    };

    let (specs, _, _) = build_tile_specs(&dash.0);
    if specs.is_empty() {
        wheel_events.clear();
        motion_events.clear();
        return;
    }

    let Some(ti) = hovered.0 else {
        wheel_events.clear();
        motion_events.clear();
        return;
    };

    views.ensure_len(specs.len());

    let Some(spec) = specs.get(ti) else {
        wheel_events.clear();
        motion_events.clear();
        return;
    };

    let mut scroll_y = 0.0f32;
    for ev in wheel_events.read() {
        scroll_y += ev.y;
    }

    let mut mouse_delta = Vec2::ZERO;
    for ev in motion_events.read() {
        mouse_delta += ev.delta;
    }

    match spec.kind {
        TileKind::TwoD { .. } | TileKind::Placeholder2D => {
            let view = &mut views.v2[ti];

            if scroll_y.abs() > 0.001 {
                view.scale = (view.scale * (1.0 + scroll_y * 0.1)).clamp(10.0, 2000.0);
            }

            if mouse_buttons.pressed(MouseButton::Left) {
                view.offset.x += mouse_delta.x;
                view.offset.y -= mouse_delta.y;
            }
        }
        TileKind::ThreeD { .. } => {
            let view = &mut views.v3[ti];

            if scroll_y.abs() > 0.001 {
                view.radius = (view.radius * (1.0 - scroll_y * 0.1)).clamp(0.5, 500.0);
            }

            if mouse_buttons.pressed(MouseButton::Left) {
                view.yaw -= mouse_delta.x * view.orbit_speed;
                view.pitch = (view.pitch - mouse_delta.y * view.orbit_speed).clamp(-1.5, 1.5);
            }

            if mouse_buttons.pressed(MouseButton::Right) {
                let right = Vec3::new(view.yaw.cos(), 0.0, -view.yaw.sin());
                let fwd = Vec3::new(view.yaw.sin(), 0.0, view.yaw.cos());
                let pan =
                    (-right * mouse_delta.x + fwd * mouse_delta.y) * view.pan_speed * view.radius;
                view.target += pan;
            }

            for (tc, mut transform) in cam3_query.iter_mut() {
                if tc.tile_idx == ti {
                    *transform = compute_3d_transform(view);
                }
            }
        }
    }
}

/* ========================================================================== */
/*                              RENDERING                                      */
/* ========================================================================== */

fn redraw_all_tiles(
    mut commands: Commands,
    dash: Option<Res<DashboardRes>>,
    hovered: Res<HoveredTile>,
    views: Res<TileViews>,
    windows: Query<&Window>,
    rendered_entities: Query<Entity, With<Rendered>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials_2d: ResMut<Assets<ColorMaterial>>,
    mut materials_3d: ResMut<Assets<StandardMaterial>>,
) {
    let Some(dash) = dash else {
        for entity in rendered_entities.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    if !dash.is_changed() && !hovered.is_changed() && !views.is_changed() {
        return;
    }

    for entity in rendered_entities.iter() {
        commands.entity(entity).despawn();
    }

    let (specs, g2_list, g3_list) = build_tile_specs(&dash.0);
    if specs.is_empty() {
        return;
    }

    let tiles = compute_tiles(window, specs.len());

    for spec in &specs {
        let ti = spec.tile_idx;
        let tile = &tiles[ti];
        let layer = RenderLayers::layer(ti % 32);
        let is_hovered = hovered.0 == Some(ti);

        draw_tile_frame(
            &mut commands,
            &mut meshes,
            &mut materials_2d,
            tile,
            layer.clone(),
            is_hovered,
        );

        match spec.kind {
            TileKind::TwoD { g2_idx } => {
                let view = views.v2.get(ti).copied().unwrap_or_default();

                draw_2d_axes(
                    &mut commands,
                    &mut meshes,
                    &mut materials_2d,
                    tile,
                    layer.clone(),
                );

                if let Some(graph) = g2_list.get(g2_idx) {
                    for layer_data in &graph.layers {
                        draw_layer_2d(
                            &mut commands,
                            &mut meshes,
                            &mut materials_2d,
                            tile,
                            view,
                            layer.clone(),
                            layer_data,
                        );
                    }
                }
            }

            TileKind::Placeholder2D => {
                draw_2d_axes(
                    &mut commands,
                    &mut meshes,
                    &mut materials_2d,
                    tile,
                    layer.clone(),
                );
            }

            TileKind::ThreeD { g3_idx } => {
                draw_3d_axes(&mut commands, &mut meshes, &mut materials_3d, layer.clone());

                if let Some(graph) = g3_list.get(g3_idx) {
                    for layer_data in &graph.layers {
                        draw_layer_3d(
                            &mut commands,
                            &mut meshes,
                            &mut materials_3d,
                            layer.clone(),
                            layer_data,
                        );
                    }
                }
            }
        }
    }
}

/* ========================================================================== */
/*                              2D DRAWING                                     */
/* ========================================================================== */

fn style_to_color(style: &Style) -> Color {
    Color::srgba(style.color.r, style.color.g, style.color.b, style.color.a)
}

fn data_to_world(tile: &Tile, view: &View2D, data_pt: Vec2) -> Vec2 {
    tile.world_center + view.offset + data_pt * view.scale
}

fn draw_tile_frame(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: &Tile,
    layer: RenderLayers,
    is_hovered: bool,
) {
    let bg_alpha = if is_hovered { 0.18 } else { 0.10 };
    let border_alpha = if is_hovered { 0.9 } else { 0.5 };

    // Background - slightly inset from tile edges
    let inset = FRAME_THICKNESS;
    let bg_w = tile.world_size.x - inset * 2.0;
    let bg_h = tile.world_size.y - inset * 2.0;

    let bg_color = Color::srgba(0.12, 0.12, 0.14, bg_alpha);
    let bg_mat = materials.add(ColorMaterial::from(bg_color));
    let bg_mesh = meshes.add(create_rect_mesh(bg_w.max(1.0), bg_h.max(1.0)));

    commands.spawn((
        Mesh2d(bg_mesh),
        MeshMaterial2d(bg_mat),
        Transform::from_translation(tile.world_center.extend(-10.0)),
        layer.clone(),
        Rendered,
    ));

    // Border - draw as 4 thin rectangles along the inner edge of tile
    let border_color = Color::srgba(0.45, 0.45, 0.50, border_alpha);
    let border_mat = materials.add(ColorMaterial::from(border_color));

    let inner_left = tile.world_min.x + inset;
    let inner_right = tile.world_max.x - inset;
    let inner_bottom = tile.world_min.y + inset;
    let inner_top = tile.world_max.y - inset;

    let inner_w = inner_right - inner_left;
    let inner_h = inner_top - inner_bottom;

    // Top border
    let top_mesh = meshes.add(create_rect_mesh(inner_w, FRAME_THICKNESS));
    commands.spawn((
        Mesh2d(top_mesh),
        MeshMaterial2d(border_mat.clone()),
        Transform::from_translation(Vec3::new(
            tile.world_center.x,
            inner_top - FRAME_THICKNESS * 0.5,
            -9.0,
        )),
        layer.clone(),
        Rendered,
    ));

    // Bottom border
    let bottom_mesh = meshes.add(create_rect_mesh(inner_w, FRAME_THICKNESS));
    commands.spawn((
        Mesh2d(bottom_mesh),
        MeshMaterial2d(border_mat.clone()),
        Transform::from_translation(Vec3::new(
            tile.world_center.x,
            inner_bottom + FRAME_THICKNESS * 0.5,
            -9.0,
        )),
        layer.clone(),
        Rendered,
    ));

    // Left border
    let left_mesh = meshes.add(create_rect_mesh(FRAME_THICKNESS, inner_h));
    commands.spawn((
        Mesh2d(left_mesh),
        MeshMaterial2d(border_mat.clone()),
        Transform::from_translation(Vec3::new(
            inner_left + FRAME_THICKNESS * 0.5,
            tile.world_center.y,
            -9.0,
        )),
        layer.clone(),
        Rendered,
    ));

    // Right border
    let right_mesh = meshes.add(create_rect_mesh(FRAME_THICKNESS, inner_h));
    commands.spawn((
        Mesh2d(right_mesh),
        MeshMaterial2d(border_mat),
        Transform::from_translation(Vec3::new(
            inner_right - FRAME_THICKNESS * 0.5,
            tile.world_center.y,
            -9.0,
        )),
        layer,
        Rendered,
    ));
}

fn draw_2d_axes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: &Tile,
    layer: RenderLayers,
) {
    let axis_color = Color::srgba(0.5, 0.5, 0.55, 0.5);
    let axis_mat = materials.add(ColorMaterial::from(axis_color));

    // Inset axes from tile border
    let inset = FRAME_THICKNESS * 3.0;
    let axis_w = tile.world_size.x - inset * 2.0;
    let axis_h = tile.world_size.y - inset * 2.0;

    // Horizontal axis
    let h_mesh = meshes.add(create_rect_mesh(axis_w.max(1.0), 1.0));
    commands.spawn((
        Mesh2d(h_mesh),
        MeshMaterial2d(axis_mat.clone()),
        Transform::from_translation(Vec3::new(tile.world_center.x, tile.world_center.y, -5.0)),
        layer.clone(),
        Rendered,
    ));

    // Vertical axis
    let v_mesh = meshes.add(create_rect_mesh(1.0, axis_h.max(1.0)));
    commands.spawn((
        Mesh2d(v_mesh),
        MeshMaterial2d(axis_mat),
        Transform::from_translation(Vec3::new(tile.world_center.x, tile.world_center.y, -5.0)),
        layer,
        Rendered,
    ));
}

fn draw_layer_2d(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: &Tile,
    view: View2D,
    layer: RenderLayers,
    layer_data: &Layer2D,
) {
    let color = style_to_color(&layer_data.style);
    let mat = materials.add(ColorMaterial::from(color));

    match layer_data.geometry {
        Geometry2D::Line => {
            if layer_data.xy.len() < 2 {
                return;
            }

            let world_pts: Vec<Vec2> = layer_data
                .xy
                .iter()
                .map(|&p| data_to_world(tile, &view, p))
                .collect();

            let mesh = create_line_mesh_clipped(&world_pts, layer_data.style.size.max(1.5), tile);
            let mesh_handle = meshes.add(mesh);

            commands.spawn((
                Mesh2d(mesh_handle),
                MeshMaterial2d(mat),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                layer,
                Rendered,
            ));
        }

        Geometry2D::Points => {
            let radius = layer_data.style.size.max(2.0) * 0.5;
            let point_mesh = meshes.add(create_rect_mesh(radius * 2.0, radius * 2.0));

            // Cull bounds with proper inset
            let inset = FRAME_THICKNESS * 3.0;
            let cull_min = tile.world_min + Vec2::splat(inset);
            let cull_max = tile.world_max - Vec2::splat(inset);

            for &data_pt in &layer_data.xy {
                let world_pt = data_to_world(tile, &view, data_pt);

                if world_pt.x < cull_min.x - radius
                    || world_pt.x > cull_max.x + radius
                    || world_pt.y < cull_min.y - radius
                    || world_pt.y > cull_max.y + radius
                {
                    continue;
                }

                commands.spawn((
                    Mesh2d(point_mesh.clone()),
                    MeshMaterial2d(mat.clone()),
                    Transform::from_translation(world_pt.extend(1.0)),
                    layer.clone(),
                    Rendered,
                ));
            }
        }

        Geometry2D::Bars => {
            let mut bar_width = 0.3;
            if layer_data.xy.len() >= 2 {
                let mut min_gap = f32::INFINITY;
                for pair in layer_data.xy.windows(2) {
                    min_gap = min_gap.min((pair[1].x - pair[0].x).abs());
                }
                if min_gap.is_finite() && min_gap > 0.0 {
                    bar_width = min_gap * 0.75;
                }
            }

            for &data_pt in &layer_data.xy {
                let height = data_pt.y;
                if height.abs() < 0.0001 {
                    continue;
                }

                let bar_center_data = Vec2::new(data_pt.x, height * 0.5);
                let world_center = data_to_world(tile, &view, bar_center_data);

                let world_w = bar_width * view.scale;
                let world_h = height.abs() * view.scale;

                let bar_mesh = meshes.add(create_rect_mesh(world_w, world_h));
                commands.spawn((
                    Mesh2d(bar_mesh),
                    MeshMaterial2d(mat.clone()),
                    Transform::from_translation(world_center.extend(0.5)),
                    layer.clone(),
                    Rendered,
                ));
            }
        }

        _ => {}
    }
}

/* ========================================================================== */
/*                              3D DRAWING                                     */
/* ========================================================================== */

fn draw_3d_axes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    layer: RenderLayers,
) {
    let axis_len = 5.0;
    let axis_thick = 0.03;

    let mat_x = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.2, 0.2, 1.0),
        ..default()
    });
    let mesh_x = meshes.add(Cuboid::new(axis_len, axis_thick, axis_thick));
    commands.spawn((
        Mesh3d(mesh_x),
        MeshMaterial3d(mat_x),
        Transform::from_translation(Vec3::new(axis_len * 0.5, 0.0, 0.0)),
        layer.clone(),
        Rendered,
    ));

    let mat_y = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.9, 0.2, 1.0),
        ..default()
    });
    let mesh_y = meshes.add(Cuboid::new(axis_thick, axis_len, axis_thick));
    commands.spawn((
        Mesh3d(mesh_y),
        MeshMaterial3d(mat_y),
        Transform::from_translation(Vec3::new(0.0, axis_len * 0.5, 0.0)),
        layer.clone(),
        Rendered,
    ));

    let mat_z = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.4, 0.9, 1.0),
        ..default()
    });
    let mesh_z = meshes.add(Cuboid::new(axis_thick, axis_thick, axis_len));
    commands.spawn((
        Mesh3d(mesh_z),
        MeshMaterial3d(mat_z),
        Transform::from_translation(Vec3::new(0.0, 0.0, axis_len * 0.5)),
        layer,
        Rendered,
    ));
}

fn draw_layer_3d(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    layer: RenderLayers,
    layer_data: &Layer3D,
) {
    let color = style_to_color(&layer_data.style);

    match layer_data.geometry {
        Geometry3D::Points => {
            let radius = (layer_data.style.size * 0.02).max(0.02);
            let sphere_mesh = meshes.add(Sphere::new(radius));
            let mat = materials.add(StandardMaterial {
                base_color: color,
                ..default()
            });

            for &pt in &layer_data.xyz {
                commands.spawn((
                    Mesh3d(sphere_mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(pt),
                    layer.clone(),
                    Rendered,
                ));
            }
        }

        Geometry3D::Surface { grid } => {
            let mesh = create_surface_mesh(&layer_data.xyz, grid);
            let mesh_handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.9,
                cull_mode: None,
                ..default()
            });

            commands.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat),
                Transform::IDENTITY,
                layer,
                Rendered,
            ));
        }
    }
}

/* ========================================================================== */
/*                              MESH BUILDERS                                  */
/* ========================================================================== */

fn create_rect_mesh(width: f32, height: f32) -> Mesh {
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

fn create_line_mesh_clipped(points: &[Vec2], width: f32, tile: &Tile) -> Mesh {
    let half_w = width * 0.5;
    let mut positions: Vec<[f32; 3]> = Vec::new();

    // Inset clip region to stay within tile frame
    let inset = FRAME_THICKNESS * 3.0;
    let clip_min = tile.world_min + Vec2::splat(inset);
    let clip_max = tile.world_max - Vec2::splat(inset);

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

fn create_surface_mesh(vertices: &[Vec3], grid: UVec2) -> Mesh {
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
