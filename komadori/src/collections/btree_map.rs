//! Collectors for [`BTreeMap`]
//!
//! This module corresponds to [`std::collections::btree_map`].

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

// #[cfg(all(not(feature = "std"), feature = "unstable"))]
// use alloc::collections::btree_map::{Entry, OccupiedEntry, VacantEntry};
// #[cfg(all(feature = "std", feature = "unstable"))]
// use std::collections::btree_map::{Entry, OccupiedEntry, VacantEntry};

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

// #[cfg(feature = "unstable")]
// // #[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "unstable"))))]
// impl<'a, K, V> VacantGroup for VacantEntry<'a, K, V>
// where
//     K: Ord,
// {
//     type Key = K;

//     type Value = V;

//     #[inline]
//     fn key(&self) -> &Self::Key {
//         self.key()
//     }

//     #[inline]
//     fn insert(self, value: Self::Value) {
//         self.insert(value);
//     }
// }

// #[cfg(feature = "unstable")]
// // #[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "unstable"))))]
// impl<'a, K, V> OccupiedGroup for OccupiedEntry<'a, K, V>
// where
//     K: Ord,
// {
//     type Key = K;

//     type Value = V;

//     #[inline]
//     fn key(&self) -> &Self::Key {
//         self.key()
//     }

//     #[inline]
//     fn value(&self) -> &Self::Value {
//         self.get()
//     }

//     #[inline]
//     fn value_mut(&mut self) -> &mut Self::Value {
//         self.get_mut()
//     }
// }

// #[cfg(feature = "unstable")]
// // #[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "unstable"))))]
// impl<K, V> GroupMap for BTreeMap<K, V>
// where
//     K: Ord,
// {
//     type Key = K;

//     type Value = V;

//     type Vacant<'a>
//         = VacantEntry<'a, K, V>
//     where
//         Self: 'a;

//     type Occupied<'a>
//         = OccupiedEntry<'a, K, V>
//     where
//         Self: 'a;

//     #[inline]
//     fn group(&mut self, key: Self::Key) -> Group<Self::Occupied<'_>, Self::Vacant<'_>> {
//         match self.entry(key) {
//             Entry::Occupied(entry) => Group::Occupied(entry),
//             Entry::Vacant(entry) => Group::Vacant(entry),
//         }
//     }
// }
