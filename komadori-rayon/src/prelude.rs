//! Re-exports commonly used items from this crate.
//!
//! This module is intended to be imported with a wildcard, providing
//! convenient access to the most frequently used traits and types.
//!
//! # Example
//!
//! ```
//! use komadori_rayon::prelude::*;
//! ```

pub use crate::collector::{
    IntoParallelCollector, IntoParallelCollectorBase, IntoUnindexedParallelCollector,
    IntoUnindexedParallelCollectorBase, ParallelCollector, ParallelCollectorBase,
    ParallelCollectorByMut, ParallelCollectorByRef, UnindexedParallelCollector,
    UnindexedParallelCollectorBase,
};
#[cfg(feature = "rayon")]
pub use crate::iter::RayonParallelIteratorExt;
