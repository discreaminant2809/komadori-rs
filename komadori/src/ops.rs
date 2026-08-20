//! [`Collector`]s that repeatedly apply common operators in [`std::ops`].
//!
//! This module corresponds to [`std::ops`].
//!
//! [`Collector`]: crate::collector::Collector

mod and;
mod or;
mod product;
mod sum;
mod tri;

pub use and::*;
pub use or::*;
pub use product::*;
pub use sum::*;
pub(crate) use tri::*;
