//! Collectors for [`HashSet`]
//!
//! This module corresponds to [`std::collections::hash_set`].

use std::collections::HashSet;

/// A collector that inserts collected items into a [`HashSet`].
/// Its [`Output`] is [`HashSet`].
///
/// This struct is created by `HashSet::into_collector()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoCollector<T, S>(pub(super) HashSet<T, S>);

/// A collector that inserts collected items into a [`&mut HashSet`](HashSet).
/// Its [`Output`] is [`&mut HashSet`](HashSet).
///
/// This struct is created by `HashSet::collector_mut()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a, T, S>(pub(super) &'a mut HashSet<T, S>);
