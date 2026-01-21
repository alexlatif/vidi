use vidi::core::{Color, Colormap, Style};
use vidi::prelude::*;

fn main() {
    dash()
        .background_color(Color::BLACK)
        .columns(2) // 2 plots per row
        // Candlestick chart - simulated OHLC data (uses default green/red colors)
        .add_candlestick(|c| {
            let candles = generate_ohlc_data(50);
            c.data(candles).x_label("Trading Day").y_label("Price ($)")
        })
        // Correlation Heatmap
        .add_heatmap(|h| {
            let (corr_matrix, labels) = generate_correlation_matrix();
            h.from_2d(corr_matrix)
                .row_labels(labels.clone())
                .col_labels(labels)
                .colormap(Colormap::Coolwarm)
                .vmin(-1.0)
                .vmax(1.0)
                .show_values(true)
        })
        // Box plot comparing strategy returns
        .add_distribution(|d| {
            let groups = generate_strategy_returns();
            d.boxplot(groups)
                .style(Style {
                    color: Color::rgb(0.3, 0.6, 0.9),
                    size: 1.0,
                    opacity: 0.8,
                })
                .x_label("Strategy")
                .y_label("Daily Return (%)")
        })
        .show();
}

/// Generate simulated OHLC candlestick data with random walk
fn generate_ohlc_data(n: usize) -> Vec<(f32, f32, f32, f32, f32)> {
    let mut seed = 12345u64;
    let mut rng = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as f32 / u64::MAX as f32
    };

    let mut price = 100.0;
    let mut candles = Vec::with_capacity(n);

    for i in 0..n {
        let volatility = 2.0 + rng() * 1.5;
        let drift = (rng() - 0.48) * 0.5; // slight upward bias

        let open = price;
        let change = drift + (rng() - 0.5) * volatility;
        let close = (open + change).max(1.0);

        let high = open.max(close) + rng() * volatility * 0.5;
        let low = (open.min(close) - rng() * volatility * 0.5).max(0.5);

        candles.push((i as f32, open, high, low, close));
        price = close;
    }

    candles
}

/// Generate a correlation matrix for assets
fn generate_correlation_matrix() -> (Vec<Vec<f32>>, Vec<String>) {
    let labels = vec![
        "SPY".to_string(),
        "QQQ".to_string(),
        "IWM".to_string(),
        "TLT".to_string(),
        "GLD".to_string(),
        "VIX".to_string(),
    ];

    // Realistic-ish correlation matrix (symmetric, diagonal = 1)
    let corr = vec![
        vec![1.00, 0.92, 0.88, -0.35, 0.05, -0.75],
        vec![0.92, 1.00, 0.82, -0.40, -0.02, -0.70],
        vec![0.88, 0.82, 1.00, -0.30, 0.08, -0.65],
        vec![-0.35, -0.40, -0.30, 1.00, 0.20, 0.15],
        vec![0.05, -0.02, 0.08, 0.20, 1.00, 0.10],
        vec![-0.75, -0.70, -0.65, 0.15, 0.10, 1.00],
    ];

    (corr, labels)
}

/// Generate simulated daily returns for different strategies
fn generate_strategy_returns() -> Vec<(String, Vec<f32>)> {
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 as f32 / u64::MAX as f32
        }

        fn normal(&mut self, mean: f32, std: f32) -> f32 {
            let u1 = self.next().max(1e-10);
            let u2 = self.next();
            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * std::f32::consts::PI * u2;
            mean + std * r * theta.cos()
        }
    }

    let mut rng = Rng(54321);

    let strategies = vec![
        ("Momentum", 0.08, 1.2), // mean, std
        ("Value", 0.05, 0.9),
        ("Quality", 0.06, 0.7),
        ("Low Vol", 0.04, 0.5),
        ("Market", 0.07, 1.0),
    ];

    strategies
        .into_iter()
        .map(|(name, mean, std)| {
            let returns: Vec<f32> = (0..200)
                .map(|_| {
                    let mut r = rng.normal(mean, std);
                    // Add some outliers
                    if rng.next() < 0.02 {
                        r += if rng.next() > 0.5 { 3.0 } else { -3.5 } * std;
                    }
                    r
                })
                .collect();
            (name.to_string(), returns)
        })
        .collect()
}
