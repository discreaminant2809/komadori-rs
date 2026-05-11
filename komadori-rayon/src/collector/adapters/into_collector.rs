use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::{ParallelCollector, ParallelCollectorBase};

/// A (serial) collector created from a parallel collector.
///
/// This `struct` is created by [`ParallelCollectorBase::into_collector()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct IntoCollector<C> {
    par_collector: C,
}

impl<C> IntoCollector<C> {
    pub(in crate::collector) fn new(par_collector: C) -> Self {
        Self { par_collector }
    }
}

impl<C> CollectorBase for IntoCollector<C>
where
    C: ParallelCollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.par_collector.finish()
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.par_collector.break_hint()
    }
}

impl<C, T> Collector<T> for IntoCollector<C>
where
    C: ParallelCollector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        let (_, consumer, commit) = self.par_collector.parts(1);
        let mut collector = consumer.into_collector();
        let _ = collector.collect(item);
        commit(collector.finish())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // If we ever have specialization stabilized, we will use it.
        // Or else, we should only accept (indexed) parallel collectors
        // for maximum generalization (also work for unindexed ones).

        let mut items = items.into_iter();
        let (lower_sh, _) = items.size_hint();

        // We can guarantee this amount of items till the lower size hint.
        let (_, consumer, commit) = self.par_collector.parts(lower_sh);
        let collector = consumer.into_collector();
        let output = collector.collect_then_finish(items.by_ref().take(lower_sh));
        commit(output)?;

        items.try_for_each(|item| self.collect(item))
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let mut items = items.into_iter();
        let (lower_sh, upper_sh) = items.size_hint();

        if Some(lower_sh) == upper_sh {
            // The iterator's size is exact. We can use `take_parts()` and done!
            let (_, consumer, commit) = self.par_collector.take_parts(lower_sh);
            let collector = consumer.into_collector();
            let output = collector.collect_then_finish(items.take(lower_sh));
            commit(output);
            return self.finish();
        }

        let (_, consumer, commit) = self.par_collector.parts(lower_sh);
        let collector = consumer.into_collector();
        let output = collector.collect_then_finish(items.by_ref().take(lower_sh));
        if commit(output).is_continue() {
            // After the lower size hint, we literally can't assume anything now.
            let _ = items.try_for_each(|item| self.collect(item));
        }

        self.finish()
    }
}
