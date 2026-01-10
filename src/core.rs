use crate::prelude::components::PlotId;
use bevy_math::{UVec2, Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Common metadata for all plot types
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlotMeta {
    /// Title displayed at the top of the plot
    pub title: Option<String>,
    /// Optional description displayed below the title
    pub description: Option<String>,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
// pub struct PlotId(pub u64);

// impl PlotId {
//     pub fn new() -> Self {
//         static CTR: AtomicU32 = AtomicU32::new(1);
//         Self(CTR.fetch_add(1, Ordering::Relaxed).into())
//     }
// }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
    pub const fn with_a(self, a: f32) -> Self {
        Self { a, ..self }
    }

    // Common named colors (keep it small; you can add more later)
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
}

impl From<Color> for bevy::prelude::Color {
    #[inline]
    fn from(c: Color) -> Self {
        bevy::prelude::Color::linear_rgba(c.r, c.g, c.b, c.a)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Style {
    pub color: Color,
    pub size: f32,    // line width / point radius / etc
    pub opacity: f32, // multiplied into alpha
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            size: 2.0,
            opacity: 1.0,
        }
    }
}

impl Style {
    #[inline]
    pub const fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }

    #[inline]
    pub const fn rgb(self, r: f32, g: f32, b: f32) -> Self {
        self.color(Color::rgb(r, g, b))
    }

    #[inline]
    pub const fn rgba(self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color(Color::rgba(r, g, b, a))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Scale {
    Linear,
    Log10,
    Symlog { lin_thresh: f64 },
    Power { exponent: f64 },
    Time,
    Categorical,
}

impl Default for Scale {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Interaction {
    pub pan: bool,
    pub zoom: bool,
    pub rotate: bool,
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            pan: true,
            zoom: true,
            rotate: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Plot {
    Graph2D(Graph2D),
    Graph3D(Graph3D),
    Distribution(Distribution),
    Field(Field),
    Radial(Radial),
    Candlestick(Candlestick),
    Heatmap(Heatmap),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Graph2D {
    pub id: PlotId,
    pub meta: PlotMeta,
    pub layers: Vec<Layer2D>,
    pub x_scale: Scale,
    pub y_scale: Scale,
    pub interaction: Interaction,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
}

impl Graph2D {
    pub fn new() -> Self {
        Self {
            id: PlotId::new(),
            meta: PlotMeta::default(),
            layers: vec![],
            x_scale: Scale::default(),
            y_scale: Scale::default(),
            interaction: Interaction::default(),
            x_label: None,
            y_label: None,
        }
    }

    pub fn with_layer(mut self, layer: Layer2D) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn bounds(&self) -> Option<([f32; 2], [f32; 2])> {
        let mut min = [f32::INFINITY; 2];
        let mut max = [f32::NEG_INFINITY; 2];
        let mut any = false;
        for l in &self.layers {
            for p in &l.xy {
                if !p.x.is_finite() || !p.y.is_finite() {
                    continue;
                }
                min[0] = min[0].min(p.x);
                min[1] = min[1].min(p.y);
                max[0] = max[0].max(p.x);
                max[1] = max[1].max(p.y);
                any = true;
            }
            // Also consider lower_line for FillBetween geometry
            if let Some(lower) = &l.lower_line {
                for p in lower {
                    if !p.x.is_finite() || !p.y.is_finite() {
                        continue;
                    }
                    min[0] = min[0].min(p.x);
                    min[1] = min[1].min(p.y);
                    max[0] = max[0].max(p.x);
                    max[1] = max[1].max(p.y);
                    any = true;
                }
            }
        }
        any.then_some((min, max))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Geometry2D {
    Line,
    Points,
    Area,
    Bars,       // interpret xy as (x, y) heights
    Stems,      // vertical from baseline to y
    FillBetween, // fills area between two lines (xy = upper, lower_line = lower)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer2D {
    pub geometry: Geometry2D,
    pub xy: Vec<Vec2>,
    pub style: Style,
    /// For FillBetween geometry: the lower line (xy contains the upper line)
    pub lower_line: Option<Vec<Vec2>>,
    /// For bubble charts: size of each point (optional, uses style.size if None)
    pub sizes: Option<Vec<f32>>,
}

impl Layer2D {
    pub fn new(geometry: Geometry2D, xy: Vec<Vec2>) -> Self {
        Self {
            geometry,
            xy,
            style: Style::default(),
            lower_line: None,
            sizes: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Graph3D {
    pub id: PlotId,
    pub meta: PlotMeta,
    pub layers: Vec<Layer3D>,
    pub interaction: Interaction,
    /// X-axis label (e.g., "Learning Rate")
    pub x_label: Option<String>,
    /// Y-axis label (e.g., "Loss")
    pub y_label: Option<String>,
    /// Z-axis label (e.g., "Momentum")
    pub z_label: Option<String>,
}

impl Graph3D {
    pub fn new() -> Self {
        Self {
            id: PlotId::new(),
            meta: PlotMeta::default(),
            layers: vec![],
            interaction: Interaction {
                rotate: true,
                ..Interaction::default()
            },
            x_label: None,
            y_label: None,
            z_label: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Geometry3D {
    Points,
    Surface { grid: UVec2 }, // (nx, ny)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer3D {
    pub geometry: Geometry3D,
    pub xyz: Vec<Vec3>,
    pub style: Style,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Distribution {
    Histogram {
        meta: PlotMeta,
        values: Vec<f32>,
        bins: usize,
        style: Style,
        x_label: Option<String>,
        y_label: Option<String>,
    },
    Pdf {
        meta: PlotMeta,
        values: Vec<f32>,
        style: Style,
        x_label: Option<String>,
        y_label: Option<String>,
    },
    BoxPlot {
        meta: PlotMeta,
        /// Each group is (label, values)
        groups: Vec<(String, Vec<f32>)>,
        style: Style,
        x_label: Option<String>,
        y_label: Option<String>,
    },
    /// Empirical Cumulative Distribution Function (step function)
    ECDF {
        meta: PlotMeta,
        values: Vec<f32>,
        style: Style,
        x_label: Option<String>,
        y_label: Option<String>,
    },
}

// Field plots (heatmap, image, tensors)
// heatmaps, attention matrices, correlation matrices, scalar fields
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
    pub meta: PlotMeta,
    pub dims: UVec2,      // (nx, ny)
    pub values: Vec<f32>, // nx*ny
    pub vmin: f32,
    pub vmax: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Radial {
    Pie {
        meta: PlotMeta,
        slices: Vec<(String, f32)>, // label, value
    },
    Radar {
        meta: PlotMeta,
        axes: Vec<String>,    // axis labels
        values: Vec<f32>,     // values for each axis (0-1 normalized)
        style: Style,
    },
}

/// OHLC candlestick data for financial time series
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candlestick {
    pub meta: PlotMeta,
    pub candles: Vec<OHLC>,
    pub up_color: Color,
    pub down_color: Color,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
}

/// Single OHLC candle
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OHLC {
    pub x: f32,     // time/index
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
}

impl OHLC {
    pub fn new(x: f32, open: f32, high: f32, low: f32, close: f32) -> Self {
        Self { x, open, high, low, close }
    }
}

/// 2D heatmap with labeled axes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Heatmap {
    pub meta: PlotMeta,
    pub dims: UVec2,           // (cols, rows)
    pub values: Vec<f32>,      // row-major: values[row * cols + col]
    pub vmin: Option<f32>,     // auto if None
    pub vmax: Option<f32>,     // auto if None
    pub row_labels: Option<Vec<String>>,
    pub col_labels: Option<Vec<String>>,
    pub show_values: bool,     // show numeric values in cells
    pub colormap: Colormap,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub enum Colormap {
    #[default]
    Viridis,
    Plasma,
    Inferno,
    Magma,
    Coolwarm,   // diverging: blue-white-red
    RdBu,       // diverging: red-white-blue
    Blues,
    Reds,
    Greens,
}

impl Colormap {
    /// Map value in [0, 1] to RGB color
    pub fn sample(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        match self {
            Colormap::Viridis => Self::viridis(t),
            Colormap::Plasma => Self::plasma(t),
            Colormap::Inferno => Self::inferno(t),
            Colormap::Magma => Self::magma(t),
            Colormap::Coolwarm => Self::coolwarm(t),
            Colormap::RdBu => Self::rdbu(t),
            Colormap::Blues => Self::blues(t),
            Colormap::Reds => Self::reds(t),
            Colormap::Greens => Self::greens(t),
        }
    }

    // Clean, minimal colormaps using only red, green, blue with opacity/brightness variation

    fn viridis(t: f32) -> Color {
        // Blue gradient: light to saturated blue
        let opacity = 0.3 + t * 0.7;
        Color::rgba(0.2, 0.4, 0.9, opacity)
    }

    fn plasma(t: f32) -> Color {
        // Red gradient: light to saturated red
        let opacity = 0.3 + t * 0.7;
        Color::rgba(0.9, 0.2, 0.2, opacity)
    }

    fn inferno(t: f32) -> Color {
        // Green gradient: light to saturated green
        let opacity = 0.3 + t * 0.7;
        Color::rgba(0.2, 0.8, 0.3, opacity)
    }

    fn magma(t: f32) -> Color {
        // Blue to green transition
        let r = 0.2;
        let g = 0.3 + t * 0.5;
        let b = 0.9 - t * 0.6;
        Color::rgba(r, g, b, 0.5 + t * 0.4)
    }

    fn coolwarm(t: f32) -> Color {
        // Diverging: Blue (low) -> neutral -> Red (high)
        if t < 0.5 {
            let s = (0.5 - t) * 2.0; // 1 at t=0, 0 at t=0.5
            Color::rgba(0.2, 0.4, 0.9, 0.3 + s * 0.6) // Blue
        } else {
            let s = (t - 0.5) * 2.0; // 0 at t=0.5, 1 at t=1
            Color::rgba(0.9, 0.2, 0.2, 0.3 + s * 0.6) // Red
        }
    }

    fn rdbu(t: f32) -> Color {
        // Diverging: Red (low) -> neutral -> Blue (high)
        if t < 0.5 {
            let s = (0.5 - t) * 2.0;
            Color::rgba(0.9, 0.2, 0.2, 0.3 + s * 0.6) // Red
        } else {
            let s = (t - 0.5) * 2.0;
            Color::rgba(0.2, 0.4, 0.9, 0.3 + s * 0.6) // Blue
        }
    }

    fn blues(t: f32) -> Color {
        // Pure blue with varying opacity
        Color::rgba(0.2, 0.5, 1.0, 0.2 + t * 0.7)
    }

    fn reds(t: f32) -> Color {
        // Pure red with varying opacity
        Color::rgba(1.0, 0.25, 0.2, 0.2 + t * 0.7)
    }

    fn greens(t: f32) -> Color {
        // Pure green with varying opacity
        Color::rgba(0.2, 0.85, 0.35, 0.2 + t * 0.7)
    }
}

/// A tab containing a set of plots
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tab {
    pub name: String,
    pub plots: Vec<Plot>,
    pub columns: Option<usize>,
}

impl Tab {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            plots: vec![],
            columns: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dashboard {
    pub background: Color,
    /// Direct plots (when not using tabs)
    pub plots: Vec<Plot>,
    /// Number of columns per row (default: auto based on plot count)
    pub columns: Option<usize>,
    /// Tabs (alternative to direct plots)
    pub tabs: Vec<Tab>,
    /// Currently active tab index
    pub active_tab: usize,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self {
            background: Color::rgba(0.05, 0.05, 0.09, 1.0),
            plots: vec![],
            columns: None,
            tabs: vec![],
            active_tab: 0,
        }
    }
}

impl Dashboard {
    /// Returns true if this dashboard uses tabs
    pub fn has_tabs(&self) -> bool {
        !self.tabs.is_empty()
    }

    /// Get the active plots (from active tab if using tabs, otherwise direct plots)
    pub fn active_plots(&self) -> &[Plot] {
        if self.has_tabs() {
            self.tabs.get(self.active_tab)
                .map(|t| t.plots.as_slice())
                .unwrap_or(&[])
        } else {
            &self.plots
        }
    }

    /// Get the columns setting for the active view
    pub fn active_columns(&self) -> Option<usize> {
        if self.has_tabs() {
            self.tabs.get(self.active_tab)
                .and_then(|t| t.columns)
                .or(self.columns)
        } else {
            self.columns
        }
    }

    /// Get tab names for UI
    pub fn tab_names(&self) -> Vec<&str> {
        self.tabs.iter().map(|t| t.name.as_str()).collect()
    }
}
