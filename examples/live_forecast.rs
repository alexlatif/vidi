//! Live ML Forecasting Experiment
//!
//! This example demonstrates:
//! - Posting a dashboard to vidi-server with `run_web()`
//! - Background task that updates forecast predictions in real-time
//! - Multiple tabs with different visualization types
//! - 5-minute TTL dashboard that auto-expires
//!
//! Run the server first:
//!   cargo run -p vidi-server
//!
//! Then run this example:
//!   cargo run --example live_forecast

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use glam::{Vec2, Vec3};
use vidi::core::{Color, Colormap, Style};
use vidi::prelude::*;

/// Plot IDs for updating specific plots
/// These match the order plots are added to the dashboard
const FORECAST_PLOT_ID: u64 = 1; // First plot added gets ID 1

fn main() {
    println!("=== Live ML Forecasting Experiment ===\n");

    // Server URL (default vidi-server)
    let server_url =
        std::env::var("VIDI_SERVER").unwrap_or_else(|_| "http://localhost:8080".to_string());

    println!("Connecting to vidi-server at: {}", server_url);

    // Post to server with 5-minute TTL
    let config = WebConfig::new()
        .xp_name("transformer-forecast-v2")
        .user("ml-team")
        .tag("forecast")
        .tag("live")
        .tag("lstm")
        .ttl(300); // 5 minutes

    println!("Posting dashboard to server...");

    let web_dash = match dash()
        .add_tab("Forecast", |t| build_forecast_tab(t))
        .add_tab("Metrics", |t| build_metrics_tab(t))
        .add_tab("Loss Surface", |t| build_loss_surface_tab(t))
        .add_tab("Distributions", |t| build_distributions_tab(t))
        .run_web(&server_url, config)
    {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to post dashboard: {}", e);
            eprintln!("\nMake sure vidi-server is running:");
            eprintln!("  cargo run -p vidi-server");
            return;
        }
    };

    println!("\nDashboard created!");
    println!("  ID: {}", web_dash.id);
    println!("  URL: {}", web_dash.view_url);
    println!("  TTL: 5 minutes (will auto-expire)\n");

    // Flag to stop the background update thread
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    // Spawn background task to update forecast predictions
    println!("Starting live forecast updates (Ctrl+C to stop)...\n");

    let server_url_clone = server_url.clone();
    let dashboard_id = web_dash.id.clone();

    let update_thread = thread::spawn(move || {
        simulate_live_forecast(&server_url_clone, &dashboard_id, running_clone);
    });

    // Wait for Ctrl+C
    ctrlc_wait();

    println!("\nStopping updates...");
    running.store(false, Ordering::SeqCst);
    let _ = update_thread.join();

    // Ask user if they want to delete the dashboard
    println!("\nDashboard will expire in ~5 minutes.");
    println!("Delete now? (y/n): ");

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() && input.trim().to_lowercase() == "y" {
        match web_dash.delete() {
            Ok(_) => println!("Dashboard deleted."),
            Err(e) => println!("Failed to delete: {}", e),
        }
    } else {
        println!("Dashboard left on server (will auto-expire).");
    }
}

/// Simulate live forecast updates
fn simulate_live_forecast(server_url: &str, dashboard_id: &str, running: Arc<AtomicBool>) {
    let client = ureq::Agent::new_with_defaults();
    let update_url = format!("{}/api/v1/dashboards/{}/update", server_url, dashboard_id);

    let mut step = 0u32;
    let mut seed = 42u64;

    // Simple PRNG
    let mut rng = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed as f32 / u64::MAX as f32) * 2.0 - 1.0
    };

    while running.load(Ordering::SeqCst) {
        // Generate new forecast point with some noise
        let t = step as f32 * 0.1;
        let base_forecast = (t * 0.3).sin() * 0.5 + t * 0.02;
        let noise = rng() * 0.05;
        let new_point = (t + 50.0, base_forecast + noise + 1.0); // Offset to forecast region

        // Append the new point to the forecast scatter plot (layer 1 = forecast points)
        let payload = serde_json::json!({
            "type": "append_points_2d",
            "plot_id": FORECAST_PLOT_ID,
            "layer_idx": 1, // Forecast scatter layer
            "points": [[new_point.0, new_point.1]],
        });

        let body_str = serde_json::to_string(&payload).unwrap();

        match client
            .post(&update_url)
            .content_type("application/json")
            .send(body_str.as_bytes())
        {
            Ok(_) => {
                if step % 10 == 0 {
                    println!(
                        "  Step {}: forecast point ({:.2}, {:.3})",
                        step, new_point.0, new_point.1
                    );
                }
            }
            Err(e) => {
                eprintln!("Update failed: {} (dashboard may have expired)", e);
                break;
            }
        }

        step += 1;
        thread::sleep(Duration::from_millis(500)); // Update every 500ms
    }

    println!("Forecast simulation stopped at step {}", step);
}

