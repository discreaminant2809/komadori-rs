//! Parallel collectors that repeatedly apply common operators in [`std::ops`].
//!
//! This module corresponds to [`std::ops`].

mod advanced_par_closure;
mod and_or;
mod basic_par_closure;
mod call;
mod par_fn;
mod par_product;
mod par_sum;
mod tri;

pub(crate) use advanced_par_closure::*;
pub use and_or::*;
pub(crate) use basic_par_closure::*;
pub(crate) use call::*;
pub(crate) use par_fn::*;
pub use par_product::*;
pub use par_sum::*;
pub(crate) use tri::*;
