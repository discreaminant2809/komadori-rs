use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, Fuse};

/// A collector that destructures each 2-tuple `(A, B)` item and distributes its fields:
/// `A` goes to the first collector, and `B` goes to the second collector.
///
/// This `struct` is created by [`CollectorBase::unzip()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Unzip<C1, C2> {
    // `Fuse` is neccessary since either may end earlier.
    // It can ease the implementation.
    collector1: Fuse<C1>,
    collector2: Fuse<C2>,
}

impl<C1, C2> Unzip<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            collector1: Fuse::new(collector1),
            collector2: Fuse::new(collector2),
        }
    }
}

impl<C1, C2> CollectorBase for Unzip<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    type Output = (C1::Output, C2::Output);

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

impl<C1, C2, T1, T2> Collector<(T1, T2)> for Unzip<C1, C2>
where
    C1: Collector<T1>,
    C2: Collector<T2>,
{
    fn collect(&mut self, (item1, item2): (T1, T2)) -> ControlFlow<()> {
        let res1 = self.collector1.collect(item1);
        let res2 = self.collector2.collect(item2);

        if res1.is_break() && res2.is_break() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = (T1, T2)>) -> ControlFlow<()> {
        // Avoid consuming one item prematurely.
        self.break_hint()?;

        let mut items = items.into_iter();

        match items.try_for_each(|(item1, item2)| {
            if self.collector1.collect(item1).is_break() {
                return ControlFlow::Break(Which::First { item2 });
            }

            self.collector2.collect(item2).map_break(|_| Which::Second)
        }) {
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
            ControlFlow::Break(Which::First { item2 }) => self
                .collector2
                .collect_many(Some(item2).into_iter().chain(items.map(|(_, item2)| item2))),
            ControlFlow::Break(Which::Second) => {
                self.collector1.collect_many(items.map(|(item1, _)| item1))
            }
        }
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = (T1, T2)>) -> Self::Output {
        // Avoid consuming one item prematurely.
        if self.break_hint().is_break() {
            return self.finish();
        }

        let mut items = items.into_iter();

        match items.try_for_each(|(item1, item2)| {
            if self.collector1.collect(item1).is_break() {
                return ControlFlow::Break(Which::First { item2 });
            }

            self.collector2.collect(item2).map_break(|_| Which::Second)
        }) {
            ControlFlow::Continue(_) => self.finish(),
            ControlFlow::Break(Which::First { item2 }) => (
                self.collector1.finish(),
                self.collector2.collect_then_finish(
                    Some(item2).into_iter().chain(items.map(|(_, item2)| item2)),
                ),
            ),
            ControlFlow::Break(Which::Second) => (
                self.collector1
                    .collect_then_finish(items.map(|(item1, _)| item1)),
                self.collector2.finish(),
            ),
        }
    }
}

enum Which<T> {
    First { item2: T },
    Second,
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    proptest! {
        /// Since `unzip()` is essentially just `combine()` (but used for destructuring),
        /// we can just copy the test from there to here.
        ///
        /// Precondition:
        /// - [`crate::collector::Collector::take()`]
        /// - [`crate::vec::IntoCollector`]
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=4),
            first_count in ..=4_usize,
            second_count in ..=4_usize,
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
            iter_factory: || nums.iter().map(|&num| (num, num)),
            collector_factory: || {
                vec![]
                    .into_collector()
                    .take(first_count)
                    .unzip(vec![].into_collector().take(second_count))
            },
            should_break_pred: |iter| iter.count() >= first_count.max(second_count),
            pred: |iter, output, remaining| {
                let first = nums.iter().copied().take(first_count).collect::<Vec<_>>();
                let second = nums.iter().copied().take(second_count).collect::<Vec<_>>();
                let max_len = first_count.max(second_count);

                if output != (first, second) {
                    Err(PredError::IncorrectOutput)
                } else if iter.skip(max_len).ne(remaining) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
