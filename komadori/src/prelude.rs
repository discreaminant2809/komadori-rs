//! Re-exports commonly used items from this crate.
//!
//! This module is intended to be imported with a wildcard, providing
//! convenient access to the most frequently used traits and types.
//!
//! # Example
//!
//! ```
//! use komadori::prelude::*;
//! ```

pub use crate::{
    collector::{
        Collector, CollectorBase, CollectorByMut, CollectorByRef, IntoCollector, IntoCollectorBase,
    },
    iter::IteratorExt,
    ops::{Adding, Muling},
    slice::Concat,
};
