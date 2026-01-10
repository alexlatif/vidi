use bevy::prelude::*;
use bevy_camera::visibility::RenderLayers;

use crate::core::{Geometry2D, Layer2D, Style};

use super::layout::{FRAME_THICKNESS, compute_tiles};
use super::mesh::{create_line_mesh_clipped, create_rect_mesh};
use super::resources::{DashboardRes, HoveredTile, TileViews, View2D};
use super::specs::{TileKind, build_tile_specs};

#[derive(Component)]
pub struct Rendered;

fn style_to_color(style: &Style) -> Color {
    Color::srgba(style.color.r, style.color.g, style.color.b, style.color.a)
}

fn data_to_world(tile: &super::layout::Tile, view: &View2D, data_pt: Vec2) -> Vec2 {
    // IMPORTANT: anchor to content rect so plot "fits within the 2 space"
    tile.content_center + view.offset + data_pt * view.scale
}

fn draw_tile_frame(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: &super::layout::Tile,
    layer: RenderLayers,
    is_hovered: bool,
) {
    let bg_alpha = if is_hovered { 0.15 } else { 0.08 };
    let border_alpha = if is_hovered { 0.8 } else { 0.4 };

    let bg_color = Color::srgba(0.12, 0.12, 0.14, bg_alpha);
    let bg_mat = materials.add(ColorMaterial::from(bg_color));
    let bg_mesh = meshes.add(create_rect_mesh(tile.world_size.x, tile.world_size.y));

    commands.spawn((
        Mesh2d(bg_mesh),
        MeshMaterial2d(bg_mat),
        Transform::from_translation(tile.world_center.extend(-10.0)),
        layer.clone(),
        Rendered,
    ));

    let border_color = Color::srgba(0.4, 0.4, 0.45, border_alpha);
    let border_mat = materials.add(ColorMaterial::from(border_color));

    let t = FRAME_THICKNESS;
    let w = tile.world_size.x;
    let h = tile.world_size.y;

    let horiz_mesh = meshes.add(create_rect_mesh(w, t));
    let vert_mesh = meshes.add(create_rect_mesh(t, h));

    // Top (inside)
    commands.spawn((
        Mesh2d(horiz_mesh.clone()),
        MeshMaterial2d(border_mat.clone()),
        Transform::from_translation(Vec3::new(
            tile.world_center.x,
            tile.world_max.y - t * 0.5,
            -9.0,
        )),
        layer.clone(),
        Rendered,
    ));

    // Bottom (inside)
    commands.spawn((
        Mesh2d(horiz_mesh),
        MeshMaterial2d(border_mat.clone()),
        Transform::from_translation(Vec3::new(
            tile.world_center.x,
            tile.world_min.y + t * 0.5,
            -9.0,
        )),
        layer.clone(),
        Rendered,
    ));

    // Left (inside)
    commands.spawn((
        Mesh2d(vert_mesh.clone()),
        MeshMaterial2d(border_mat.clone()),
        Transform::from_translation(Vec3::new(
            tile.world_min.x + t * 0.5,
            tile.world_center.y,
            -9.0,
        )),
        layer.clone(),
        Rendered,
    ));

    // Right (inside)
    commands.spawn((
        Mesh2d(vert_mesh),
        MeshMaterial2d(border_mat),
        Transform::from_translation(Vec3::new(
            tile.world_max.x - t * 0.5,
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
    tile: &super::layout::Tile,
    view: View2D,
    layer: RenderLayers,
) {
    let axis_color = Color::srgba(0.5, 0.5, 0.55, 0.6);
    let axis_mat = materials.add(ColorMaterial::from(axis_color));

    // Transform origin (0,0) in data space to world space
    let origin_world = tile.content_center + view.offset;

    // Clamp axes to content rect
    let h_pos_x = tile.content_center.x;
    let h_pos_y = origin_world.y.clamp(tile.content_min.y, tile.content_max.y);

    let v_pos_x = origin_world.x.clamp(tile.content_min.x, tile.content_max.x);
    let v_pos_y = tile.content_center.y;

    // Horizontal axis (y = 0 in data space)
    let h_mesh = meshes.add(create_rect_mesh(tile.content_size.x, 1.0));
    commands.spawn((
        Mesh2d(h_mesh),
        MeshMaterial2d(axis_mat.clone()),
        Transform::from_translation(Vec3::new(h_pos_x, h_pos_y, -5.0)),
        layer.clone(),
        Rendered,
    ));

    // Vertical axis (x = 0 in data space)
    let v_mesh = meshes.add(create_rect_mesh(1.0, tile.content_size.y));
    commands.spawn((
        Mesh2d(v_mesh),
        MeshMaterial2d(axis_mat),
        Transform::from_translation(Vec3::new(v_pos_x, v_pos_y, -5.0)),
        layer,
        Rendered,
    ));
}

fn draw_layer_2d(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: &super::layout::Tile,
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
                Transform::from_translation(Vec3::ZERO),
                layer,
                Rendered,
            ));
        }

        Geometry2D::Points => {
            let radius = layer_data.style.size.max(2.0) * 0.5;
            let point_mesh = meshes.add(create_rect_mesh(radius * 2.0, radius * 2.0));

            for &data_pt in &layer_data.xy {
                let world_pt = data_to_world(tile, &view, data_pt);

                if world_pt.x < tile.content_min.x - radius
                    || world_pt.x > tile.content_max.x + radius
                    || world_pt.y < tile.content_min.y - radius
                    || world_pt.y > tile.content_max.y + radius
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

pub fn redraw_all_tiles(
    mut commands: Commands,
    dash: Option<Res<DashboardRes>>,
    hovered: Res<HoveredTile>,
    mut views: ResMut<TileViews>,
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

    // Ensure views vec has enough capacity
    while views.v2.len() < tiles.len() {
        views.v2.push(View2D::default());
    }

    // Auto-fit plots to content area
    for spec in &specs {
        let ti = spec.tile_idx;
        let tile = &tiles[ti];

        if let TileKind::TwoD { g2_idx } = spec.kind {
            if let Some(graph) = g2_list.get(g2_idx) {
                if let Some(bounds) = graph.bounds() {
                    let mut view = views.v2[ti];
                    // Only auto-fit if view is at default (not user-modified)
                    if view.scale == 1.0 && view.offset == Vec2::ZERO {
                        view.fit_bounds(bounds, tile.content_size);
                        views.v2[ti] = view;
                    }
                }
            }
        }
    }

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
                    view,
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
                    View2D::default(),
                    layer.clone(),
                );
            }

            TileKind::ThreeD { g3_idx } => {
                super::draw3d::draw_3d_axes(
                    &mut commands,
                    &mut meshes,
                    &mut materials_3d,
                    layer.clone(),
                );

                if let Some(graph) = g3_list.get(g3_idx) {
                    for layer_data in &graph.layers {
                        super::draw3d::draw_layer_3d(
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
