# Vidi

A high-performance data visualization library for Rust, powered by [Bevy](https://bevyengine.org/).

Vidi provides a declarative API for creating interactive 2D/3D plots and dashboards that run natively or in the browser via WebAssembly.

## Features

- **2D Charts**: Line plots, scatter plots, area charts, bar charts, bubble charts, fill-between regions
- **3D Visualization**: 3D scatter plots and surface plots with orbit controls
- **Statistical Plots**: Histograms, PDFs (kernel density estimation), box plots, ECDF
- **Financial Charts**: Candlestick/OHLC charts
- **Heatmaps**: 2D heatmaps with multiple colormaps
- **Radial Charts**: Pie charts and radar/spider charts
- **Interactive**: Pan, zoom, and rotate controls out of the box
- **Multi-plot Dashboards**: Grid layouts with tabs
- **Real-time Updates**: Stream data to dashboards via WebSocket
- **Dual Target**: Native desktop and WebAssembly (browser)

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
vidi = "0.1"
```

Create a simple dashboard:

```rust
use vidi::prelude::*;
use glam::Vec2;

fn main() {
    dash()
        .add_2d(|p| {
            let data: Vec<Vec2> = (0..100)
                .map(|i| {
                    let x = i as f32 * 0.1;
                    Vec2::new(x, x.sin())
                })
                .collect();

            p.line(data, None)
                .x_label("Time")
                .y_label("Amplitude")
                .title("Sine Wave")
        })
        .run_local();
}
```

Run with:

```bash
cargo run --example simple
```

## Examples

### Scatter Plot with Regression Line

```rust
dash()
    .add_2d(|p| {
        p.scatter(points, Style::default().color(Color::RED))
         .line(regression_line, Style::default().color(Color::BLUE))
    })
    .run_local();
```

### 3D Surface Plot

```rust
dash()
    .add_3d(|p| {
        let (xyz, nx, ny) = generate_surface();
        p.surface(xyz, nx, ny, None)
            .title("3D Surface")
    })
    .run_local();
```

### Histogram

```rust
dash()
    .add_distribution(|d| {
        d.histogram(samples)
            .bins(30)
            .style(Style::default().color(Color::GREEN))
    })
    .run_local();
```

### Multi-Plot Dashboard with Tabs

```rust
dash()
    .add_tab("Overview", |tab| {
        tab.add_2d(|p| p.line(data1, None))
           .add_2d(|p| p.scatter(data2, None))
    })
    .add_tab("Details", |tab| {
        tab.add_distribution(|d| d.histogram(samples))
           .add_heatmap(|h| h.from_2d(matrix))
    })
    .run_local();
```

## Web Dashboard (Server Mode)

Vidi includes a server component for hosting dashboards in the browser:

```rust
// Post dashboard to server and open in browser
let handle = dash()
    .add_2d(|p| p.line(data, None))
    .run_web("http://localhost:8080", WebConfig::default())?;

// Stream updates in real-time
handle.append_points_2d(plot_id, layer_idx, &new_points)?;
```

### Running the Server

```bash
# Build and run the server
cargo run -p vidi-server

# Or with Docker
docker build -t vidi-server .
docker run -p 8080:8080 vidi-server
```

## API Overview

### Dashboard Builder

```rust
dash()
    .background_color(Color::BLACK)  // Set background
    .columns(2)                       // Force 2-column layout
    .add_2d(|p| { ... })             // Add 2D plot
    .add_3d(|p| { ... })             // Add 3D plot
    .add_distribution(|d| { ... })   // Add histogram/PDF/boxplot
    .add_candlestick(|c| { ... })    // Add OHLC chart
    .add_heatmap(|h| { ... })        // Add heatmap
    .add_radial(|r| { ... })         // Add pie/radar chart
    .add_tab("Name", |t| { ... })    // Add tabbed section
    .run_local()                     // Run native window
```

### 2D Plot Builder

```rust
plot.line(points, style)           // Line chart
    .scatter(points, style)        // Scatter plot
    .area(points, style)           // Area chart
    .bars(points, style)           // Bar chart
    .bubble(points, sizes, style)  // Bubble chart
    .fill_between(upper, lower, style)  // Confidence bands
    .x_label("X Axis")
    .y_label("Y Axis")
    .title("Plot Title")
```

### 3D Plot Builder

```rust
plot.points(xyz, style)            // 3D scatter
    .surface(xyz, nx, ny, style)   // 3D surface mesh
    .x_label("X").y_label("Y").z_label("Z")
```

### Distribution Builder

```rust
dist.histogram(values).bins(30)    // Histogram
    .pdf(values)                   // Probability density
    .boxplot(groups)               // Box plot
    .ecdf(values)                  // Empirical CDF
```

### Style

```rust
Style {
    color: Color::rgb(1.0, 0.0, 0.0),  // RGB color
    size: 2.0,                          // Line width / point size
    opacity: 0.8,                       // Transparency
}
```

## Controls

| Action | 2D Plots | 3D Plots |
|--------|----------|----------|
| Pan | Click + Drag | - |
| Zoom | Scroll wheel | Scroll wheel |
| Rotate | - | Click + Drag |
| Reset | Double-click | Double-click |

## Architecture

Vidi is built on the [Bevy](https://bevyengine.org/) game engine, which provides:

- GPU-accelerated rendering via wgpu
- Cross-platform support (Windows, macOS, Linux, Web)
- Entity Component System (ECS) for efficient updates
- Hot-reloading and developer ergonomics

## Requirements

- Rust 1.85+ (edition 2024)
- For native: GPU with Vulkan/Metal/DX12 support
- For web: Modern browser with WebGL2/WebGPU

## License

MIT OR Apache-2.0

## Contributing

Contributions welcome! Please open an issue or PR on GitHub.
