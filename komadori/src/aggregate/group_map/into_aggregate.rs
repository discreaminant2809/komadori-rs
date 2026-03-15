use std::ops::ControlFlow;

use crate::{
    aggregate::{AggregateOp, Group, GroupMap, OccupiedGroup, VacantGroup},
    assert_collector,
    collector::Collector,
};

/// A [`Collector`] that aggregates items into groups.
///
/// This `struct` is created by [`GroupMap::into_aggregate()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct IntoAggregate<M, Op> {
    map: M,
    op: Op,
}

impl<M, Op> IntoAggregate<M, Op>
where
    M: GroupMap,
    Op: AggregateOp<Key = M::Key, Value = M::Value>,
{
    #[inline]
    pub(super) fn new(map: M, op: Op) -> Self {
        assert_collector(Self { map, op })
    }

    fn collect_impl(&mut self, key: M::Key, item: Op::Item) {
        match self.map.group(key) {
            Group::Occupied(mut entry) => self.op.modify(entry.value_mut(), item),
            Group::Vacant(entry) => {
                let value = self.op.new_value(entry.key(), item);
                entry.insert(value);
            }
        }
    }
}

impl<M, Op> Collector for IntoAggregate<M, Op>
where
    M: GroupMap,
    Op: AggregateOp<Key = M::Key, Value = M::Value>,
{
    type Item = (M::Key, Op::Item);

    type Output = M;

    #[inline]
    fn collect(&mut self, (key, item): Self::Item) -> ControlFlow<()> {
        self.collect_impl(key, item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn finish(self) -> Self::Output {
        self.map
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = Self::Item>) -> ControlFlow<()> {
        items
            .into_iter()
            .for_each(|(key, item)| self.collect_impl(key, item));

        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = Self::Item>) -> Self::Output {
        items
            .into_iter()
            .for_each(|(key, item)| self.collect_impl(key, item));

        self.map
    }
}
