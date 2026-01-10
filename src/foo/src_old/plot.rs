use crate::utils::{Interaction, Scale, Style};
use bevy_math::{UVec2, Vec2, Vec3};

#[derive(Clone, Debug)]
pub enum Plot {
    Graph2D(Graph2D),
    Graph3D(Graph3D),
    Distribution(Distribution),
    Field(Field),
    Radial(Radial),
    Composite(Vec<Plot>),
}

#[derive(Clone, Debug)]
pub struct Graph2D {
    pub layers: Vec<Layer2D>,

    pub x_scale: Scale,
    pub y_scale: Scale,
    pub style: Style,
    pub interaction: Interaction,
    pub labels: Vec<Label2D>,
}

#[derive(Clone, Debug)]
pub struct Label2D {
    pub at: Vec2, // in plot/world units (same space as xy)
    pub text: String,
    pub style: Style, // use style.color / size / opacity
}

impl Graph2D {
    pub fn new(layers: Vec<Layer2D>) -> Self {
        Self {
            layers,
            x_scale: Scale::Linear,
            y_scale: Scale::Linear,
            style: Style::default(),
            interaction: Interaction::default(),
            labels: Vec::new(),
        }
    }

    pub fn line(xy: Vec<Vec2>) -> Self {
        Self::new(vec![Layer2D::line(xy)])
    }
}

#[derive(Clone, Debug)]
pub struct Layer2D {
    pub geometry: Geometry2D,
    pub xy: Vec<Vec2>,
    pub style: Style,
    pub labels: Vec<Label2D>,
}

impl Layer2D {
    pub fn new(geometry: Geometry2D, xy: Vec<Vec2>) -> Self {
        Self {
            geometry,
            xy,
            style: Style::default(),
            labels: Vec::new(),
        }
    }

    pub fn line(xy: Vec<Vec2>) -> Self {
        Self::new(Geometry2D::Line, xy)
    }

    pub fn points(xy: Vec<Vec2>) -> Self {
        Self::new(Geometry2D::Points, xy)
    }
}

#[derive(Clone, Debug)]
pub enum Geometry2D {
    Points,
    Line,
    Area,
    Bars,
    Stems,
}
// Scatter = Points
// Line = Line
// Error bars = Points + Stems
// Candlestick = Bars + Stems
// Confidence band = Line + Area

#[derive(Clone, Debug)]
pub struct Graph3D {
    pub layers: Vec<Layer3D>,
    pub x_scale: Scale,
    pub y_scale: Scale,
    pub z_scale: Scale,
    pub style: Style,
    pub interaction: Interaction,
}

#[derive(Clone, Debug)]
pub struct Layer3D {
    pub geometry: Geometry3D,
    pub xyz: Vec<Vec3>,
    pub grid: Option<UVec2>, // required for Surface
    pub style: Style,
}

#[derive(Clone, Debug)]
pub enum Geometry3D {
    Points,
    Line,
    Surface,
}

impl Graph3D {
    pub fn surface(xyz: Vec<Vec3>, nx: u32, ny: u32) -> Self {
        Self {
            layers: vec![Layer3D {
                geometry: Geometry3D::Surface,
                xyz,
                grid: Some(UVec2::new(nx, ny)),
                style: Style::default(),
            }],
            x_scale: Scale::Linear,
            y_scale: Scale::Linear,
            z_scale: Scale::Linear,
            style: Style::default(),
            interaction: Interaction::default(),
        }
    }
}

// #[derive(Clone, Debug)]
// pub struct Graph2D {
//     /// One plot can have multiple geometry layers.
//     /// Example:
//     /// - confidence band: Area + Line
//     /// - error bars: Points + Stems
//     /// - candlestick: Bars + Stems
//     pub layers: Vec<Layer2D>,

//     pub x_scale: Scale,
//     pub y_scale: Scale,

//     /// Default style for the whole graph. Each layer can override.
//     pub style: Style,

//     pub interaction: Interaction,
// }

// #[derive(Clone, Debug)]
// pub struct Layer2D {
//     pub geometry: Geometry2D,

//     /// The data for THIS geometry.
//     pub xy: Vec<Vec2>,

//     /// Style overrides for this layer (None fields fall back to Graph2D.style).
//     pub style: Style,
// }

// #[derive(Clone, Debug)]
// pub enum Geometry2D {
//     Points,
//     Line,
//     Area,
//     Bars,
//     Stems,
// }

// impl Graph2D {
//     /// Minimal constructor.
//     pub fn new() -> Self {
//         Self {
//             layers: Vec::new(),
//             x_scale: Scale::Linear,
//             y_scale: Scale::Linear,
//             style: Style {
//                 color: None,
//                 size: None,
//                 opacity: None,
//             },
//             interaction: Interaction::default(),
//         }
//     }

//     /// Convenient one-liner for f(x)=y as a line.
//     pub fn line_fn<F>(x_min: f32, x_max: f32, n: usize, mut f: F) -> Self
//     where
//         F: FnMut(f32) -> f32,
//     {
//         let xy = sample_fn_xy(x_min, x_max, n, &mut f);
//         Self::new().with_layer(Layer2D::line_xy(xy))
//     }

//     /// Add a layer (builder style).
//     pub fn with_layer(mut self, layer: Layer2D) -> Self {
//         self.layers.push(layer);
//         self
//     }

