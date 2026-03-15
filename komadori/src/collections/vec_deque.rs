//! Collectors for [`VecDeque`]
//!
//! This module corresponds to [`std::collections::vec_deque`].

#[cfg(not(feature = "std"))]
use alloc::collections::VecDeque;
#[cfg(feature = "std")]
use std::collections::VecDeque;

/// A collector that pushes collected items into the back of a [`VecDeque`].
/// Its [`Output`] is [`VecDeque`].
///
/// This struct is created by `VecDeque::into_collector()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoCollector<T>(pub(super) VecDeque<T>);

/// A collector that pushes collected items into the back of a [`&mut VecDeque`](VecDeque).
/// Its [`Output`] is [`&mut VecDeque`](VecDeque).
///
/// This struct is created by `VecDeque::collector_mut()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a, T>(pub(super) &'a mut VecDeque<T>);
