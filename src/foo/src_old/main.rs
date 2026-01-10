mod plot;
mod render;
mod utils;

use bevy::prelude::*;
use bevy_math::{Vec2, Vec3};
use plot::{Geometry2D, Graph2D, Graph3D, Layer2D, Plot};
use render::{PlotRenderPlugin, PlotScene};
use utils::{Color as VidiColor, Style};

fn main() {
    scatter_2d();
    // surface_3d();
}

#[allow(dead_code)]
fn scatter_2d() {
    // f(x) = sin(x)
    let mut line_xy = Vec::new();
    let mut x = -6.0_f32;
    while x <= 6.0 {
        line_xy.push(Vec2::new(x, x.sin()));
        x += 0.02;
    }

    // scatter around sin(x)
    let mut seed: u64 = 0xC0FFEE;
    let mut rnd = || {
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        ((seed.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32) as f32 / (u32::MAX as f32 + 1.0)
    };

    let mut scatter_xy = Vec::with_capacity(900);
    for _ in 0..900 {
        let x0 = -6.0 + 12.0 * rnd();
        let noise_x = (rnd() + rnd()) - 1.0;
        let noise_y = (rnd() + rnd()) - 1.0;
        let xj = x0 + noise_x * 0.04;
        let yj = x0.sin() + noise_y * 0.18;
        scatter_xy.push(Vec2::new(xj, yj));
    }

    let mut g = Graph2D::line(line_xy);
    g.layers[0].style = Style {
        color: Some(VidiColor::Named("cyan")),
        size: Some(2.5),
        opacity: Some(1.0),
    };

    g.layers.push(Layer2D {
        geometry: Geometry2D::Points,
        xy: scatter_xy,
        style: Style {
            color: Some(VidiColor::Named("deepskyblue")),
            size: Some(2.0),
            opacity: Some(0.55),
        },
        labels: vec![],
    });

    let scene = PlotScene::single(Plot::Graph2D(g));

    App::new()
        .insert_resource(ClearColor(bevy::prelude::Color::srgb(0.05, 0.05, 0.09)))
        .insert_resource(scene)
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            PlotRenderPlugin,
        ))
        .run();
}

fn surface_3d() {
    // Surface demo: z = sin(x) * cos(y)
    let nx: usize = 160;
    let ny: usize = 120;

    let x_min = -6.0_f32;
    let x_max = 6.0_f32;
    let y_min = -5.0_f32;
    let y_max = 5.0_f32;

    let mut xyz = Vec::with_capacity(nx * ny);

    for iy in 0..ny {
        let ty = iy as f32 / (ny as f32 - 1.0);
        let y = y_min + (y_max - y_min) * ty;

        for ix in 0..nx {
            let tx = ix as f32 / (nx as f32 - 1.0);
            let x = x_min + (x_max - x_min) * tx;

            let z = x.sin() * y.cos();
            xyz.push(Vec3::new(x, z, y)); // note: (x, height, y) reads nicely in 3D
        }
    }

    let mut g = Graph3D::surface(xyz, nx as u32, ny as u32);
    g.layers[0].style = Style {
        color: Some(VidiColor::Named("deepskyblue")),
        size: None,
        opacity: Some(1.0),
    };

    let scene = PlotScene::single(Plot::Graph3D(g));

    App::new()
        .insert_resource(ClearColor(bevy::prelude::Color::srgb(0.05, 0.05, 0.09)))
        .insert_resource(scene)
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            PlotRenderPlugin,
        ))
        .run();
}
