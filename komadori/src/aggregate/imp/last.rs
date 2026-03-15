use std::{fmt::Debug, marker::PhantomData};

use crate::aggregate::{AggregateOp, assert_op};

/// An [`AggregateOp`] that sets the last item it operated on.
///
/// It can also act as an `insert` op.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use komadori::{
///     prelude::*,
///     aggregate::{self, GroupMap},
/// };
///
/// let mut collector = HashMap::new()
///     .into_aggregate(aggregate::Last::new());
///
/// assert!(collector.collect((1, 1)).is_continue());
/// assert!(collector.collect((1, 4)).is_continue());
/// assert!(collector.collect((2, 1)).is_continue());
/// assert!(collector.collect((1, 2)).is_continue());
/// assert!(collector.collect((2, 3)).is_continue());
///
/// let counts = collector.finish();
///
/// assert_eq!(counts[&1], 2);
/// assert_eq!(counts[&2], 3);
/// ```
pub struct Last<K, V> {
    _marker: PhantomData<fn(&K, V, &mut V) -> V>,
}

impl<K, V> Last<K, V> {
    /// Creates a new instance of this aggregate op.
    #[inline]
    pub const fn new() -> Self {
        assert_op(Self {
            _marker: PhantomData,
        })
    }
}

impl<K, V> AggregateOp for Last<K, V> {
    type Key = K;

    type Value = V;

    type Item = V;

    #[inline]
    fn new_value(&mut self, _key: &Self::Key, item: Self::Item) -> Self::Value {
        item
    }

    #[inline]
    fn modify(&mut self, value: &mut Self::Value, item: Self::Item) {
        *value = item;
    }
}

impl<K, V> Default for Last<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Clone for Last<K, V> {
    fn clone(&self) -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<K, V> Debug for Last<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Last").finish()
    }
}
