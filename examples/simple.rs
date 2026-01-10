use glam::Vec2;
use vidi::core::Color;
use vidi::prelude::*;

fn main() {
    dash()
        .background_color(Color::BLACK)
        .add_2d(|p| {
            p.line(
                generate_sine_wave(),
                Style {
                    color: Color::BLUE,
                    size: 2.5,
                    opacity: 1.0,
                },
            )
            .scatter(
                generate_noisy_data(),
                Style {
                    color: Color::RED,
                    size: 2.0,
                    opacity: 0.55,
                },
            )
        })
        .add_2d(|p| {
            p.line(
                generate_sine_wave(),
                Style {
                    color: Color::BLUE,
                    size: 2.5,
                    opacity: 1.0,
                },
            )
            .scatter(
                generate_noisy_data(),
                Style {
                    color: Color::RED,
                    size: 2.0,
                    opacity: 0.55,
                },
            )
        })
        .show();
    // .show_local();
    // .show_online(); // uploads to xp server, runs dash for lifetime
    // .show_background(pool); // runs in server process to host dash wasms, in dashboard lib, with online data changes
    // dash_web(id).update(|p| { ... }); // for wasm32 target
}

fn generate_sine_wave() -> Vec<Vec2> {
    (0..200)
        .map(|i| {
            let x = i as f32 * 0.05;
            Vec2::new(x, x.sin())
        })
        .collect()
}

fn generate_noisy_data() -> Vec<Vec2> {
    let mut seed = 12345u64;
    let mut rng = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed as f32 / u64::MAX as f32) * 2.0 - 1.0
    };

    (0..300)
        .map(|i| {
            let x = i as f32 * 0.05;
            let noise = rng() * 0.2;
            Vec2::new(x, x.sin() + noise)
        })
        .collect()
}
