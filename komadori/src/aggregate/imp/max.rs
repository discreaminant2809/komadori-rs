use std::{fmt::Debug, marker::PhantomData};

use crate::aggregate::{AggregateOp, assert_op};

/// An [`AggregateOp`] that set the maximum value among items it operated on.
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
///     .into_aggregate(aggregate::Max::new());
///
/// assert!(collector.collect((1, 1)).is_continue());
/// assert!(collector.collect((1, 4)).is_continue());
/// assert!(collector.collect((2, 1)).is_continue());
/// assert!(collector.collect((1, 2)).is_continue());
/// assert!(collector.collect((2, 3)).is_continue());
///
/// let counts = collector.finish();
///
/// assert_eq!(counts[&1], 4);
/// assert_eq!(counts[&2], 3);
/// ```
pub struct Max<K, V> {
    _marker: PhantomData<fn(&K, V, &mut V) -> V>,
}

impl<K, V: Ord> Max<K, V> {
    /// Creates a new instance of this aggregate op.
    #[inline]
    pub const fn new() -> Self {
        assert_op(Self {
            _marker: PhantomData,
        })
    }
}

impl<K, V: Ord> AggregateOp for Max<K, V> {
    type Key = K;

    type Value = V;

    type Item = V;

    fn new_value(&mut self, _key: &Self::Key, item: Self::Item) -> Self::Value {
        item
    }

    fn modify(&mut self, value: &mut Self::Value, item: Self::Item) {
        // As long as there's no `unsafe` block, there's no UB.
        let value_ptr = value as *const _;
        let max = (&*value).max(&item);
        if !std::ptr::eq(value_ptr, max) {
            *value = item;
        }
    }
}

impl<K, V: Ord> Default for Max<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Clone for Max<K, V> {
    fn clone(&self) -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<K, V> Debug for Max<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Max").finish()
    }
}
