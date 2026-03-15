use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, Fuse};

/// A collector that feeds the first collector until it stop accumulating,
/// then feeds the second collector.
///
/// This `struct` is created by [`CollectorBase::chain()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Chain<C1, C2> {
    collector1: Fuse<C1>,
    collector2: C2,
}

impl<C1, C2> Chain<C1, C2>
where
    C1: CollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            collector1: collector1.fuse(),
            collector2,
        }
    }
}

impl<C1, C2> CollectorBase for Chain<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    type Output = (C1::Output, C2::Output);

    #[inline]
    fn finish(self) -> Self::Output {
        (self.collector1.finish(), self.collector2.finish())
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        // We're sure that whether this collector has finished or not is
        // entirely based on the 2nd collector.
        // Also, by this method being called it is assumed that
        // this collector has not finished, which mean the 2nd collector
        // has not finished, which means it's always sound to call here.
        //
        // Since the 1st collector is fused, we won't cause any unsoundness
        // by repeatedly calling it.
        if self.collector1.break_hint().is_break() && self.collector2.break_hint().is_break() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

impl<T, C1, C2> Collector<T> for Chain<C1, C2>
where
    C1: Collector<T>,
    C2: Collector<T>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if self.collector1.break_hint().is_break() {
            self.collector2.collect(item)
        } else if self.collector1.collect(item).is_continue() {
            ControlFlow::Continue(())
        } else {
            self.collector2.break_hint()
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        let mut items = items.into_iter();

        // No need to consult the `break_hint`
        if self.collector1.collect_many(&mut items).is_break() {
            self.collector2.collect_many(items)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let mut items = items.into_iter();

        // No need to consult the `break_hint`
        (
            self.collector1.collect_then_finish(&mut items),
            self.collector2.collect_then_finish(items),
        )
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    proptest! {
        /// Precondition:
        /// - [`crate::collector::Collector::take()`]
        /// - [`crate::vec::IntoCollector`]
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=7),
            first_count in 0..=3_usize,
            second_count in 0..=3_usize,
        ) {
            all_collect_methods_impl(nums, first_count, second_count)?;
        }
    }

    fn all_collect_methods_impl(
        nums: Vec<i32>,
        first_count: usize,
        second_count: usize,
    ) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                vec![]
                    .into_collector()
                    .take(first_count)
                    .chain(vec![].into_collector().take(second_count))
            },
            should_break_pred: |iter| iter.count() >= first_count + second_count,
            pred: |mut iter, output, remaining| {
                let first = iter.by_ref().take(first_count).collect::<Vec<_>>();
                let second = iter.by_ref().take(second_count).collect::<Vec<_>>();

                if output != (first, second) {
                    Err(PredError::IncorrectOutput)
                } else if iter.ne(remaining) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