/// Build the Forecast tab with live-updating scatter plot
fn build_forecast_tab(tab: TabBuilder) -> TabBuilder {
    // Generate historical data
    let history: Vec<Vec2> = (0..500)
        .map(|i| {
            let t = i as f32 * 0.1;
            let y = (t * 0.3).sin() * 0.5 + t * 0.02 + pseudo_noise(i) * 0.1;
            Vec2::new(t, y)
        })
        .collect();

    // Initial forecast (will be updated live)
    let forecast_initial: Vec<Vec2> = vec![Vec2::new(50.0, 1.0)]; // Single starting point

    // Generate confidence bands
    let (upper_95, lower_95) = generate_confidence_band(&history, 50.0, 2.0);
    let (upper_68, lower_68) = generate_confidence_band(&history, 50.0, 1.0);

    tab.columns(2)
        .add_2d(|p| {
            p
                // Historical data (blue line)
                .line(
                    history.clone(),
                    Style {
                        color: Color::rgb(0.2, 0.5, 0.9),
                        size: 2.0,
                        opacity: 1.0,
                    },
                )
                // Forecast scatter (orange points - will be updated)
                .scatter(
                    forecast_initial,
                    Style {
                        color: Color::rgb(1.0, 0.6, 0.2),
                        size: 4.0,
                        opacity: 0.9,
                    },
                )
                // 95% confidence band
                .fill_between(
                    upper_95,
                    lower_95,
                    Style {
                        color: Color::rgb(1.0, 0.6, 0.2),
                        size: 1.0,
                        opacity: 0.15,
                    },
                )
                // 68% confidence band
                .fill_between(
                    upper_68,
                    lower_68,
                    Style {
                        color: Color::rgb(1.0, 0.6, 0.2),
                        size: 1.0,
                        opacity: 0.25,
                    },
                )
                .title("LSTM Forecast - Live Updates")
                .x_label("Time Step")
                .y_label("Value")
        })
        // Residuals plot
        .add_2d(|p| {
            let residuals: Vec<Vec2> = (0..100)
                .map(|i| {
                    let x = i as f32;
                    let y = pseudo_noise(i * 7) * 0.2;
                    Vec2::new(x, y)
                })
                .collect();

            p.scatter(
                residuals,
                Style {
                    color: Color::rgb(0.4, 0.8, 0.4),
                    size: 3.0,
                    opacity: 0.7,
                },
            )
            .line(
                vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)],
                Style {
                    color: Color::WHITE,
                    size: 1.0,
                    opacity: 0.5,
                },
            )
            .title("Forecast Residuals")
            .x_label("Sample")
            .y_label("Error")
        })
}

