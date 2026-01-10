use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css::{AQUA, BLACK, BLUE, DARK_GRAY, DEEP_SKY_BLUE, GRAY, LIME, RED, WHITE},
    input::mouse::{MouseMotion, MouseWheel},
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use bevy_prototype_lyon::prelude::*;

use crate::plot::{Geometry2D, Graph2D, Layer2D, Layer3D, Plot};
use crate::utils::{Color as VidiColor, Style};

/// Only one plot for now.
#[derive(Resource, Clone, Debug)]
pub struct PlotScene {
    pub plot: Plot,
}
impl PlotScene {
    pub fn single(plot: Plot) -> Self {
        Self { plot }
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum RenderMode {
    TwoD,
    ThreeD,
}

/// 2D pan/zoom state.
#[derive(Resource, Clone, Copy, Debug)]
pub struct View2D {
    pub offset: Vec2,
    pub scale: f32,
    pub invert_pan: bool,
}
impl Default for View2D {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 80.0,
            invert_pan: false,
        }
    }
}

/// 3D orbit state.
#[derive(Resource, Clone, Copy, Debug)]
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

#[derive(Component)]
struct Rendered;

#[derive(Component)]
struct PlotCam2D;

#[derive(Component)]
struct PlotCam3D;

pub struct PlotRenderPlugin;

impl Plugin for PlotRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<View2D>()
            .init_resource::<View3D>()
            .insert_resource(RenderMode::TwoD)
            .add_plugins(ShapePlugin)
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_2d_gestures, apply_3d_camera))
            .add_systems(Update, (redraw_2d, redraw_3d));
    }
}

fn setup(mut commands: Commands, scene: Res<PlotScene>) {
    let mode = match &scene.plot {
        Plot::Graph3D(_) => RenderMode::ThreeD,
        _ => RenderMode::TwoD,
    };
    commands.insert_resource(mode);

    match mode {
        RenderMode::TwoD => {
            commands.spawn((
                Camera2d::default(),
                Msaa::Sample4,
                PlotCam2D,
                Name::new("PlotCam2D"),
            ));
        }
        RenderMode::ThreeD => {
            let t = Transform::from_xyz(2.8, 2.2, 2.8).looking_at(Vec3::ZERO, Vec3::Y);
            commands.spawn((Camera3d::default(), t, PlotCam3D, Name::new("PlotCam3D")));
            commands.spawn((PointLight::default(), t, Name::new("PlotLight")));
            commands.insert_resource(AmbientLight {
                brightness: 500.0,
                ..default()
            });
        }
    }
}

/* -----------------------------
   2D: pan + zoom
------------------------------ */

fn handle_2d_gestures(
    mut ev_wheel: MessageReader<MouseWheel>,
    mut ev_motion: MessageReader<MouseMotion>,
    btn: Res<ButtonInput<MouseButton>>,
    mode: Res<RenderMode>,
    mut view: ResMut<View2D>,
    windows: Query<&Window>,
) {
    if *mode != RenderMode::TwoD {
        ev_wheel.clear();
        ev_motion.clear();
        return;
    }

    let window = windows.single().expect("expected exactly one Window");

    for ev in ev_wheel.read() {
        view.scale = (view.scale - ev.y * 10.0).clamp(20.0, 600.0);
    }

    let mut delta = Vec2::ZERO;
    for ev in ev_motion.read() {
        delta += ev.delta;
    }

    if btn.pressed(MouseButton::Left) {
        // invert_pan=true feels like “grab the paper”
        let s = if view.invert_pan { 1.0 } else { -1.0 };
        view.offset.x += s * delta.x;
        view.offset.y += -s * delta.y;
    }

    // keep origin in view
    let hw = window.width() * 0.5;
    let hh = window.height() * 0.5;
    view.offset.x = view.offset.x.clamp(-hw + 20.0, hw - 20.0);
    view.offset.y = view.offset.y.clamp(-hh + 20.0, hh - 20.0);
}

