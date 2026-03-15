use std::{fmt::Debug, iter, marker::PhantomData, ops::MulAssign};

use crate::aggregate::{AggregateOp, assert_op};

/// An [`AggregateOp`] that calculates the sum of items it operated on.
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
/// let mut collector = HashMap::<_, i32>::new()
///     .into_aggregate(aggregate::Product::new());
///
/// assert!(collector.collect((1, 1)).is_continue());
/// assert!(collector.collect((1, 4)).is_continue());
/// assert!(collector.collect((2, 1)).is_continue());
/// assert!(collector.collect((1, 2)).is_continue());
/// assert!(collector.collect((2, 3)).is_continue());
///
/// let counts = collector.finish();
///
/// assert_eq!(counts[&1], 8);
/// assert_eq!(counts[&2], 3);
/// ```
pub struct Product<K, V, T = V> {
    _marker: PhantomData<fn(&K, &mut V, T) -> V>,
}

impl<K, V, T> Product<K, V, T>
where
    V: iter::Product<T> + MulAssign<T>,
{
    /// Creates a new instance of this aggregate op.
    #[inline]
    pub const fn new() -> Self {
        assert_op(Self {
            _marker: PhantomData,
        })
    }
}

impl<K, V, T> AggregateOp for Product<K, V, T>
where
    V: iter::Product<T> + MulAssign<T>,
{
    type Key = K;

    type Value = V;

    type Item = T;

    #[inline]
    fn new_value(&mut self, _key: &Self::Key, item: Self::Item) -> Self::Value {
        let mut value = iter::empty().product();
        value *= item;
        value
    }

    #[inline]
    fn modify(&mut self, value: &mut Self::Value, item: Self::Item) {
        *value *= item;
    }
}

impl<K, V, T> Default for Product<K, V, T>
where
    V: iter::Product<T> + MulAssign<T>,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, T> Clone for Product<K, V, T> {
    fn clone(&self) -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<K, V, T> Debug for Product<K, V, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Product").finish()
    }
}
