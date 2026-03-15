use crate::aggregate::{AggregateOp, RefAggregateOp};

/// A [`RefAggregateOp`] that [`clone`](Clone::clone)s every operated item.
///
/// This `struct` is created by [`AggregateOp::cloning()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Cloning<Op> {
    op: Op,
}

impl<Op> Cloning<Op> {
    pub(super) fn new(op: Op) -> Self {
        Self { op }
    }
}

impl<Op> AggregateOp for Cloning<Op>
where
    Op: AggregateOp,
{
    type Key = Op::Key;

    type Value = Op::Value;

    type Item = Op::Item;

    #[inline]
    fn new_value(&mut self, key: &Self::Key, item: Self::Item) -> Self::Value {
        self.op.new_value(key, item)
    }

    #[inline]
    fn modify(&mut self, value: &mut Self::Value, item: Self::Item) {
        self.op.modify(value, item);
    }
}

impl<Op> RefAggregateOp for Cloning<Op>
where
    Self::Item: Clone,
    Op: AggregateOp,
{
    #[inline]
    fn new_value_ref(&mut self, key: &Self::Key, item: &mut Self::Item) -> Self::Value {
        self.op.new_value(key, item.clone())
    }

    #[inline]
    fn modify_ref(&mut self, value: &mut Self::Value, item: &mut Self::Item) {
        self.op.modify(value, item.clone());
    }
}