fn redraw_2d(
    mut commands: Commands,
    mode: Res<RenderMode>,
    scene: Res<PlotScene>,
    view: Res<View2D>,
    windows: Query<&Window>,
    rendered: Query<Entity, With<Rendered>>,
) {
    if *mode != RenderMode::TwoD {
        return;
    }
    if !(scene.is_changed() || view.is_changed()) {
        return;
    }

    let window = windows.single().expect("expected exactly one Window");

    for e in rendered.iter() {
        commands.entity(e).despawn();
    }

    draw_axes_and_ticks(&mut commands, window, *view);

    match &scene.plot {
        Plot::Graph2D(g) => draw_graph2d(&mut commands, g, *view),
        Plot::Composite(v) => {
            for pp in v {
                if let Plot::Graph2D(g) = pp {
                    draw_graph2d(&mut commands, g, *view);
                }
            }
        }
        _ => {}
    }
}

fn draw_axes_and_ticks(commands: &mut Commands, window: &Window, view: View2D) {
    let origin = view.offset;
    let hw = window.width() * 0.5;
    let hh = window.height() * 0.5;

    // axes
    commands.spawn((
        ShapeBuilder::with(&shapes::Line(
            Vec2::new(-1_000_000.0, origin.y),
            Vec2::new(1_000_000.0, origin.y),
        ))
        .stroke((GRAY, 1.5))
        .build(),
        Rendered,
    ));
    commands.spawn((
        ShapeBuilder::with(&shapes::Line(
            Vec2::new(origin.x, -1_000_000.0),
            Vec2::new(origin.x, 1_000_000.0),
        ))
        .stroke((GRAY, 1.5))
        .build(),
        Rendered,
    ));

    // ticks
    let tick = 5.0;

    let start_x = (((-hw - origin.x) / view.scale).floor() as i32).saturating_sub(1);
    let end_x = (((hw - origin.x) / view.scale).ceil() as i32).saturating_add(1);
    for xi in start_x..=end_x {
        if xi == 0 {
            continue;
        }
        let sx = origin.x + xi as f32 * view.scale;
        commands.spawn((
            ShapeBuilder::with(&shapes::Line(
                Vec2::new(sx, origin.y - tick),
                Vec2::new(sx, origin.y + tick),
            ))
            .stroke((DARK_GRAY, 1.0))
            .build(),
            Rendered,
        ));
    }

    let start_y = (((-hh - origin.y) / view.scale).floor() as i32).saturating_sub(1);
    let end_y = (((hh - origin.y) / view.scale).ceil() as i32).saturating_add(1);
    for yi in start_y..=end_y {
        if yi == 0 {
            continue;
        }
        let sy = origin.y + yi as f32 * view.scale;
        commands.spawn((
            ShapeBuilder::with(&shapes::Line(
                Vec2::new(origin.x - tick, sy),
                Vec2::new(origin.x + tick, sy),
            ))
            .stroke((DARK_GRAY, 1.0))
            .build(),
            Rendered,
        ));
    }
}

fn draw_graph2d(commands: &mut Commands, g: &Graph2D, view: View2D) {
    for layer in &g.layers {
        draw_layer2d(commands, layer, &g.style, view);
    }
}

fn draw_layer2d(commands: &mut Commands, layer: &Layer2D, graph_style: &Style, view: View2D) {
    if layer.xy.is_empty() {
        return;
    }

    let origin = view.offset;
    let pts_screen: Vec<Vec2> = layer.xy.iter().map(|p| origin + *p * view.scale).collect();

    let color = pick_color(&layer.style, graph_style);
    let stroke_w = layer.style.size.or(graph_style.size).unwrap_or(2.0);
    let point_r = (layer.style.size.or(graph_style.size).unwrap_or(3.0) * 0.5).max(1.5);

    match layer.geometry {
        Geometry2D::Line => {
            if pts_screen.len() < 2 {
                return;
            }
            for w in pts_screen.windows(2) {
                commands.spawn((
                    ShapeBuilder::with(&shapes::Line(w[0], w[1]))
                        .stroke((color, stroke_w))
                        .build(),
                    Rendered,
                ));
            }
        }
        Geometry2D::Points => {
            for p in pts_screen {
                commands.spawn((
                    ShapeBuilder::with(&shapes::Circle {
                        radius: point_r,
                        center: p,
                    })
                    .fill(color)
                    .build(),
                    Rendered,
                ));
            }
        }
        _ => {}
    }
}

