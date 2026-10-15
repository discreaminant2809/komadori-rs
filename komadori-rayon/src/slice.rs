//! Parallel collectors for slice manipulation.
//!
//! This module corresponds to [`std::slice`].

pub(crate) mod in_place_write;
#[cfg(test)]
pub(crate) mod par_iter;
#[cfg(test)]
pub(crate) mod par_iter_mut;

mod concat;

pub use concat::*;
