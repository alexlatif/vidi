pub mod core;
pub mod dash;
pub mod render;
pub mod runtime;

use std::fmt;

#[derive(Debug)]
pub struct VidiError;

impl fmt::Display for VidiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VidiError")
    }
}

impl std::error::Error for VidiError {}

pub type Result<T> = std::result::Result<T, error_stack::Report<VidiError>>;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

pub mod prelude {
    pub use crate::core::*;
    pub use crate::dash::*;
    pub use crate::render::*;
    pub use crate::runtime::*;
}
