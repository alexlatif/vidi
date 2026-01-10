//! AI Loss Landscape Visualization
//!
//! This example demonstrates 3D visualization for machine learning analysis:
//! - 3D Surface: sin(x)*cos(z) mathematical surface with smooth shading
//! - 3D Scatter: Data points in 3D space with axes and grid
//! - 2D charts for comparison

use bevy_math::{Vec2, Vec3};
use vidi::core::{Color, Style};
use vidi::prelude::*;

fn main() {
    dash()
        .background_color(Color::BLACK)
        .columns(2)
        // 3D Surface: sin(x) * cos(z) - classic mathematical surface
        .add_3d(|p| {
            let (surface_xyz, nx, ny) = generate_sincos_surface();
            p.surface(
                surface_xyz,
                nx,
                ny,
                Style {
                    color: Color::rgb(0.0, 0.7, 0.9),
                    size: 1.0,
                    opacity: 0.95,
                },
            )
            .title("Loss Landscape")
            .description("Loss vs hyperparameters")
            .x_label("Learning Rate")
            .y_label("Loss")
            .z_label("Momentum")
        })
        // 3D Scatter: Random 3D point cloud with visible points
        .add_3d(|p| {
            let points = generate_3d_scatter();
            p.points(
                points,
                Style {
                    color: Color::rgb(1.0, 0.5, 0.2),
                    size: 5.0, // Larger points
                    opacity: 1.0,
                },
            )
            .title("Training Trajectory")
            .description("Optimizer path through loss landscape")
            .x_label("Param A")
            .y_label("Param B")
            .z_label("Param C")
        })
        // 2D Learning Curve
        .add_2d(|p| {
            let loss_curve = generate_learning_curve();
            p.line(
                loss_curve.clone(),
                Style {
                    color: Color::rgb(0.2, 0.9, 0.4),
                    size: 2.0,
                    opacity: 1.0,
                },
            )
            .scatter(
                loss_curve.iter().step_by(10).cloned().collect(),
                Style {
                    color: Color::rgb(0.9, 0.9, 0.2),
                    size: 4.0,
                    opacity: 0.8,
                },
            )
            .title("Training Loss")
            .description("Loss vs epoch")
            .x_label("Epoch")
            .y_label("Loss")
        })
        // 2D Scatter
        .add_2d(|p| {
            let params = generate_2d_scatter();
            p.scatter(
                params,
                Style {
                    color: Color::rgb(0.6, 0.3, 0.9),
                    size: 4.0,
                    opacity: 0.7,
                },
            )
            .title("2D Scatter")
            .description("Random point distribution")
            .x_label("X")
            .y_label("Y")
        })
        .show();
}

/// Generate sin(x) * cos(z) surface - classic 3D math visualization
fn generate_sincos_surface() -> (Vec<Vec3>, u32, u32) {
    let nx = 80;
    let ny = 80;

    let x_min = -6.0_f32;
    let x_max = 6.0_f32;
    let z_min = -6.0_f32;
    let z_max = 6.0_f32;

    let mut xyz = Vec::with_capacity(nx * ny);

    for iy in 0..ny {
        let ty = iy as f32 / (ny as f32 - 1.0);
        let z = z_min + (z_max - z_min) * ty;

        for ix in 0..nx {
            let tx = ix as f32 / (nx as f32 - 1.0);
            let x = x_min + (x_max - x_min) * tx;

            // z = sin(x) * cos(z) - note: using z for the depth coordinate
            let y = x.sin() * z.cos();
            xyz.push(Vec3::new(x, y, z));
        }
    }

    (xyz, nx as u32, ny as u32)
}

/// Generate 3D scatter points
fn generate_3d_scatter() -> Vec<Vec3> {
    let mut seed = 12345u64;
    let mut rng = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as f32 / u64::MAX as f32
    };

    let n = 150;
    let mut points = Vec::with_capacity(n);

    for _ in 0..n {
        // Random points in a [-4, 4] cube
        let x = rng() * 8.0 - 4.0;
        let y = rng() * 4.0 - 1.0; // Shorter in Y
        let z = rng() * 8.0 - 4.0;
        points.push(Vec3::new(x, y, z));
    }

    points
}

/// Generate training loss curve over epochs
fn generate_learning_curve() -> Vec<Vec2> {
    let n_epochs = 100;
    let mut seed = 42u64;
    let mut rng = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed as f32 / u64::MAX as f32) * 2.0 - 1.0
    };

    (0..n_epochs)
        .map(|epoch| {
            let t = epoch as f32;
            let base_loss = 2.0 * (-t / 30.0).exp() + 0.1;
            let noise = rng() * 0.05 * base_loss;
            Vec2::new(t, (base_loss + noise).max(0.05))
        })
        .collect()
}

/// Generate 2D scatter points
fn generate_2d_scatter() -> Vec<Vec2> {
    let mut seed = 54321u64;
    let mut rng = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as f32 / u64::MAX as f32
    };

    let n = 100;
    (0..n)
        .map(|_| {
            let x = rng() * 10.0;
            let y = rng() * 10.0;
            Vec2::new(x, y)
        })
        .collect()
}