/// Build the Metrics tab with training curves
fn build_metrics_tab(tab: TabBuilder) -> TabBuilder {
    // Training loss curve
    let train_loss: Vec<Vec2> = (0..200)
        .map(|i| {
            let x = i as f32;
            let y = 2.0 * (-x * 0.02).exp() + 0.1 + pseudo_noise(i) * 0.05;
            Vec2::new(x, y)
        })
        .collect();

    // Validation loss curve
    let val_loss: Vec<Vec2> = (0..200)
        .map(|i| {
            let x = i as f32;
            let y = 2.2 * (-x * 0.018).exp() + 0.15 + pseudo_noise(i + 100) * 0.08;
            Vec2::new(x, y)
        })
        .collect();

    // Learning rate schedule
    let lr_schedule: Vec<Vec2> = (0..200)
        .map(|i| {
            let x = i as f32;
            let y = 0.001 * (0.95f32).powf(x / 10.0);
            Vec2::new(x, y)
        })
        .collect();

    tab.columns(2)
        .add_2d(|p| {
            p.line(
                train_loss,
                Style {
                    color: Color::rgb(0.3, 0.7, 1.0),
                    size: 2.0,
                    opacity: 1.0,
                },
            )
            .line(
                val_loss,
                Style {
                    color: Color::rgb(1.0, 0.4, 0.4),
                    size: 2.0,
                    opacity: 1.0,
                },
            )
            .title("Training Progress")
            .x_label("Epoch")
            .y_label("Loss (MSE)")
        })
        .add_2d(|p| {
            p.line(
                lr_schedule,
                Style {
                    color: Color::rgb(0.9, 0.7, 0.2),
                    size: 2.0,
                    opacity: 1.0,
                },
            )
            .title("Learning Rate Schedule")
            .x_label("Epoch")
            .y_label("LR")
        })
        // Attention heatmap
        .add_heatmap(|h| {
            let size = 12;
            let values: Vec<f32> = (0..size * size)
                .map(|i| {
                    let row = i / size;
                    let col = i % size;
                    // Diagonal attention pattern
                    let diag = (row as i32 - col as i32).abs() as f32;
                    (-diag * 0.3).exp() + pseudo_noise(i as u32) * 0.1
                })
                .collect();

            h.data(size, size, values)
                .colormap(Colormap::Blues)
                .title("Attention Weights")
        })
        // Metrics bar chart
        .add_2d(|p| {
            let metrics = vec![
                Vec2::new(0.0, 0.92), // Accuracy
                Vec2::new(1.0, 0.89), // Precision
                Vec2::new(2.0, 0.91), // Recall
                Vec2::new(3.0, 0.90), // F1
                Vec2::new(4.0, 0.85), // MAPE
            ];

            p.bars(
                metrics,
                Style {
                    color: Color::rgb(0.4, 0.8, 0.6),
                    size: 0.6,
                    opacity: 0.9,
                },
            )
            .title("Model Metrics")
            .x_label("Metric")
            .y_label("Score")
        })
}

/// Build the Loss Surface tab with 3D visualization
fn build_loss_surface_tab(tab: TabBuilder) -> TabBuilder {
    // Generate loss surface
    let nx = 40;
    let ny = 40;
    let mut surface_points = Vec::with_capacity(nx * ny);

    for j in 0..ny {
        for i in 0..nx {
            let x = (i as f32 / nx as f32) * 4.0 - 2.0;
            let y = (j as f32 / ny as f32) * 4.0 - 2.0;
            // Rosenbrock-like loss surface
            let z = (1.0 - x).powi(2) + 10.0 * (y - x.powi(2)).powi(2);
            let z = z.min(50.0) * 0.02; // Clamp and scale
            surface_points.push(Vec3::new(x, z, y));
        }
    }

    // Optimization trajectory
    let trajectory: Vec<Vec3> = (0..50)
        .map(|i| {
            let t = i as f32 / 50.0;
            let x = -1.5 + t * 2.5;
            let y = x.powi(2) + (1.0 - t) * 0.5;
            let z = (1.0 - x).powi(2) + 10.0 * (y - x.powi(2)).powi(2);
            let z = z.min(50.0) * 0.02;
            Vec3::new(x, z + 0.05, y) // Slightly above surface
        })
        .collect();

    tab.add_3d(|p| {
        p.surface(
            surface_points,
            nx as u32,
            ny as u32,
            Style {
                color: Color::rgb(0.3, 0.5, 0.9),
                size: 1.0,
                opacity: 0.7,
            },
        )
        .points(
            trajectory,
            Style {
                color: Color::rgb(1.0, 0.3, 0.3),
                size: 5.0,
                opacity: 1.0,
            },
        )
        .title("Loss Landscape + Optimizer Path")
        .x_label("Weight 1")
        .y_label("Loss")
        .z_label("Weight 2")
    })
}

