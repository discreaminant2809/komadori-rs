//! Parallel collectors that repeatedly apply common operators in [`std::ops`].
//!
//! This module corresponds to [`std::ops`].

mod par_product;
mod par_sum;

pub use par_product::*;
pub use par_sum::*;
