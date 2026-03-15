//! Collectors for [`LinkedList`]
//!
//! This module corresponds to [`std::collections::linked_list`].

#[cfg(not(feature = "std"))]
use alloc::collections::LinkedList;
#[cfg(feature = "std")]
use std::collections::LinkedList;

/// A collector that pushes collected items into the back of a [`LinkedList`].
/// Its [`Output`] is [`LinkedList`].
///
/// This struct is created by `LinkedList::into_collector()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoCollector<T>(pub(super) LinkedList<T>);

/// A collector that pushes collected items into the back of a [`&mut LinkedList`](LinkedList).
/// Its [`Output`] is [`&mut LinkedList`](LinkedList).
///
/// This struct is created by `LinkedList::collector_mut()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a, T>(pub(super) &'a mut LinkedList<T>);
