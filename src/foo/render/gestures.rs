use bevy::prelude::*;

use super::resources::{DashboardRes, HoveredTile, TileViews};
use super::specs::TileCam3D;
use super::specs::{TileKind, build_tile_specs};

#[allow(deprecated)]
pub fn handle_gestures(
    mut wheel_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    dash: Option<Res<DashboardRes>>,
    hovered: Res<HoveredTile>,
    mut views: ResMut<TileViews>,
    mut cam3_query: Query<(&super::specs::TileCam, &mut Transform), With<TileCam3D>>,
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

            // update the camera transform for THIS tile
            for (tc, mut transform) in cam3_query.iter_mut() {
                if tc.tile_idx == ti {
                    let cy = view.yaw.cos();
                    let sy = view.yaw.sin();
                    let cp = view.pitch.cos();
                    let sp = view.pitch.sin();

                    let dir = Vec3::new(sy * cp, sp, cy * cp);
                    let pos = view.target + dir * view.radius;
                    *transform = Transform::from_translation(pos).looking_at(view.target, Vec3::Y);
                }
            }
        }
    }
}
