use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that skips the first `n` collected items before it begins
/// accumulating them.
///
/// This `struct` is created by [`CollectorBase::skip()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Skip<C> {
    collector: C,
    remaining: usize,
}

impl<C> Skip<C> {
    pub(in crate::collector) fn new(collector: C, n: usize) -> Self {
        Self {
            collector,
            remaining: n,
        }
    }
}

impl<C> CollectorBase for Skip<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.collector.break_hint()
    }
}

impl<C, T> Collector<T> for Skip<C>
where
    C: Collector<T>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if self.remaining == 0 {
            return self.collector.collect(item);
        }

        self.remaining -= 1;
        self.collector.break_hint()
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // Unlike `Collector::take()`, a guard is needed because we drop
        // items (via `drop_n_items`) before forwarding to the underlying collector.
        self.break_hint()?;

        let mut items = items.into_iter();
        let (lower_sh, _) = items.size_hint();

        if self.remaining <= lower_sh {
            items
                .by_ref()
                .take(std::mem::take(&mut self.remaining))
                .try_for_each(|_| self.collector.break_hint())?;

            return self.collector.collect_many(items);
        }

        self.remaining -= lower_sh;
        items
            .by_ref()
            .take(lower_sh)
            .try_for_each(|_| self.collector.break_hint())?;

        match items.by_ref().try_for_each(|_| {
            self.collector
                .break_hint()
                .map_break(|_| ControlFlow::Break(()))?;
            self.remaining -= 1;
            if self.remaining == 0 {
                ControlFlow::Break(ControlFlow::Continue(()))
            } else {
                ControlFlow::Continue(())
            }
        }) {
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
            ControlFlow::Break(ControlFlow::Break(_)) => ControlFlow::Break(()),
            ControlFlow::Break(ControlFlow::Continue(_)) => self.collector.collect_many(items),
        }
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        if self.break_hint().is_break() {
            return self.collector.finish();
        }

        let mut items = items.into_iter();
        let (lower_sh, _) = items.size_hint();

        if self.remaining <= lower_sh {
            return if items
                .by_ref()
                .take(std::mem::take(&mut self.remaining))
                .try_for_each(|_| self.collector.break_hint())
                .is_break()
            {
                self.collector.finish()
            } else {
                self.collector.collect_then_finish(items)
            };
        }

        self.remaining -= lower_sh;
        if items
            .by_ref()
            .take(lower_sh)
            .try_for_each(|_| self.collector.break_hint())
            .is_break()
        {
            return self.collector.finish();
        }

        match items.by_ref().try_for_each(|_| {
            self.collector
                .break_hint()
                .map_break(|_| ControlFlow::Break(()))?;

            self.remaining -= 1;
            if self.remaining == 0 {
                ControlFlow::Break(ControlFlow::Continue(()))
            } else {
                ControlFlow::Continue(())
            }
        }) {
            ControlFlow::Continue(_) | ControlFlow::Break(ControlFlow::Break(_)) => {
                self.collector.finish()
            }
            ControlFlow::Break(ControlFlow::Continue(_)) => {
                self.collector.collect_then_finish(items)
            }
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};
    use crate::{mem::Dropping, prelude::*};

    // We need to use `take()` to simulate the break case when enough items are skipped.
    // Precondition:
    // - `Vec::IntoCollector`
    // - `Collector::take()`
    // - `Dropping`
    proptest! {
        #[test]
        fn all_collect_methods(
            // We keep just enough "space" for the take count to land on
            // each size hint interval.
            // The "diagram" is as below (E = when the take count is equal to either lower or upper bound)
            // 0 1 2 E 4 5 6 E 8 9
            nums1 in propvec(any::<i32>(), ..=3),
            nums2 in propvec(any::<i32>(), ..=4),
            (take_count, skip_count, starting_nums) in (..=9_usize, ..=9_usize)
                .prop_flat_map(|(take_count, skip_count)| (
                    Just(take_count), Just(skip_count),
                    if take_count == 0 {
                        Just(None).boxed()
                    } else {
                        let total = skip_count + take_count;

                        // Less than the total, or else we can accidentally
                        // make the collector stops.
                        propvec(any::<i32>(), ..=(total - 1).min(2))
                            .prop_map(Some)
                            .boxed()
                    }
                )),
        ) {
            all_collect_methods_impl(nums1, nums2, take_count,skip_count, starting_nums)?;
        }
    }

    fn all_collect_methods_impl(
        nums1: Vec<i32>,
        nums2: Vec<i32>,
        take_count: usize,
        skip_count: usize,
        starting_nums: Option<Vec<i32>>,
    ) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || {
                nums1
                    .iter()
                    .copied()
                    .chain(nums2.iter().copied().filter(|&num| num > 0))
            },
            collector_factory: || {
                let mut collector = vec![].into_collector().take(take_count).skip(skip_count);
                if let Some(starting_nums) = &starting_nums {
                    assert!(
                        collector
                            .collect_many(starting_nums.iter().copied())
                            .is_continue()
                    );
                }
                collector
            },
            should_break_pred: |iter| {
                Dropping
                    .take(take_count)
                    .collect_many(
                        (starting_nums.iter().flatten().copied().chain(iter)).skip(skip_count),
                    )
                    .is_break()
            },
            pred: |mut iter, output, remaining| {
                if output
                    != starting_nums
                        .iter()
                        .flatten()
                        .copied()
                        .chain(&mut iter)
                        .skip(skip_count)
                        .take(take_count)
                        .collect::<Vec<_>>()
                {
                    Err(PredError::IncorrectOutput)
                } else if !iter.eq(remaining) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