/// Build the Distributions tab
fn build_distributions_tab(tab: TabBuilder) -> TabBuilder {
    // Prediction error distribution
    let errors: Vec<f32> = (0..500)
        .map(|i| pseudo_noise(i) * 0.3 + pseudo_noise(i + 1000) * 0.1)
        .collect();

    // Feature importance
    let feature_data = vec![
        ("lag_1", generate_samples(100, 0.8, 0.1)),
        ("lag_7", generate_samples(100, 0.6, 0.15)),
        ("trend", generate_samples(100, 0.4, 0.2)),
        ("season", generate_samples(100, 0.3, 0.12)),
    ];

    tab.columns(2)
        .add_distribution(|d| {
            d.histogram(errors.clone())
                .bins(30)
                .style(Style {
                    color: Color::rgb(0.5, 0.3, 0.8),
                    size: 1.0,
                    opacity: 0.8,
                })
                .title("Prediction Error Distribution")
                .x_label("Error")
                .y_label("Count")
        })
        .add_distribution(|d| {
            d.pdf(errors)
                .style(Style {
                    color: Color::rgb(0.8, 0.4, 0.6),
                    size: 2.0,
                    opacity: 1.0,
                })
                .title("Error Density (KDE)")
                .x_label("Error")
                .y_label("Density")
        })
        .add_distribution(|d| {
            d.boxplot(feature_data)
                .style(Style {
                    color: Color::rgb(0.4, 0.7, 0.9),
                    size: 1.0,
                    opacity: 0.9,
                })
                .title("Feature Importance by Lag")
                .x_label("Feature")
                .y_label("Importance")
        })
        .add_radial(|r| {
            r.radar(
                vec!["Accuracy", "Latency", "Memory", "Params", "FLOPs"],
                vec![0.92, 0.75, 0.6, 0.45, 0.55],
            )
            .title("Model Profile")
        })
}

// Helper functions

fn generate_confidence_band(
    history: &[Vec2],
    forecast_start: f32,
    sigma_mult: f32,
) -> (Vec<Vec2>, Vec<Vec2>) {
    let n_points = 60;
    let last_y = history.last().map(|p| p.y).unwrap_or(1.0);

    let upper: Vec<Vec2> = (0..n_points)
        .map(|i| {
            let t = forecast_start + i as f32;
            let mean = (t * 0.3).sin() * 0.5 + t * 0.02;
            let uncertainty = (i as f32 * 0.005) * sigma_mult;
            Vec2::new(t, mean + uncertainty + last_y - history.last().unwrap().y)
        })
        .collect();

    let lower: Vec<Vec2> = (0..n_points)
        .map(|i| {
            let t = forecast_start + i as f32;
            let mean = (t * 0.3).sin() * 0.5 + t * 0.02;
            let uncertainty = (i as f32 * 0.005) * sigma_mult;
            Vec2::new(t, mean - uncertainty + last_y - history.last().unwrap().y)
        })
        .collect();

    (upper, lower)
}

fn generate_samples(n: usize, mean: f32, std: f32) -> Vec<f32> {
    (0..n)
        .map(|i| mean + pseudo_noise(i as u32 + (mean * 1000.0) as u32) * std)
        .collect()
}

/// Simple deterministic pseudo-noise for reproducibility
fn pseudo_noise(i: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E3779B9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85EBCA6B);
    x ^= x >> 13;
    x = x.wrapping_mul(0xC2B2AE35);
    x ^= x >> 16;
    (x as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// Wait for Ctrl+C (simplified cross-platform)
fn ctrlc_wait() {
    println!("Press Enter to stop (or wait for dashboard to expire)...");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
}
