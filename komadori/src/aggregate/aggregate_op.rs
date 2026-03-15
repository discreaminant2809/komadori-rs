mod cloning;
mod combine;
mod map;
mod map_ref;

pub use cloning::*;
pub use combine::*;
pub use map::*;
pub use map_ref::*;

use crate::aggregate::{assert_op, assert_ref_op};

/// Defines group's entry manipulation.
pub trait AggregateOp {
    /// The group's key.
    type Key;

    /// The group's value.
    type Value;

    /// What the aggregation operates on?
    type Item;

    /// Creates a new value for a newly created group.
    ///
    /// It must accumulate the provided item right away, not just creating the "default" value.
    fn new_value(&mut self, key: &Self::Key, item: Self::Item) -> Self::Value;

    /// Modifies an existing group's value.
    ///
    /// The current limitations prevent us from providing the key of the group.
    /// That parameter may be added soon.
    fn modify(&mut self, value: &mut Self::Value, item: Self::Item);

    /// Creates an [`AggregateOp`] that that calls a closure on each item before operating on.
    ///
    /// This is used when [`Combine`] expects to operate on `T`,
    /// but you have an aggregate op that operates on `U`. In that case,
    /// you can use `map()` to transform `U` into `T` before passing it along.
    ///
    /// Since it does not implement [`RefAggregateOp`], this adaptor should be used
    /// on the final aggregate op in [`Combine`], or adapted into a [`RefAggregateOp`]
    /// using the appropriate adaptor.
    /// If you find yourself writing `map().cloning()` or `map().copying()`,
    /// consider using [`map_ref()`](AggregateOp::map_ref) instead, which avoids unnecessary cloning.
    ///
    /// # Examples
    ///
    /// [`RefAggregateOp`]: super::RefAggregateOp
    #[inline]
    fn map<T, F>(self, f: F) -> Map<Self, T, F>
    where
        Self: Sized,
        F: FnMut(T) -> Self::Item,
    {
        assert_op(Map::new(self, f))
    }

    /// Creates a [`RefAggregateOp`] that that calls a closure on each item before operating on.
    ///
    /// This is used when [`Combine`] expects to operate on `T`,
    /// but you have an aggregate op that operates on `U`. In that case,
    /// you can use `map_ref()` to transform `U` into `T` before passing it along.
    ///
    /// Unlike [`map()`](AggregateOp::map), this adaptor only receives a mutable reference to each item.
    /// Because of that, it can be used in the middle of [`Combine`],
    /// since it is a [`RefAggregateOp`].
    /// While it can also appear at the end of [`Combine`], consider using [`map()`](AggregateOp::map) there
    /// instead for better clarity.
    ///
    /// # Examples
    ///
    /// [`RefAggregateOp`]: super::RefAggregateOp
    #[inline]
    fn map_ref<T, F>(self, f: F) -> MapRef<Self, T, F>
    where
        Self: Sized,
        F: FnMut(&mut T) -> Self::Item,
    {
        assert_ref_op(MapRef::new(self, f))
    }

    /// Creates a [`RefAggregateOp`] that [`clone`](Clone::clone)s every operated item.
    ///
    /// This is useful when you need ownership of items, but you still want the agregate op
    /// to be in the middle of [`Combine`].
    ///
    /// As a [`AggregateOp`], `cloning()` does nothing (effectively a no-op) and is usually useless
    /// at the end of [`Combine`].
    /// It only performs its intended behavior when used as a [`RefAggregateOp`].
    ///
    /// # Examples
    ///
    /// [`RefAggregateOp`]: super::RefAggregateOp
    #[inline]
    fn cloning(self) -> Cloning<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        assert_ref_op(Cloning::new(self))
    }
}
