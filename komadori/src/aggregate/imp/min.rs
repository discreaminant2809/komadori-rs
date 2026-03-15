use std::{fmt::Debug, marker::PhantomData};

use crate::aggregate::{AggregateOp, assert_op};

/// An [`AggregateOp`] that set the minimum value among items it operated on.
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
///     .into_aggregate(aggregate::Min::new());
///
/// assert!(collector.collect((1, 3)).is_continue());
/// assert!(collector.collect((1, 2)).is_continue());
/// assert!(collector.collect((2, 1)).is_continue());
/// assert!(collector.collect((1, 4)).is_continue());
/// assert!(collector.collect((2, 3)).is_continue());
///
/// let counts = collector.finish();
///
/// assert_eq!(counts[&1], 2);
/// assert_eq!(counts[&2], 1);
/// ```
pub struct Min<K, V> {
    _marker: PhantomData<fn(&K, V, &mut V) -> V>,
}

impl<K, V: Ord> Min<K, V> {
    /// Creates a new instance of this aggregate op.
    #[inline]
    pub const fn new() -> Self {
        assert_op(Self {
            _marker: PhantomData,
        })
    }
}

impl<K, V: Ord> AggregateOp for Min<K, V> {
    type Key = K;

    type Value = V;

    type Item = V;

    fn new_value(&mut self, _key: &Self::Key, item: Self::Item) -> Self::Value {
        item
    }

    fn modify(&mut self, value: &mut Self::Value, item: Self::Item) {
        // As long as there's no `unsafe` block, there's no UB.
        let value_ptr = value as *const _;
        let min = (&*value).min(&item);
        if !std::ptr::eq(value_ptr, min) {
            *value = item;
        }
    }
}

impl<K, V: Ord> Default for Min<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Clone for Min<K, V> {
    fn clone(&self) -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<K, V> Debug for Min<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Min").finish()
    }
}
