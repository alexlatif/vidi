use crate::core::*;
use bevy_math::{UVec2, Vec2, Vec3};

pub fn dash() -> DashBuilder {
    DashBuilder {
        dash: Dashboard::default(),
    }
}

pub struct DashBuilder {
    dash: Dashboard,
}

impl DashBuilder {
    pub fn background_color(mut self, c: Color) -> Self {
        self.dash.background = c;
        self
    }

    pub fn add_2d<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Plot2DBuilder) -> Plot2DBuilder,
    {
        let b = f(Plot2DBuilder::new());
        self.dash.plots.push(Plot::Graph2D(b.graph));
        self
    }

    pub fn add_3d<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Plot3DBuilder) -> Plot3DBuilder,
    {
        let b = f(Plot3DBuilder::new());
        self.dash.plots.push(Plot::Graph3D(b.graph));
        self
    }

    pub fn add_distribution<F>(mut self, f: F) -> Self
    where
        F: FnOnce(DistBuilder) -> DistBuilder,
    {
        let b = f(DistBuilder::new());
        self.dash.plots.push(Plot::Distribution(b.dist));
        self
    }

    pub fn add_field(mut self, dims: UVec2, values: Vec<f32>, vmin: f32, vmax: f32) -> Self {
        self.dash.plots.push(Plot::Field(Field {
            dims,
            values,
            vmin,
            vmax,
        }));
        self
    }

    pub fn add_radial<F>(mut self, f: F) -> Self
    where
        F: FnOnce(RadialBuilder) -> RadialBuilder,
    {
        let b = f(RadialBuilder::new());
        self.dash.plots.push(Plot::Radial(b.rad));
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn show(self) {
        crate::runtime::run_dashboard(self.dash);
    }
}

/* -------------------- PLOT 2D BUILDER -------------------- */

pub struct Plot2DBuilder {
    graph: Graph2D,
}

impl Plot2DBuilder {
    fn new() -> Self {
        Self {
            graph: Graph2D::new(),
        }
    }

    fn push_layer(mut self, geometry: Geometry2D, xy: Vec<Vec2>, style: Option<Style>) -> Self {
        let mut layer = Layer2D::new(geometry, xy);
        if let Some(st) = style {
            layer.style = st;
        }
        self.graph.layers.push(layer);
        self
    }

    pub fn line(self, xy: Vec<Vec2>, style: impl Into<Option<Style>>) -> Self {
        self.push_layer(Geometry2D::Line, xy, style.into())
    }

    pub fn scatter(self, xy: Vec<Vec2>, style: impl Into<Option<Style>>) -> Self {
        self.push_layer(Geometry2D::Points, xy, style.into())
    }

    pub fn area(self, xy: Vec<Vec2>, style: impl Into<Option<Style>>) -> Self {
        self.push_layer(Geometry2D::Area, xy, style.into())
    }

    pub fn bars(self, xy: Vec<Vec2>, style: impl Into<Option<Style>>) -> Self {
        self.push_layer(Geometry2D::Bars, xy, style.into())
    }

    pub fn stems(self, xy: Vec<Vec2>, style: impl Into<Option<Style>>) -> Self {
        self.push_layer(Geometry2D::Stems, xy, style.into())
    }
}

// Allow passing &Style into the `impl Into<Option<Style>>` slot.
// (Do NOT implement From<Style> for Option<Style> — std already has it.)
impl From<&Style> for Option<Style> {
    #[inline]
    fn from(s: &Style) -> Self {
        Some(*s)
    }
}

/* -------------------- PLOT 3D BUILDER -------------------- */

pub struct Plot3DBuilder {
    graph: Graph3D,
}

impl Plot3DBuilder {
    fn new() -> Self {
        Self {
            graph: Graph3D::new(),
        }
    }

    pub fn points(mut self, xyz: Vec<Vec3>, style: impl Into<Option<Style>>) -> Self {
        let mut layer = Layer3D {
            geometry: Geometry3D::Points,
            xyz,
            style: Style::default(),
        };
        if let Some(st) = style.into() {
            layer.style = st;
        }
        self.graph.layers.push(layer);
        self
    }

    pub fn surface(
        mut self,
        xyz: Vec<Vec3>,
        nx: u32,
        ny: u32,
        style: impl Into<Option<Style>>,
    ) -> Self {
        let mut layer = Layer3D {
            geometry: Geometry3D::Surface {
                grid: UVec2::new(nx, ny),
            },
            xyz,
            style: Style::default(),
        };
        if let Some(st) = style.into() {
            layer.style = st;
        }
        self.graph.layers.push(layer);
        self
    }
}

/* -------------------- DISTRIBUTION BUILDER -------------------- */

pub struct DistBuilder {
    dist: Distribution,
}

impl DistBuilder {
    fn new() -> Self {
        Self {
            dist: Distribution::Histogram {
                values: vec![],
                bins: 30,
                style: Style::default(),
            },
        }
    }

    pub fn histogram(mut self, values: Vec<f32>) -> Self {
        if let Distribution::Histogram { values: v, .. } = &mut self.dist {
            *v = values;
        }
        self
    }

    pub fn bins(mut self, bins: usize) -> Self {
        if let Distribution::Histogram { bins: b, .. } = &mut self.dist {
            *b = bins.max(1);
        }
        self
    }

    pub fn style(mut self, s: &Style) -> Self {
        if let Distribution::Histogram { style, .. } = &mut self.dist {
            *style = *s;
        }
        self
    }
}

/* -------------------- RADIAL BUILDER -------------------- */

pub struct RadialBuilder {
    rad: Radial,
}

impl RadialBuilder {
    fn new() -> Self {
        Self {
            rad: Radial::Pie { slices: vec![] },
        }
    }

    pub fn pie(mut self, slices: Vec<(impl Into<String>, f32)>) -> Self {
        self.rad = Radial::Pie {
            slices: slices.into_iter().map(|(l, v)| (l.into(), v)).collect(),
        };
        self
    }
}
