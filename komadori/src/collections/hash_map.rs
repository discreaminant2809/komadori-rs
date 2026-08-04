//! Collectors for [`HashMap`]
//!
//! This module corresponds to [`std::collections::hash_map`].

use std::collections::HashMap;

// #[cfg(feature = "unstable")]
// use crate::aggregate::{Group, GroupMap, OccupiedGroup, VacantGroup};

/// A collector that inserts collected items into a [`HashMap`].
/// Its [`Output`] is [`HashMap`].
///
/// This struct is created by `HashMap::into_collector()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoCollector<K, V, S>(pub(super) HashMap<K, V, S>);

/// A collector that inserts collected items into a [`&mut HashMap`](HashMap).
/// Its [`Output`] is [`&mut HashMap`](HashMap).
///
/// This struct is created by `HashMap::collector_mut()`.
///
/// [`Output`]: crate::collector::CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a, K, V, S>(pub(super) &'a mut HashMap<K, V, S>);
