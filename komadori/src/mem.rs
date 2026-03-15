//! [`Collector`]s that deal with memory.
//!
//! This module corresponds to [`std::mem`].
//!
//! [`Collector`]: crate::collector::Collector

mod dropping;
mod forgetting;

pub use dropping::*;
pub use forgetting::*;