/* -----------------------------
   3D: orbit + surface mesh
------------------------------ */

fn apply_3d_camera(
    mut ev_wheel: MessageReader<MouseWheel>,
    mut ev_motion: MessageReader<MouseMotion>,
    btn: Res<ButtonInput<MouseButton>>,
    mode: Res<RenderMode>,
    mut view: ResMut<View3D>,
    mut cams: Query<&mut Transform, With<PlotCam3D>>,
) {
    if *mode != RenderMode::ThreeD {
        ev_wheel.clear();
        ev_motion.clear();
        return;
    }

    for ev in ev_wheel.read() {
        view.radius = (view.radius - ev.y * 0.5).clamp(1.0, 300.0);
    }

    let mut delta = Vec2::ZERO;
    for ev in ev_motion.read() {
        delta += ev.delta;
    }

    // orbit (LMB)
    if btn.pressed(MouseButton::Left) {
        view.yaw -= delta.x * view.orbit_speed;
        view.pitch = (view.pitch - delta.y * view.orbit_speed).clamp(-1.5, 1.5);
    }

    // pan (RMB) — fix borrow checker by using locals
    if btn.pressed(MouseButton::Right) {
        let pan_speed = view.pan_speed;
        let radius = view.radius;
        let right = Vec3::new(view.yaw.cos(), 0.0, -view.yaw.sin());
        let fwd = Vec3::new(view.yaw.sin(), 0.0, view.yaw.cos());
        let dv = (-right * delta.x + fwd * delta.y) * pan_speed * radius;
        view.target += dv;
    }

    let Ok(mut tf) = cams.single_mut() else {
        return;
    };

    let cy = view.yaw.cos();
    let sy = view.yaw.sin();
    let cp = view.pitch.cos();
    let sp = view.pitch.sin();

    let dir = Vec3::new(sy * cp, sp, cy * cp);
    let pos = view.target + dir * view.radius;
    *tf = Transform::from_translation(pos).looking_at(view.target, Vec3::Y);
}

fn redraw_3d(
    mut commands: Commands,
    mode: Res<RenderMode>,
    scene: Res<PlotScene>,
    view: Res<View3D>,
    rendered: Query<Entity, With<Rendered>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *mode != RenderMode::ThreeD {
        return;
    }
    if !(scene.is_changed() || view.is_changed()) {
        return;
    }

    for e in rendered.iter() {
        commands.entity(e).despawn();
    }

    let Plot::Graph3D(g) = &scene.plot else {
        return;
    };
    let Some(layer) = g.layers.first() else {
        return;
    };

    spawn_axes_grid_3d(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::ZERO,
        6.0,
        2.5,
        6.0,  // x_len, y_len, z_len
        0.5,  // grid_step
        0.02, // thickness
    );
    // Layer3D fields (matches your error): geometry, xyz, grid, style
    let mesh = build_surface_mesh(layer);
    let mesh_h = meshes.add(mesh);

    let base = pick_color(&layer.style, &g.style);
    let mat_h = materials.add(StandardMaterial {
        base_color: Color::from(base),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh_h),
        MeshMaterial3d(mat_h),
        Transform::default(),
        Rendered,
    ));
}

