use crate::render_gpt1::layout::TileRect;
use crate::render_gpt1::tile::{DashboardRes, PlotKind, PlotTile, RenderRoot, TileRegistry, View2D};
use bevy::prelude::*;
use bevy::prelude::{Mesh2d, MeshMaterial2d};
use bevy_camera::visibility::RenderLayers;
use wgpu_types::PrimitiveTopology;

#[derive(Resource, Clone)]
pub struct UnitMeshes {
    pub quad: Handle<Mesh>,
}

pub fn ensure_unit_meshes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(UnitMeshes {
        quad: meshes.add(Mesh::from(Rectangle::new(1.0, 1.0))),
    });
}

#[derive(Component)]
pub struct Rendered;

fn layer_for(index: usize) -> RenderLayers {
    RenderLayers::layer(1 + (index % 31))
}

pub fn redraw_changed_tiles(
    mut commands: Commands,
    dash: Option<Res<DashboardRes>>,
    reg: Res<TileRegistry>,
    unit: Res<UnitMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    // redraw on layout/view changes OR when dashboard changes
    changed: Query<
        (Entity, &PlotTile, &TileRect, Option<&View2D>),
        Or<(Changed<TileRect>, Changed<View2D>, Added<PlotTile>)>,
    >,
    all_tiles: Query<(Entity, &PlotTile, &TileRect, Option<&View2D>)>,
    roots_children: Query<&Children, With<RenderRoot>>,
    rendered: Query<(), With<Rendered>>,
) {
    let Some(dash) = dash else {
        return;
    };

    let redraw_all = dash.is_changed();
    let iter: Vec<(Entity, &PlotTile, &TileRect, Option<&View2D>)> = if redraw_all {
        all_tiles.iter().collect()
    } else {
        changed.iter().collect()
    };

    for (_tile_e, tile, rect, v2) in iter {
        let Some(&root_e) = reg.root_of.get(&tile.id) else {
            continue;
        };

        // Clear only this tile's previous renderables
        if let Ok(children) = roots_children.get(root_e) {
            for &c in children.iter() {
                if rendered.get(c).is_ok() {
                    commands.entity(c).despawn_recursive();
                }
            }
        }

        // Always draw tile frame (cheap)
        draw_frame(
            &mut commands,
            root_e,
            rect,
            &unit,
            &mut mats,
            layer_for(tile.index),
        );

        match tile.kind {
            PlotKind::TwoD => {
                // ADAPTER: match your real plot types
                let Some(crate::core::Plot::Graph2D(g)) = dash.0.plots.get(tile.index) else {
                    continue;
                };
                let view = v2.copied().unwrap_or_default();
                draw_graph2d(
                    &mut commands,
                    root_e,
                    rect,
                    &view,
                    g,
                    &mut meshes,
                    &mut mats,
                    layer_for(tile.index),
                );
            }
            _ => {}
        }
    }
}

fn draw_frame(
    commands: &mut Commands,
    root: Entity,
    rect: &TileRect,
    unit: &UnitMeshes,
    mats: &mut Assets<ColorMaterial>,
    layer: RenderLayers,
) {
    let bg = mats.add(ColorMaterial::from(Color::srgba(0.10, 0.10, 0.12, 0.12)));
    let border = mats.add(ColorMaterial::from(Color::srgba(0.5, 0.5, 0.58, 0.55)));

    commands.entity(root).with_children(|p| {
        p.spawn((
            Mesh2d(unit.quad.clone()),
            MeshMaterial2d(bg),
            Transform {
                translation: rect.world_center.extend(-10.0),
                scale: rect.world_size.extend(1.0),
                ..default()
            },
            layer,
            Rendered,
        ));

        let t = 1.0;
        let w = rect.world_size.x;
        let h = rect.world_size.y;

        let top = Vec3::new(
            rect.world_center.x,
            rect.world_center.y + h * 0.5 - t * 0.5,
            -9.0,
        );
        let bot = Vec3::new(
            rect.world_center.x,
            rect.world_center.y - h * 0.5 + t * 0.5,
            -9.0,
        );
        let left = Vec3::new(
            rect.world_center.x - w * 0.5 + t * 0.5,
            rect.world_center.y,
            -9.0,
        );
        let right = Vec3::new(
            rect.world_center.x + w * 0.5 - t * 0.5,
            rect.world_center.y,
            -9.0,
        );

        for (pos, sx, sy) in [(top, w, t), (bot, w, t), (left, t, h), (right, t, h)] {
            p.spawn((
                Mesh2d(unit.quad.clone()),
                MeshMaterial2d(border.clone()),
                Transform {
                    translation: pos,
                    scale: Vec3::new(sx, sy, 1.0),
                    ..default()
                },
                layer,
                Rendered,
            ));
        }
    });
}

fn draw_graph2d(
    commands: &mut Commands,
    root: Entity,
    rect: &TileRect,
    view: &View2D,
    graph: &crate::core::Graph2D,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<ColorMaterial>,
    layer: RenderLayers,
) {
    let Some(layer0) = graph.layers.first() else {
        return;
    };
    if layer0.xy.len() < 2 {
        return;
    }

    let pts: Vec<Vec2> = layer0
        .xy
        .iter()
        .map(|p| rect.content_center + view.offset_px + (*p) * view.scale)
        .collect();

    let mesh = polyline_mesh(&pts, 2.0, rect);
    let mh = meshes.add(mesh);
    let mat = mats.add(ColorMaterial::from(Color::srgba(0.2, 0.65, 0.95, 0.95)));

    commands.entity(root).with_children(|p| {
        p.spawn((
            Mesh2d(mh),
            MeshMaterial2d(mat),
            Transform::IDENTITY,
            layer,
            Rendered,
        ));
    });
}

fn polyline_mesh(points: &[Vec2], width: f32, rect: &TileRect) -> Mesh {
    let hw = width * 0.5;
    let mut pos: Vec<[f32; 3]> = Vec::new();

    for w in points.windows(2) {
        let a = w[0];
        let b = w[1];

        // light clip
        let min = rect.content_min;
        let max = rect.content_max;
        if (a.x < min.x && b.x < min.x)
            || (a.x > max.x && b.x > max.x)
            || (a.y < min.y && b.y < min.y)
            || (a.y > max.y && b.y > max.y)
        {
            continue;
        }

        let d = b - a;
        let len = d.length().max(1e-6);
        let n = Vec2::new(-d.y, d.x) / len * hw;

        let v0 = a + n;
        let v1 = b + n;
        let v2 = a - n;
        let v3 = b - n;

        pos.extend_from_slice(&[
            [v0.x, v0.y, 0.0],
            [v1.x, v1.y, 0.0],
            [v2.x, v2.y, 0.0],
            [v2.x, v2.y, 0.0],
            [v1.x, v1.y, 0.0],
            [v3.x, v3.y, 0.0],
        ]);
    }

    let n = pos.len();
    let normals = vec![[0.0, 0.0, 1.0]; n];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh
}
