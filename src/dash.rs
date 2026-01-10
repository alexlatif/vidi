use crate::core::{
    Candlestick, Color, Colormap, Dashboard, Distribution, Field, Geometry2D, Geometry3D, Graph2D,
    Graph3D, Heatmap, Layer2D, Layer3D, OHLC, Plot, PlotMeta, Radial, Style, Tab,
};
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

    /// Set the number of columns per row (default: auto based on plot count)
    pub fn columns(mut self, cols: usize) -> Self {
        self.dash.columns = Some(cols.max(1));
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
            meta: PlotMeta::default(),
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

    pub fn add_candlestick<F>(mut self, f: F) -> Self
    where
        F: FnOnce(CandlestickBuilder) -> CandlestickBuilder,
    {
        let b = f(CandlestickBuilder::new());
        self.dash.plots.push(Plot::Candlestick(b.candle));
        self
    }

    pub fn add_heatmap<F>(mut self, f: F) -> Self
    where
        F: FnOnce(HeatmapBuilder) -> HeatmapBuilder,
    {
        let b = f(HeatmapBuilder::new());
        self.dash.plots.push(Plot::Heatmap(b.heatmap));
        self
    }

    /// Add a tab to the dashboard
    pub fn add_tab<F>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(TabBuilder) -> TabBuilder,
    {
        let b = f(TabBuilder::new(name));
        self.dash.tabs.push(b.tab);
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn show(self) {
        crate::runtime::run_dashboard(self.dash);
    }
}

/* -------------------- TAB BUILDER -------------------- */

pub struct TabBuilder {
    tab: Tab,
}

impl TabBuilder {
    fn new(name: impl Into<String>) -> Self {
        Self {
            tab: Tab::new(name),
        }
    }

    /// Set the number of columns per row for this tab
    pub fn columns(mut self, cols: usize) -> Self {
        self.tab.columns = Some(cols.max(1));
        self
    }

    pub fn add_2d<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Plot2DBuilder) -> Plot2DBuilder,
    {
        let b = f(Plot2DBuilder::new());
        self.tab.plots.push(Plot::Graph2D(b.graph));
        self
    }

    pub fn add_3d<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Plot3DBuilder) -> Plot3DBuilder,
    {
        let b = f(Plot3DBuilder::new());
        self.tab.plots.push(Plot::Graph3D(b.graph));
        self
    }

    pub fn add_distribution<F>(mut self, f: F) -> Self
    where
        F: FnOnce(DistBuilder) -> DistBuilder,
    {
        let b = f(DistBuilder::new());
        self.tab.plots.push(Plot::Distribution(b.dist));
        self
    }

    pub fn add_field(mut self, dims: UVec2, values: Vec<f32>, vmin: f32, vmax: f32) -> Self {
        self.tab.plots.push(Plot::Field(Field {
            meta: PlotMeta::default(),
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
        self.tab.plots.push(Plot::Radial(b.rad));
        self
    }

    pub fn add_candlestick<F>(mut self, f: F) -> Self
    where
        F: FnOnce(CandlestickBuilder) -> CandlestickBuilder,
    {
        let b = f(CandlestickBuilder::new());
        self.tab.plots.push(Plot::Candlestick(b.candle));
        self
    }

    pub fn add_heatmap<F>(mut self, f: F) -> Self
    where
        F: FnOnce(HeatmapBuilder) -> HeatmapBuilder,
    {
        let b = f(HeatmapBuilder::new());
        self.tab.plots.push(Plot::Heatmap(b.heatmap));
        self
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

    /// Bubble chart (scatter with variable point sizes)
    pub fn bubble(mut self, xy: Vec<Vec2>, sizes: Vec<f32>, style: impl Into<Option<Style>>) -> Self {
        let mut layer = Layer2D::new(Geometry2D::Points, xy);
        layer.sizes = Some(sizes);
        if let Some(st) = style.into() {
            layer.style = st;
        }
        self.graph.layers.push(layer);
        self
    }

    /// Set the X-axis label
    pub fn x_label(mut self, label: impl Into<String>) -> Self {
        self.graph.x_label = Some(label.into());
        self
    }

    /// Set the Y-axis label
    pub fn y_label(mut self, label: impl Into<String>) -> Self {
        self.graph.y_label = Some(label.into());
        self
    }

    /// Set the plot title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.graph.meta.title = Some(title.into());
        self
    }

    /// Set the plot description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.graph.meta.description = Some(desc.into());
        self
    }

    /// Fill between two lines (for confidence intervals, ranges, etc.)
    ///
    /// # Arguments
    /// * `upper` - Upper bound line as Vec<Vec2>
    /// * `lower` - Lower bound line as Vec<Vec2>
    /// * `style` - Style for the fill (opacity controls transparency)
    pub fn fill_between(
        mut self,
        upper: Vec<Vec2>,
        lower: Vec<Vec2>,
        style: impl Into<Option<Style>>,
    ) -> Self {
        let mut layer = Layer2D::new(Geometry2D::FillBetween, upper);
        layer.lower_line = Some(lower);

        if let Some(st) = style.into() {
            layer.style = st;
        }
        self.graph.layers.push(layer);
        self
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

    /// Set the plot title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.graph.meta.title = Some(title.into());
        self
    }

    /// Set the plot description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.graph.meta.description = Some(desc.into());
        self
    }

    /// Set the X-axis label
    pub fn x_label(mut self, label: impl Into<String>) -> Self {
        self.graph.x_label = Some(label.into());
        self
    }

    /// Set the Y-axis label
    pub fn y_label(mut self, label: impl Into<String>) -> Self {
        self.graph.y_label = Some(label.into());
        self
    }

    /// Set the Z-axis label
    pub fn z_label(mut self, label: impl Into<String>) -> Self {
        self.graph.z_label = Some(label.into());
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
                meta: PlotMeta::default(),
                values: vec![],
                bins: 30,
                style: Style::default(),
                x_label: None,
                y_label: None,
            },
        }
    }

    pub fn histogram(mut self, values: Vec<f32>) -> Self {
        self.dist = Distribution::Histogram {
            meta: PlotMeta::default(),
            values,
            bins: 30,
            style: Style::default(),
            x_label: None,
            y_label: None,
        };
        self
    }

    pub fn pdf(mut self, values: Vec<f32>) -> Self {
        self.dist = Distribution::Pdf {
            meta: PlotMeta::default(),
            values,
            style: Style::default(),
            x_label: None,
            y_label: None,
        };
        self
    }

    pub fn bins(mut self, bins: usize) -> Self {
        if let Distribution::Histogram { bins: b, .. } = &mut self.dist {
            *b = bins.max(1);
        }
        self
    }

    pub fn boxplot(mut self, groups: Vec<(impl Into<String>, Vec<f32>)>) -> Self {
        self.dist = Distribution::BoxPlot {
            meta: PlotMeta::default(),
            groups: groups.into_iter().map(|(l, v)| (l.into(), v)).collect(),
            style: Style::default(),
            x_label: None,
            y_label: None,
        };
        self
    }

    pub fn ecdf(mut self, values: Vec<f32>) -> Self {
        self.dist = Distribution::ECDF {
            meta: PlotMeta::default(),
            values,
            style: Style::default(),
            x_label: None,
            y_label: None,
        };
        self
    }

    pub fn style(mut self, s: Style) -> Self {
        match &mut self.dist {
            Distribution::Histogram { style, .. } => *style = s,
            Distribution::Pdf { style, .. } => *style = s,
            Distribution::BoxPlot { style, .. } => *style = s,
            Distribution::ECDF { style, .. } => *style = s,
        }
        self
    }

    pub fn x_label(mut self, label: impl Into<String>) -> Self {
        match &mut self.dist {
            Distribution::Histogram { x_label, .. } => *x_label = Some(label.into()),
            Distribution::Pdf { x_label, .. } => *x_label = Some(label.into()),
            Distribution::BoxPlot { x_label, .. } => *x_label = Some(label.into()),
            Distribution::ECDF { x_label, .. } => *x_label = Some(label.into()),
        }
        self
    }

    pub fn y_label(mut self, label: impl Into<String>) -> Self {
        match &mut self.dist {
            Distribution::Histogram { y_label, .. } => *y_label = Some(label.into()),
            Distribution::Pdf { y_label, .. } => *y_label = Some(label.into()),
            Distribution::BoxPlot { y_label, .. } => *y_label = Some(label.into()),
            Distribution::ECDF { y_label, .. } => *y_label = Some(label.into()),
        }
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        match &mut self.dist {
            Distribution::Histogram { meta, .. } => meta.title = Some(title.into()),
            Distribution::Pdf { meta, .. } => meta.title = Some(title.into()),
            Distribution::BoxPlot { meta, .. } => meta.title = Some(title.into()),
            Distribution::ECDF { meta, .. } => meta.title = Some(title.into()),
        }
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        match &mut self.dist {
            Distribution::Histogram { meta, .. } => meta.description = Some(desc.into()),
            Distribution::Pdf { meta, .. } => meta.description = Some(desc.into()),
            Distribution::BoxPlot { meta, .. } => meta.description = Some(desc.into()),
            Distribution::ECDF { meta, .. } => meta.description = Some(desc.into()),
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
            rad: Radial::Pie { meta: PlotMeta::default(), slices: vec![] },
        }
    }

    pub fn pie(mut self, slices: Vec<(impl Into<String>, f32)>) -> Self {
        self.rad = Radial::Pie {
            meta: PlotMeta::default(),
            slices: slices.into_iter().map(|(l, v)| (l.into(), v)).collect(),
        };
        self
    }

    pub fn radar(mut self, axes: Vec<impl Into<String>>, values: Vec<f32>) -> Self {
        self.rad = Radial::Radar {
            meta: PlotMeta::default(),
            axes: axes.into_iter().map(|a| a.into()).collect(),
            values,
            style: Style::default(),
        };
        self
    }

    pub fn style(mut self, s: Style) -> Self {
        if let Radial::Radar { style, .. } = &mut self.rad {
            *style = s;
        }
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        match &mut self.rad {
            Radial::Pie { meta, .. } => meta.title = Some(title.into()),
            Radial::Radar { meta, .. } => meta.title = Some(title.into()),
        }
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        match &mut self.rad {
            Radial::Pie { meta, .. } => meta.description = Some(desc.into()),
            Radial::Radar { meta, .. } => meta.description = Some(desc.into()),
        }
        self
    }
}

