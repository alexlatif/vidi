use vidi::core::{Color, Colormap, Style};
use vidi::prelude::*;

fn main() {
    dash()
        .background_color(Color::BLACK)
        // Tab 1: Tier 1 charts - Candlestick, Heatmap, Boxplot
        .add_tab("Tier 1", |t| {
            t.columns(2)
                // Candlestick chart
                .add_candlestick(|c| {
                    let candles = generate_ohlc_data(50);
                    c.data(candles)
                        .title("Stock Price Movement")
                        .description("50-day OHLC data with random walk simulation")
                        .x_label("Trading Day")
                        .y_label("Price ($)")
                })
                // Correlation Heatmap
                .add_heatmap(|h| {
                    let (corr_matrix, labels) = generate_correlation_matrix();
                    h.from_2d(corr_matrix)
                        .title("Asset Correlation Matrix")
                        .description("Pairwise correlations between major ETFs")
                        .row_labels(labels.clone())
                        .col_labels(labels)
                        .colormap(Colormap::Coolwarm)
                        .vmin(-1.0)
                        .vmax(1.0)
                        .show_values(true)
                })
                // Box plot
                .add_distribution(|d| {
                    let groups = generate_strategy_returns();
                    d.boxplot(groups)
                        .title("Strategy Return Distributions")
                        .description("Daily returns by investment strategy")
                        .style(Style {
                            color: Color::rgb(0.3, 0.6, 0.9),
                            size: 1.0,
                            opacity: 0.8,
                        })
                        .x_label("Strategy")
                        .y_label("Daily Return (%)")
                })
        })
        // Tab 2: Tier 2 charts - ECDF, Drawdown, Bubble
        .add_tab("Tier 2", |t| {
            t.columns(2)
                // ECDF - Empirical Cumulative Distribution Function
                .add_distribution(|d| {
                    let returns = generate_return_distribution();
                    d.ecdf(returns)
                        .title("Cumulative Return Distribution")
                        .description("ECDF of simulated portfolio returns")
                        .style(Style {
                            color: Color::rgb(0.3, 0.7, 0.9),
                            size: 2.0,
                            opacity: 0.9,
                        })
                        .x_label("Return (%)")
                        .y_label("F(x)")
                })
                // Drawdown chart (line with fill_between)
                .add_2d(|p| {
                    let (cummax, drawdown, _zero_line) = generate_drawdown_data();
                    p.fill_between(
                        cummax.clone(),
                        drawdown.clone(),
                        Style {
                            color: Color::rgb(0.9, 0.3, 0.3),
                            size: 1.0,
                            opacity: 0.4,
                        },
                    )
                    .line(cummax, Style::default().rgb(0.4, 0.8, 0.4))
                    .line(drawdown, Style::default().rgb(0.9, 0.3, 0.3))
                    .title("Portfolio Drawdown")
                    .description("Current value vs cumulative maximum")
                    .x_label("Time")
                    .y_label("Value")
                })
                // Bubble chart (scatter with variable sizes)
                .add_2d(|p| {
                    let (xy, sizes) = generate_bubble_data();
                    p.bubble(
                        xy,
                        sizes,
                        Style {
                            color: Color::rgb(0.4, 0.6, 0.9),
                            size: 1.0,
                            opacity: 0.7,
                        },
                    )
                    .title("Risk vs Return")
                    .description("Fund comparison by market cap (bubble size)")
                    .x_label("Risk (Volatility)")
                    .y_label("Return (%)")
                })
        })
        // Tab 3: Tier 3 charts - Radar, Pie
        .add_tab("Tier 3", |t| {
            t.columns(2)
                // Radar chart - Portfolio metrics
                .add_radial(|r| {
                    r.radar(
                        vec!["Sharpe", "Sortino", "Calmar", "Win Rate", "Profit Factor"],
                        vec![0.75, 0.82, 0.65, 0.68, 0.78],
                    )
                    .title("Strategy Performance Metrics")
                    .description("Normalized scores (0-1) across key indicators")
                    .style(Style {
                        color: Color::rgb(0.3, 0.7, 0.9),
                        size: 2.0,
                        opacity: 0.8,
                    })
                })
                // Pie chart - Portfolio allocation
                .add_radial(|r| {
                    r.pie(vec![
                        ("Stocks", 45.0),
                        ("Bonds", 25.0),
                        ("Real Estate", 15.0),
                        ("Commodities", 10.0),
                        ("Cash", 5.0),
                    ])
                    .title("Portfolio Allocation")
                    .description("Current asset class distribution")
                })
        })
        .show();

    println!("App running - check if tab bar is visible");
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
        let drift = (rng() - 0.48) * 0.5;

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
        ("Momentum", 0.08, 1.2),
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

/// Generate return distribution for histogram/PDF/ECDF
fn generate_return_distribution() -> Vec<f32> {
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

    let mut rng = Rng(99999);
    (0..500).map(|_| rng.normal(0.05, 1.5)).collect()
}

/// Generate drawdown data (cumulative max and current value)
fn generate_drawdown_data() -> (
    Vec<bevy_math::Vec2>,
    Vec<bevy_math::Vec2>,
    Vec<bevy_math::Vec2>,
) {
    let mut seed = 33333u64;
    let mut rng = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed as f32 / u64::MAX as f32) * 2.0 - 1.0
    };

    let n = 100;
    let mut cummax = Vec::with_capacity(n);
    let mut current = Vec::with_capacity(n);
    let mut zero_line = Vec::with_capacity(n);

    let mut value = 100.0;
    let mut max_value = value;

    for i in 0..n {
        let x = i as f32;

        // Random walk with slight upward drift
        value = value * (1.0 + rng() * 0.03 + 0.001);
        max_value = max_value.max(value);

        cummax.push(bevy_math::Vec2::new(x, max_value));
        current.push(bevy_math::Vec2::new(x, value));
        zero_line.push(bevy_math::Vec2::new(x, 100.0));
    }

    (cummax, current, zero_line)
}

/// Generate bubble chart data (xy positions and sizes)
fn generate_bubble_data() -> (Vec<bevy_math::Vec2>, Vec<f32>) {
    let mut seed = 55555u64;
    let mut rng = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as f32 / u64::MAX as f32
    };

    let n = 30;
    let mut xy = Vec::with_capacity(n);
    let mut sizes = Vec::with_capacity(n);

    for _ in 0..n {
        // x = risk (0-20), y = return (-5 to 15), size = market cap
        let risk = rng() * 18.0 + 2.0;
        let ret = rng() * 18.0 - 3.0 + risk * 0.3; // Higher risk -> higher expected return
        let size = rng() * 15.0 + 5.0; // Size between 5 and 20

        xy.push(bevy_math::Vec2::new(risk, ret));
        sizes.push(size);
    }

    (xy, sizes)
}
