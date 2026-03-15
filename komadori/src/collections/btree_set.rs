//! Collectors for [`BTreeSet`]
//!
//! This module corresponds to [`std::collections::btree_set`].

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeSet;
#[cfg(feature = "std")]
use std::collections::BTreeSet;

/// A collector that inserts collected items into a [`BTreeSet`].
/// Its [`Output`] is [`BTreeSet`].
///
/// This struct is created by `BTreeSet::into_collector()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoCollector<T>(pub(super) BTreeSet<T>);

/// A collector that inserts collected items into a [`&mut BTreeSet`](BTreeSet).
/// Its [`Output`] is [`&mut BTreeSet`](BTreeSet).
///
/// This struct is created by `BTreeSet::collector_mut()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a, T>(pub(super) &'a mut BTreeSet<T>);