/* -------------------- CANDLESTICK BUILDER -------------------- */

pub struct CandlestickBuilder {
    candle: Candlestick,
}

impl CandlestickBuilder {
    fn new() -> Self {
        Self {
            candle: Candlestick {
                meta: PlotMeta::default(),
                candles: vec![],
                up_color: Color::rgb(0.2, 0.8, 0.3),   // green
                down_color: Color::rgb(0.9, 0.2, 0.2), // red
                x_label: None,
                y_label: None,
            },
        }
    }

    /// Add OHLC data as Vec<(x, open, high, low, close)>
    pub fn data(mut self, candles: Vec<(f32, f32, f32, f32, f32)>) -> Self {
        self.candle.candles = candles
            .into_iter()
            .map(|(x, o, h, l, c)| OHLC::new(x, o, h, l, c))
            .collect();
        self
    }

    /// Add OHLC data directly
    pub fn ohlc(mut self, candles: Vec<OHLC>) -> Self {
        self.candle.candles = candles;
        self
    }

    pub fn up_color(mut self, c: Color) -> Self {
        self.candle.up_color = c;
        self
    }

    pub fn down_color(mut self, c: Color) -> Self {
        self.candle.down_color = c;
        self
    }

    pub fn x_label(mut self, label: impl Into<String>) -> Self {
        self.candle.x_label = Some(label.into());
        self
    }

