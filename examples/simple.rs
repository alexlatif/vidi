use glam::Vec2;
use vidi::core::{Color, Style};
use vidi::prelude::*;

fn main() {
    dash()
        .background_color(Color::BLACK)
        // Left plot: Scatter with regression line
        .add_2d(|p| {
            let noisy_data = generate_noisy_data();
            let regression_line = compute_regression(&noisy_data);

            p.scatter(
                noisy_data,
                Style {
                    color: Color::RED,
                    size: 3.0,
                    opacity: 0.6,
                },
            )
            .line(
                regression_line,
                Style {
                    color: Color::BLUE,
                    size: 2.5,
                    opacity: 1.0,
                },
            )
            .x_label("Time (s)")
            .y_label("Amplitude")
        })
        // Right plot: Historical + Forecast with layered confidence bands
        .add_2d(|p| {
            let (history, mean_forecast, bands) = generate_forecast_data();

            // Historical line (realized values up to "now")
            let mut plot = p.line(
                history,
                Style {
                    color: Color::BLUE,
                    size: 2.5,
                    opacity: 1.0,
                },
            );

            // Render bands from outermost to innermost (so inner renders on top)
            // Outer bands are more transparent, inner bands more opaque
            let band_styles = [
                (0.15, Color::rgb(0.5, 0.7, 1.0)),  // outermost - very light
                (0.25, Color::rgb(0.4, 0.6, 0.95)),
                (0.4, Color::rgb(0.3, 0.5, 0.9)),
                (0.6, Color::rgb(0.2, 0.4, 0.85)),  // innermost - more saturated
            ];

            for (i, (upper, lower)) in bands.iter().rev().enumerate() {
                let (opacity, color) = band_styles[i];
                plot = plot.fill_between(
                    upper.clone(),
                    lower.clone(),
                    Style {
                        color,
                        size: 1.0,
                        opacity,
                    },
                );
            }

            // Mean forecast line on top
            plot.line(
                mean_forecast,
                Style {
                    color: Color::rgb(0.1, 0.3, 0.8),
                    size: 2.0,
                    opacity: 1.0,
                },
            )
            .x_label("Days")
            .y_label("Value")
        })
        // Third plot: Histogram
        .add_distribution(|d| {
            let samples = generate_normal_samples(1000, 0.0, 1.0);
            d.histogram(samples)
                .bins(25)
                .style(Style {
                    color: Color::rgb(0.2, 0.7, 0.4),
                    size: 1.0,
                    opacity: 0.8,
                })
                .x_label("Sample Value")
                .y_label("Count")
        })
        // Fourth plot: Probability Density Function (KDE)
        .add_distribution(|d| {
            let samples = generate_bimodal_samples(800);
            d.pdf(samples)
                .style(Style {
                    color: Color::rgb(0.8, 0.3, 0.5),
                    size: 2.0,
                    opacity: 1.0,
                })
                .x_label("Measurement")
                .y_label("Probability")
        })
        .show();
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
            let noise = rng() * 0.3;
            Vec2::new(x, x.sin() + noise)
        })
        .collect()
}

fn compute_regression(data: &[Vec2]) -> Vec<Vec2> {
    // Smooth sine curve through the mean of the noisy data
    // For simplicity, we just generate the underlying sine wave
    let x_min = data.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let x_max = data.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);

    (0..200)
        .map(|i| {
            let t = i as f32 / 199.0;
            let x = x_min + t * (x_max - x_min);
            Vec2::new(x, x.sin())
        })
        .collect()
}

/// Returns (history, mean_forecast, bands) where bands is vec of (upper, lower) pairs
/// ordered from innermost (highest confidence) to outermost (lowest confidence)
fn generate_forecast_data() -> (Vec<Vec2>, Vec<Vec2>, Vec<(Vec<Vec2>, Vec<Vec2>)>) {
    // Historical data: realized values up to "now" (x=0 to x=5)
    let history: Vec<Vec2> = (0..=50)
        .map(|i| {
            let x = i as f32 * 0.1;
            let y = (x * 0.5).sin() + x * 0.05;
            Vec2::new(x, y)
        })
        .collect();

    let last_history = history.last().unwrap();
    let forecast_start_x = last_history.x;
    let n_forecast = 60;

    // Mean forecast line
    let mean_forecast: Vec<Vec2> = (0..=n_forecast)
        .map(|i| {
            let x = forecast_start_x + i as f32 * 0.15;
            let y = (x * 0.5).sin() + x * 0.05;
            Vec2::new(x, y)
        })
        .collect();

    // Generate bands at different confidence levels (multipliers for uncertainty)
    let band_multipliers = [0.5, 1.0, 1.5, 2.0]; // 50%, 68%, 86%, 95% roughly

    let bands: Vec<(Vec<Vec2>, Vec<Vec2>)> = band_multipliers
        .iter()
        .map(|&mult| {
            let upper: Vec<Vec2> = (0..=n_forecast)
                .map(|i| {
                    let x = forecast_start_x + i as f32 * 0.15;
                    let mean = (x * 0.5).sin() + x * 0.05;
                    let uncertainty = (x - forecast_start_x) * 0.03 * mult;
                    Vec2::new(x, mean + uncertainty)
                })
                .collect();

            let lower: Vec<Vec2> = (0..=n_forecast)
                .map(|i| {
                    let x = forecast_start_x + i as f32 * 0.15;
                    let mean = (x * 0.5).sin() + x * 0.05;
                    let uncertainty = (x - forecast_start_x) * 0.03 * mult;
                    Vec2::new(x, mean - uncertainty)
                })
                .collect();

            (upper, lower)
        })
        .collect();

    (history, mean_forecast, bands)
}

/// Generate normally distributed samples using Box-Muller transform
fn generate_normal_samples(n: usize, mean: f32, std_dev: f32) -> Vec<f32> {
    let mut seed = 54321u64;
    let mut rng = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed as f32 / u64::MAX as f32)
    };

    let mut samples = Vec::with_capacity(n);
    for _ in 0..(n / 2 + 1) {
        // Box-Muller transform
        let u1 = rng().max(1e-10);
        let u2 = rng();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;

        samples.push(mean + std_dev * r * theta.cos());
        if samples.len() < n {
            samples.push(mean + std_dev * r * theta.sin());
        }
    }
    samples.truncate(n);
    samples
}

/// Generate bimodal distribution (mixture of two normals)
fn generate_bimodal_samples(n: usize) -> Vec<f32> {
    let mut seed = 98765u64;
    let mut rng = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed as f32 / u64::MAX as f32)
    };

    let mut samples = Vec::with_capacity(n);
    for _ in 0..n {
        // 40% from first mode, 60% from second
        let (mean, std_dev) = if rng() < 0.4 {
            (-2.0, 0.8)
        } else {
            (2.5, 1.2)
        };

        // Box-Muller for single sample
        let u1 = rng().max(1e-10);
        let u2 = rng();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        samples.push(mean + std_dev * r * theta.cos());
    }
    samples
}
