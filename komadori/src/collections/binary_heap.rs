//! Collectors for [`BinaryHeap`]
//!
//! This module corresponds to [`std::collections::binary_heap`].
//!
//! # Notes
//!
//! You generally should **not** use `BinaryHeap::new().into_collector()`
//! to construct a new `BinaryHeap`, since the time complexity is `O(n log n)`,
//! which can be less efficient than constructing a [`Vec`] then converting to
//! a `BinaryHeap` (`O(n)`).
//!
//! Do the following instead to construct a [`BinaryHeap`] from scratch:
//!
//! ```
//! use std::collections::BinaryHeap;
//! use komadori::prelude::*;
//!
//! let mut collector = vec![]
//!     .into_collector()
//!     .map_output(BinaryHeap::from);
//! #
//! # collector.collect(1);
//! ```

#[cfg(not(feature = "std"))]
use alloc::collections::BinaryHeap;
#[cfg(feature = "std")]
use std::collections::BinaryHeap;

/// A collector that pushes collected items into a [`BinaryHeap`].
/// Its [`Output`] is [`BinaryHeap`].
///
/// This struct is created by `BinaryHeap::into_collector()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoCollector<T>(pub(super) BinaryHeap<T>);

/// A collector that pushes collected items into a [`&mut BinaryHeap`](BinaryHeap).
/// Its [`Output`] is [`&mut BinaryHeap`](BinaryHeap).
///
/// This struct is created by `BinaryHeap::collector_mut()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a, T>(pub(super) &'a mut BinaryHeap<T>);