    pub fn y_label(mut self, label: impl Into<String>) -> Self {
        self.candle.y_label = Some(label.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.candle.meta.title = Some(title.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.candle.meta.description = Some(desc.into());
        self
    }
}

/* -------------------- HEATMAP BUILDER -------------------- */

pub struct HeatmapBuilder {
    heatmap: Heatmap,
}

impl HeatmapBuilder {
    fn new() -> Self {
        Self {
            heatmap: Heatmap {
                meta: PlotMeta::default(),
                dims: UVec2::ZERO,
                values: vec![],
                vmin: None,
                vmax: None,
                row_labels: None,
                col_labels: None,
                show_values: false,
                colormap: Colormap::default(),
            },
        }
    }

    /// Set data as row-major 2D array with dimensions
    pub fn data(mut self, rows: usize, cols: usize, values: Vec<f32>) -> Self {
        self.heatmap.dims = UVec2::new(cols as u32, rows as u32);
        self.heatmap.values = values;
        self
    }

    /// Set data from 2D Vec (row-major)
    pub fn from_2d(mut self, data: Vec<Vec<f32>>) -> Self {
        let rows = data.len();
        let cols = data.first().map(|r| r.len()).unwrap_or(0);
        self.heatmap.dims = UVec2::new(cols as u32, rows as u32);
        self.heatmap.values = data.into_iter().flatten().collect();
        self
    }

    pub fn vmin(mut self, v: f32) -> Self {
        self.heatmap.vmin = Some(v);
        self
    }

    pub fn vmax(mut self, v: f32) -> Self {
        self.heatmap.vmax = Some(v);
        self
    }

    pub fn row_labels(mut self, labels: Vec<impl Into<String>>) -> Self {
        self.heatmap.row_labels = Some(labels.into_iter().map(|l| l.into()).collect());
        self
    }

    pub fn col_labels(mut self, labels: Vec<impl Into<String>>) -> Self {
        self.heatmap.col_labels = Some(labels.into_iter().map(|l| l.into()).collect());
        self
    }

    pub fn show_values(mut self, show: bool) -> Self {
        self.heatmap.show_values = show;
        self
    }

    pub fn colormap(mut self, cm: Colormap) -> Self {
        self.heatmap.colormap = cm;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.heatmap.meta.title = Some(title.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.heatmap.meta.description = Some(desc.into());
        self
    }
}