fn bounds_xy(layer: &Layer3D) -> (Vec2, Vec2) {
    // returns (min_xy, max_xy)
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for p in &layer.xyz {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    (min, max)
}

#[derive(Component)]
pub struct AxesGrid3D;

pub fn spawn_axes_grid_3d(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
    x_len: f32,
    y_len: f32,
    z_len: f32,
    grid_step: f32,
    thickness: f32,
) {
    // --- materials (created once, then cloned as handles) ---
    let mat_x = materials.add(StandardMaterial {
        base_color: Color::from(RED),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    let mat_y = materials.add(StandardMaterial {
        base_color: Color::from(LIME),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    let mat_z = materials.add(StandardMaterial {
        base_color: Color::from(BLUE),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    let mat_grid = materials.add(StandardMaterial {
        base_color: Color::from(DARK_GRAY),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });

    // --- meshes ---
    let x_mesh = meshes.add(Cuboid::new(x_len, thickness, thickness));
    let y_mesh = meshes.add(Cuboid::new(thickness, y_len, thickness));
    let z_mesh = meshes.add(Cuboid::new(thickness, thickness, z_len));

    // --- axes ---
    commands.spawn((
        Mesh3d(x_mesh),
        MeshMaterial3d(mat_x.clone()),
        Transform::from_translation(origin + Vec3::new(x_len * 0.5, 0.0, 0.0)),
        AxesGrid3D,
        Name::new("AxisX"),
    ));
    commands.spawn((
        Mesh3d(y_mesh),
        MeshMaterial3d(mat_y.clone()),
        Transform::from_translation(origin + Vec3::new(0.0, y_len * 0.5, 0.0)),
        AxesGrid3D,
        Name::new("AxisY"),
    ));
    commands.spawn((
        Mesh3d(z_mesh),
        MeshMaterial3d(mat_z.clone()),
        Transform::from_translation(origin + Vec3::new(0.0, 0.0, z_len * 0.5)),
        AxesGrid3D,
        Name::new("AxisZ"),
    ));

    // --- XZ grid on y = origin.y ---
    if grid_step > 0.0 {
        let half_x = x_len * 0.5;
        let half_z = z_len * 0.5;

        let nx = (half_x / grid_step).floor() as i32;
        let nz = (half_z / grid_step).floor() as i32;

        // lines parallel to X (varying Z)
        for iz in -nz..=nz {
            let z = iz as f32 * grid_step;
            let mesh = meshes.add(Cuboid::new(x_len, thickness * 0.75, thickness * 0.75));
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_grid.clone()),
                Transform::from_translation(origin + Vec3::new(0.0, 0.0, z)),
                AxesGrid3D,
                Name::new("GridX"),
            ));
        }

        // lines parallel to Z (varying X)
        for ix in -nx..=nx {
            let x = ix as f32 * grid_step;
            let mesh = meshes.add(Cuboid::new(thickness * 0.75, thickness * 0.75, z_len));
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_grid.clone()),
                Transform::from_translation(origin + Vec3::new(x, 0.0, 0.0)),
                AxesGrid3D,
                Name::new("GridZ"),
            ));
        }
    }
}

/// Build a triangle-list mesh from a w×h grid of vertices.
/// Uses layer.grid.w / layer.grid.h (what your Layer3D actually has).
fn build_surface_mesh(layer: &Layer3D) -> Mesh {
    let grid = layer.grid.unwrap(); // Ensure grid is not None
    let w = grid.x as u32;
    let h = grid.y as u32;
    let verts = &layer.xyz;

    if w < 2 || h < 2 {
        // empty mesh, but keep it valid
        return Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
    }
    assert_eq!(verts.len(), (w * h) as usize);

    let positions: Vec<[f32; 3]> = verts.iter().map(|v| [v.x, v.y, v.z]).collect();

    let uvs: Vec<[f32; 2]> = (0..h)
        .flat_map(|yy| {
            (0..w).map(move |xx| {
                let u = xx as f32 / (w - 1) as f32;
                let v = yy as f32 / (h - 1) as f32;
                [u, v]
            })
        })
        .collect();

    let mut indices: Vec<u32> = Vec::with_capacity(((w - 1) * (h - 1) * 6) as usize);
    for y in 0..(h - 1) {
        for x in 0..(w - 1) {
            let i0 = y * w + x;
            let i1 = i0 + 1;
            let i2 = i0 + w;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    let normals = compute_normals(&positions, &indices);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn compute_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut n = vec![Vec3::ZERO; positions.len()];
    let p = |i: usize| Vec3::new(positions[i][0], positions[i][1], positions[i][2]);

    for tri in indices.chunks_exact(3) {
        let a = tri[0] as usize;
        let b = tri[1] as usize;
        let c = tri[2] as usize;

        let e1 = p(b) - p(a);
        let e2 = p(c) - p(a);
        let nn = e1.cross(e2);

        n[a] += nn;
        n[b] += nn;
        n[c] += nn;
    }

    n.into_iter()
        .map(|v| {
            let v = v.normalize_or_zero();
            [v.x, v.y, v.z]
        })
        .collect()
}

/* -----------------------------
   color
------------------------------ */

fn pick_color(layer: &Style, graph: &Style) -> Srgba {
    if let Some(c) = layer.color.as_ref().or(graph.color.as_ref()) {
        vidi_to_srgba(c, layer.opacity.or(graph.opacity))
    } else {
        DEEP_SKY_BLUE
    }
}

fn vidi_to_srgba(c: &VidiColor, extra_opacity: Option<f32>) -> Srgba {
    let mut out = match *c {
        VidiColor::Rgb(r, g, b) => Srgba::new(r, g, b, 1.0),
        VidiColor::Rgba(r, g, b, a) => Srgba::new(r, g, b, a),
        VidiColor::Named(name) => match name {
            "white" => Srgba::WHITE,
            "black" => Srgba::BLACK,
            "red" => Srgba::RED,
            "cyan" => Srgba::new(0.0, 1.0, 1.0, 1.0),
            _ => DEEP_SKY_BLUE,
        },
    };

    if let Some(op) = extra_opacity {
        out.alpha = (out.alpha * op).clamp(0.0, 1.0);
    }
    out
}

// use bevy::color::palettes::css::{
//     AQUA, BLACK, BLUE, DARK_GRAY, DEEP_SKY_BLUE, GRAY, LIME, RED, WHITE,
// };
// use bevy::input::gestures::PinchGesture;
// use bevy::input::mouse::{MouseMotion, MouseWheel};
// use bevy::prelude::*;
// use bevy_prototype_lyon::prelude::*;

// use crate::plot::{Geometry2D, Graph2D, Layer2D, Plot};
// use crate::utils::{Color as VidiColor, Style};

// /// What the app is currently displaying.
// /// A “scene” is just a list of plots.
// #[derive(Resource, Clone, Debug, Default)]
// pub struct PlotScene {
//     pub plots: Vec<Plot>,
// }

// impl PlotScene {
//     pub fn single(plot: Plot) -> Self {
//         Self { plots: vec![plot] }
//     }
// }

// /// Pan/zoom state for 2D plotting.
// /// We render in “screen-ish world units” (Bevy 2D default: 1 unit ~= 1 pixel).
// #[derive(Resource, Clone, Copy, Debug)]
// pub struct View2D {
//     pub offset: Vec2, // origin in world coords (screen-like)
//     pub scale: f32,   // pixels per 1.0 plot-unit
// }

// impl Default for View2D {
//     fn default() -> Self {
//         Self {
//             offset: Vec2::ZERO,
//             scale: 80.0,
//         }
//     }
// }

// /// Marks entities created by the renderer so we can clear/redraw cheaply.
// #[derive(Component)]
// struct Rendered;

// /// Renderer plugin (the thing you import in main).
// pub struct PlotRenderPlugin;

// struct LabelFont(pub Handle<Font>);

// impl Plugin for PlotRenderPlugin {
//     fn build(&self, app: &mut App) {
//         app.insert_resource(View2D::default())
//             .add_plugins(ShapePlugin)
//             .add_systems(Startup, setup)
//             .add_systems(Update, handle_gestures)
//             // Redraw only when view or scene changes.
//             .add_systems(
//                 Update,
//                 redraw.run_if(resource_changed::<View2D>.or(resource_changed::<PlotScene>)),
//             );
//     }
// }

// // fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
// fn setup(mut commands: Commands) {
//     commands.spawn((Camera2d, Msaa::Sample4));
//     // commands.insert_resource(LabelFont(asset_server.load("fonts/FiraSans-Bold.ttf")));
// }

// /// Pan + zoom (wheel + pinch + left-drag), clamp so the origin never leaves view.
// fn handle_gestures(
//     mut ev_wheel: MessageReader<MouseWheel>,
//     mut ev_pinch: MessageReader<PinchGesture>,
//     mut ev_motion: MessageReader<MouseMotion>,
//     btn: Res<ButtonInput<MouseButton>>,
//     mut view: ResMut<View2D>,
//     windows: Query<&Window>,
// ) {
//     let window = windows.single().expect("expected exactly one Window");

//     // ---- zoom ----
//     for ev in ev_pinch.read() {
//         view.scale = (view.scale * (1.0 + ev.0)).clamp(20.0, 400.0);
//     }
//     for ev in ev_wheel.read() {
//         view.scale = (view.scale - ev.y * 10.0).clamp(20.0, 400.0);
//     }

//     // ---- pan ----
//     let mut delta = Vec2::ZERO;
//     for ev in ev_motion.read() {
//         delta += ev.delta;
//     }
//     if btn.pressed(MouseButton::Left) {
//         view.offset.x -= delta.x;
//         view.offset.y += delta.y;
//     }

//     // ---- clamp origin to stay visible ----
//     let hw = window.width() * 0.5;
//     let hh = window.height() * 0.5;
//     view.offset.x = view.offset.x.clamp(-hw + 20.0, hw - 20.0);
//     view.offset.y = view.offset.y.clamp(-hh + 20.0, hh - 20.0);
// }

// fn redraw(
//     mut commands: Commands,
//     scene: Res<PlotScene>,
//     view: Res<View2D>,
//     windows: Query<&Window>,
//     rendered: Query<Entity, With<Rendered>>,
// ) {
//     let window = windows.single().expect("expected exactly one Window");

//     // Clear previous render output.
//     for e in rendered.iter() {
//         commands.entity(e).despawn();
//     }

//     // Axes + ticks (no text assets yet).
//     draw_axes_and_ticks(&mut commands, window, *view);

//     // Draw plots.
//     for p in &scene.plots {
//         match p {
//             Plot::Graph2D(g) => draw_graph2d(&mut commands, g, *view),
//             Plot::Composite(v) => {
//                 for pp in v {
//                     if let Plot::Graph2D(g) = pp {
//                         draw_graph2d(&mut commands, g, *view);
//                     }
//                 }
//             }
//             _ => {}
//         }
//     }
// }

// fn draw_axes_and_ticks(commands: &mut Commands, window: &Window, view: View2D) {
//     let origin = view.offset;
//     let hw = window.width() * 0.5;
//     let hh = window.height() * 0.5;

//     // Axes
//     commands.spawn((
//         ShapeBuilder::with(&shapes::Line(
//             Vec2::new(-1_000_000.0, origin.y),
//             Vec2::new(1_000_000.0, origin.y),
//         ))
//         .stroke((GRAY, 1.5))
//         .build(),
//         Rendered,
//     ));
//     commands.spawn((
//         ShapeBuilder::with(&shapes::Line(
//             Vec2::new(origin.x, -1_000_000.0),
//             Vec2::new(origin.x, 1_000_000.0),
//         ))
//         .stroke((GRAY, 1.5))
//         .build(),
//         Rendered,
//     ));

//     // Tick marks every 1.0 unit
//     let tick = 5.0;

//     let start_x = (((-hw - origin.x) / view.scale).floor() as i32).saturating_sub(1);
//     let end_x = (((hw - origin.x) / view.scale).ceil() as i32).saturating_add(1);
//     for xi in start_x..=end_x {
//         if xi == 0 {
//             continue;
//         }
//         let sx = origin.x + xi as f32 * view.scale;
//         commands.spawn((
//             ShapeBuilder::with(&shapes::Line(
//                 Vec2::new(sx, origin.y - tick),
//                 Vec2::new(sx, origin.y + tick),
//             ))
//             .stroke((DARK_GRAY, 1.0))
//             .build(),
//             Rendered,
//         ));
//     }

//     let start_y = (((-hh - origin.y) / view.scale).floor() as i32).saturating_sub(1);
//     let end_y = (((hh - origin.y) / view.scale).ceil() as i32).saturating_add(1);
//     for yi in start_y..=end_y {
//         if yi == 0 {
//             continue;
//         }
//         let sy = origin.y + yi as f32 * view.scale;
//         commands.spawn((
//             ShapeBuilder::with(&shapes::Line(
//                 Vec2::new(origin.x - tick, sy),
//                 Vec2::new(origin.x + tick, sy),
//             ))
//             .stroke((DARK_GRAY, 1.0))
//             .build(),
//             Rendered,
//         ));
//     }
// }

// fn draw_graph2d(commands: &mut Commands, g: &Graph2D, view: View2D) {
//     for layer in &g.layers {
//         draw_layer2d(commands, layer, &g.style, view);
//     }
// }

// fn draw_layer2d(commands: &mut Commands, layer: &Layer2D, graph_style: &Style, view: View2D) {
//     if layer.xy.is_empty() {
//         return;
//     }

//     let origin = view.offset;
//     let pts_screen: Vec<Vec2> = layer.xy.iter().map(|p| origin + *p * view.scale).collect();

//     let color = pick_color(&layer.style, graph_style);
//     let stroke_w = layer.style.size.or(graph_style.size).unwrap_or(2.0);
//     let point_r = (layer.style.size.or(graph_style.size).unwrap_or(3.0) * 0.5).max(1.5);

//     match layer.geometry {
//         Geometry2D::Line => {
//             if pts_screen.len() < 2 {
//                 return;
//             }
//             for w in pts_screen.windows(2) {
//                 commands.spawn((
//                     ShapeBuilder::with(&shapes::Line(w[0], w[1]))
//                         .stroke((color, stroke_w))
//                         .build(),
//                     Rendered,
//                 ));
//             }
//         }
//         Geometry2D::Points => {
//             for p in pts_screen {
//                 commands.spawn((
//                     ShapeBuilder::with(&shapes::Circle {
//                         radius: point_r,
//                         center: p,
//                     })
//                     .fill(color)
//                     .build(),
//                     Rendered,
//                 ));
//             }
//         }
//         _ => {}
//     }
// }

// fn pick_color(layer: &Style, graph: &Style) -> Srgba {
//     if let Some(c) = layer.color.as_ref().or(graph.color.as_ref()) {
//         vidi_to_srgba(c, layer.opacity.or(graph.opacity))
//     } else {
//         DEEP_SKY_BLUE
//     }
// }

// fn vidi_to_srgba(c: &VidiColor, extra_opacity: Option<f32>) -> Srgba {
//     let mut out = match *c {
//         VidiColor::Rgb(r, g, b) => Srgba::new(r, g, b, 1.0),
//         VidiColor::Rgba(r, g, b, a) => Srgba::new(r, g, b, a),
//         VidiColor::Named(name) => match name {
//             "white" => WHITE,
//             "black" => BLACK,
//             "red" => RED,
//             "green" => LIME,
//             "blue" => BLUE,
//             "gray" => GRAY,
//             "cyan" => AQUA, // Bevy CSS palette uses AQUA / DEEP_SKY_BLUE etc.
//             _ => DEEP_SKY_BLUE,
//         },
//     };

//     if let Some(op) = extra_opacity {
//         out.alpha = (out.alpha * op).clamp(0.0, 1.0);
//     }
//     out
// }
