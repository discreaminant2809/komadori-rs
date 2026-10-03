//! Parallel collectors for comparing items.
//!
//! This module provides collectors that determine the maximum or minimum
//! values among the items they collect, using different comparison strategies.
//! They correspond to [`Iterator`]’s comparison-related methods, such as
//! [`Iterator::max()`], [`Iterator::min_by()`], and [`Iterator::max_by_key()`].
//!
//! This module corresponds to [`std::cmp`].

mod is_sorted;
mod max;
mod min;

pub use is_sorted::*;
pub use max::*;
pub use min::*;
