use std::{fmt::Debug, marker::PhantomData};

use crate::aggregate::{AggregateOp, RefAggregateOp};

/// A [`RefAggregateOp`] that that calls a closure on each item before operating on.
///
/// This `struct` is created by [`AggregateOp::map_ref()`]. See its documentation for more.
pub struct MapRef<Op, T, F> {
    op: Op,
    f: F,
    _marker: PhantomData<fn(&mut T)>,
}

impl<Op, T, F> MapRef<Op, T, F> {
    pub(super) fn new(op: Op, f: F) -> Self {
        Self {
            op,
            f,
            _marker: PhantomData,
        }
    }
}

impl<Op, T, F> AggregateOp for MapRef<Op, T, F>
where
    Op: AggregateOp,
    F: FnMut(&mut T) -> Op::Item,
{
    type Key = Op::Key;

    type Value = Op::Value;

    type Item = T;

    #[inline]
    fn new_value(&mut self, key: &Self::Key, mut item: Self::Item) -> Self::Value {
        self.new_value_ref(key, &mut item)
    }

    #[inline]
    fn modify(&mut self, value: &mut Self::Value, mut item: Self::Item) {
        self.modify_ref(value, &mut item);
    }
}

impl<Op, T, F> RefAggregateOp for MapRef<Op, T, F>
where
    Op: AggregateOp,
    F: FnMut(&mut T) -> Op::Item,
{
    #[inline]
    fn new_value_ref(&mut self, key: &Self::Key, item: &mut Self::Item) -> Self::Value {
        self.op.new_value(key, (self.f)(item))
    }

    #[inline]
    fn modify_ref(&mut self, value: &mut Self::Value, item: &mut Self::Item) {
        self.op.modify(value, (self.f)(item));
    }
}

impl<Op, T, F> Clone for MapRef<Op, T, F>
where
    Op: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            op: self.op.clone(),
            f: self.f.clone(),
            _marker: PhantomData,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.op.clone_from(&source.op);
        self.f.clone_from(&source.f);
    }
}

impl<Op, T, F> Debug for MapRef<Op, T, F>
where
    Op: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapRef").field("op", &self.op).finish()
    }
}
