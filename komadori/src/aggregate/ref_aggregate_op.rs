use crate::aggregate::AggregateOp;

/// An [`AggregateOp`] that can also operates on a mutable reference to an item.
pub trait RefAggregateOp: AggregateOp {
    /// Creates a new value for a newly created group.
    ///
    /// See [`AggregateOp::new_value()`] for more detail.
    fn new_value_ref(&mut self, key: &Self::Key, item: &mut Self::Item) -> Self::Value;

    /// Creates a new value for a newly created group.
    ///
    /// See [`AggregateOp::modify()`] for more detail.
    fn modify_ref(&mut self, value: &mut Self::Value, item: &mut Self::Item);
}
