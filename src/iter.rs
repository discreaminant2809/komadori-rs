//! Extension for the [`Iterator`] trait and
//! [`Collector`]s for common operations in that trait.
//!
//! This module also includes most "terminal" operations such as [`fold()`], [`any()`]
//! and [`find()`], except some like [`min()`], [`max()`] and [`sum()`]
//! which are in more appropriate modules.
//!
//! This module corresponds to [`std::iter`].
//!
//! [`Collector`]: crate::collector::Collector
//! [`fold()`]: Iterator::fold
//! [`any()`]: Iterator::any
//! [`find()`]: Iterator::find
//! [`min()`]: Iterator::min
//! [`max()`]: Iterator::max
//! [`sum()`]: Iterator::sum

mod all_any;
mod count;
#[cfg(feature = "unstable")]
mod driver;
mod find;
mod find_map;
mod fold;
mod for_each;
mod iterator_ext;
mod last;
mod position;
mod reduce;
mod try_fold;

pub use all_any::*;
pub use count::*;
#[cfg(feature = "unstable")]
pub use driver::*;
pub use find::*;
pub use find_map::*;
pub use fold::*;
pub use for_each::*;
pub use iterator_ext::*;
pub use last::*;
pub use position::*;
pub use reduce::*;
pub use try_fold::*;
