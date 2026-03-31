//! Parallel collectors for most "terminal" operations such as [`fold()`],
//! [`any()`] and [`find()`], except some like [`min()`], [`max()`] and [`sum()`]
//! which are in more appropriate modules.
//!
//! This module corresponds to [`std::iter`].
//!
//! [`fold()`]: Iterator::fold
//! [`any()`]: Iterator::any
//! [`find()`]: Iterator::find
//! [`min()`]: Iterator::min
//! [`max()`]: Iterator::max
//! [`sum()`]: Iterator::sum

mod par_count;
mod par_reduce;
#[cfg(feature = "rayon")]
mod rayon_par_iter_ext;

pub use par_count::*;
pub use par_reduce::*;
#[cfg(feature = "rayon")]
pub use rayon_par_iter_ext::*;
