use crate::prelude::components::PlotId;
use bevy_math::{UVec2, Vec2, Vec3};
use serde::{Deserialize, Serialize};

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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Graph2D {
    pub id: PlotId,
    pub layers: Vec<Layer2D>,
    pub x_scale: Scale,
    pub y_scale: Scale,
    pub interaction: Interaction,
}

impl Graph2D {
    pub fn new() -> Self {
        Self {
            id: PlotId::new(),
            layers: vec![],
            x_scale: Scale::default(),
            y_scale: Scale::default(),
            interaction: Interaction::default(),
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
        }
        any.then_some((min, max))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Geometry2D {
    Line,
    Points,
    Area,
    Bars,  // interpret xy as (x, y) heights
    Stems, // vertical from baseline to y
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer2D {
    pub geometry: Geometry2D,
    pub xy: Vec<Vec2>,
    pub style: Style,
}

impl Layer2D {
    pub fn new(geometry: Geometry2D, xy: Vec<Vec2>) -> Self {
        Self {
            geometry,
            xy,
            style: Style::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Graph3D {
    pub id: PlotId,
    pub layers: Vec<Layer3D>,
    pub interaction: Interaction,
}

impl Graph3D {
    pub fn new() -> Self {
        Self {
            id: PlotId::new(),
            layers: vec![],
            interaction: Interaction {
                rotate: true,
                ..Interaction::default()
            },
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
        values: Vec<f32>,
        bins: usize,
        style: Style,
    },
    // KDE { bandwidth: f64 },
    // Box,
    // Violin,
    // ECDF,
    // leave room for KDE/Box/Violin later; lowering can expand
}

// Field plots (heatmap, image, tensors)
// heatmaps, attention matrices, correlation matrices, scalar fields
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
    pub dims: UVec2,      // (nx, ny)
    pub values: Vec<f32>, // nx*ny
    pub vmin: f32,
    pub vmax: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Radial {
    Pie {
        slices: Vec<(String, f32)>, // label, value
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dashboard {
    pub background: Color,
    pub plots: Vec<Plot>,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self {
            background: Color::rgba(0.05, 0.05, 0.09, 1.0),
            plots: vec![],
        }
    }
}
