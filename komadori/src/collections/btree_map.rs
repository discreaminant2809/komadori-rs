//! Collectors for [`BTreeMap`]
//!
//! This module corresponds to [`std::collections::btree_map`].

use alloc::collections::BTreeMap;

// #[cfg(feature = "unstable")]
// use crate::aggregate::{Group, GroupMap, OccupiedGroup, VacantGroup};

/// A collector that inserts collected items into a [`BTreeMap`].
/// Its [`Output`] is [`BTreeMap`].
///
/// This struct is created by `BTreeMap::into_collector()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoCollector<K, V>(pub(super) BTreeMap<K, V>);

/// A collector that inserts collected items into a [`&mut BTreeMap`](BTreeMap).
/// Its [`Output`] is [`&mut BTreeMap`](BTreeMap).
///
/// This struct is created by `BTreeMap::collector_mut()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a, K, V>(pub(super) &'a mut BTreeMap<K, V>);