//     /// Add a layer (mut style).
//     pub fn push_layer(&mut self, layer: Layer2D) {
//         self.layers.push(layer);
//     }

//     /// Bounds across all layers (useful for autoscale / ticks).
//     pub fn bounds(&self) -> Option<(Vec2, Vec2)> {
//         let mut any = false;
//         let mut min = Vec2::ZERO;
//         let mut max = Vec2::ZERO;

//         for layer in &self.layers {
//             for p in &layer.xy {
//                 if !p.x.is_finite() || !p.y.is_finite() {
//                     continue;
//                 }
//                 if !any {
//                     any = true;
//                     min = *p;
//                     max = *p;
//                     continue;
//                 }
//                 if p.x < min.x {
//                     min.x = p.x;
//                 }
//                 if p.y < min.y {
//                     min.y = p.y;
//                 }
//                 if p.x > max.x {
//                     max.x = p.x;
//                 }
//                 if p.y > max.y {
//                     max.y = p.y;
//                 }
//             }
//         }

//         any.then_some((min, max))
//     }

//     /// Drop NaN/Inf in-place (fast, no extra alloc).
//     pub fn drop_bad(&mut self) {
//         for layer in &mut self.layers {
//             layer.xy.retain(|p| p.x.is_finite() && p.y.is_finite());
//         }
//     }
// }

// impl Layer2D {
//     pub fn new(geometry: Geometry2D, xy: Vec<Vec2>) -> Self {
//         Self {
//             geometry,
//             xy,
//             style: Style {
//                 color: None,
//                 size: None,
//                 opacity: None,
//             },
//         }
//     }

//     pub fn line_xy(xy: Vec<Vec2>) -> Self {
//         let mut layer = Self::new(Geometry2D::Line, xy);
//         // default line width
//         layer.style.size = Some(2.0);
//         layer
//     }

//     pub fn points_xy(xy: Vec<Vec2>) -> Self {
//         let mut layer = Self::new(Geometry2D::Points, xy);
//         // default point radius
//         layer.style.size = Some(3.0);
//         layer
//     }

//     /// For ergonomics when you already have x/y arrays.
//     pub fn from_xy(geometry: Geometry2D, x: &[f32], y: &[f32]) -> Self {
//         assert_eq!(x.len(), y.len(), "x/y length mismatch");
//         let mut xy = Vec::with_capacity(x.len());
//         for i in 0..x.len() {
//             xy.push(Vec2::new(x[i], y[i]));
//         }
//         Self::new(geometry, xy)
//     }

//     /// Style overrides for this layer.
//     pub fn with_style(mut self, style: Style) -> Self {
//         self.style = style;
//         self
//     }
// }

// fn sample_fn_xy<F>(x_min: f32, x_max: f32, n: usize, f: &mut F) -> Vec<Vec2>
// where
//     F: FnMut(f32) -> f32,
// {
//     assert!(n >= 2, "need at least 2 samples");
//     let mut xy = Vec::with_capacity(n);
//     let denom = (n - 1) as f32;

//     for i in 0..n {
//         let t = i as f32 / denom;
//         let x = x_min + (x_max - x_min) * t;
//         xy.push(Vec2::new(x, f(x)));
//     }

//     xy
// }

// #[derive(Clone, Debug)]
// pub struct Encoding2D {
//     pub x: String,
//     pub y: String,

//     pub color: Option<String>,
//     pub size: Option<String>,
//     pub opacity: Option<String>,
// }

// #[derive(Clone, Debug)]
// pub struct Graph3D {
//     pub geometry: Geometry3D,
//     pub encoding: Encoding3D,

//     pub x_scale: Scale,
//     pub y_scale: Scale,
//     pub z_scale: Scale,

//     pub style: Style,
//     pub interaction: Interaction,
// }

// #[derive(Clone, Debug)]
// pub enum Geometry3D {
//     Points,
//     Line,
//     Surface,
//     Mesh,
//     VolumeSlice,
// }

// #[derive(Clone, Debug)]
// pub struct Encoding3D {
//     pub x: String,
//     pub y: String,
//     pub z: String,

//     pub color: Option<String>,
//     pub size: Option<String>,
// }

#[derive(Clone, Debug)]
pub struct Distribution {
    pub field: String,

    pub kind: DistributionKind,
    pub scale: Scale,

    pub style: Style,
}

#[derive(Clone, Debug)]
pub enum DistributionKind {
    Histogram { bins: usize, density: bool },
    KDE { bandwidth: f64 },
    Box,
    Violin,
    ECDF,
}

// Field plots (heatmap, image, tensors)
// heatmaps, attention matrices, correlation matrices, scalar fields
#[derive(Clone, Debug)]
pub struct Field {
    pub x: String,
    pub y: String,
    pub value: String,

    pub scale: Scale,
    pub style: Style,
}

// Radial plots (pie, radar, polar)
#[derive(Clone, Debug)]
pub struct Radial {
    pub angle: String,
    pub radius: Option<String>,
    pub value: Option<String>,

    pub kind: RadialKind,
    pub scale: Scale,

    pub style: Style,
}

#[derive(Clone, Debug)]
pub enum RadialKind {
    Pie,
    Donut,
    Radar,
    PolarPoints,
    PolarLine,
}
