#[derive(Clone, Debug)]
pub enum Scale {
    Linear,
    Log,
    Symlog { lin_thresh: f64 },
    Power { exponent: f64 },
    Time,
    Categorical,
}

#[derive(Clone, Debug)]
pub struct Style {
    pub color: Option<Color>,
    pub size: Option<f32>,
    pub opacity: Option<f32>,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            color: None,
            size: None,
            opacity: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Color {
    Rgb(f32, f32, f32),
    Rgba(f32, f32, f32, f32),
    Named(&'static str),
}

#[derive(Clone, Debug, Default)]
pub struct Interaction {
    pub hover: bool,
    pub select: bool,
    pub brush: bool,
}
