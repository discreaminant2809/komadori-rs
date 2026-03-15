use std::{iter, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

use super::Fuse;

/// A collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`CollectorBase::tee_funnel()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct TeeFunnel<C1, C2> {
    collector1: Fuse<C1>,
    collector2: Fuse<C2>,
}

impl<C1, C2> TeeFunnel<C1, C2>
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

impl<C1, C2> CollectorBase for TeeFunnel<C1, C2>
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

impl<T, C1, C2> Collector<T> for TeeFunnel<C1, C2>
where
    C1: for<'a> Collector<&'a mut T>,
    C2: Collector<T>,
{
    #[inline]
    fn collect(&mut self, mut item: T) -> ControlFlow<()> {
        match (
            self.collector1.collect(&mut item),
            self.collector2.collect(item),
        ) {
            (ControlFlow::Break(_), ControlFlow::Break(_)) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    }

    // fn reserve(&mut self, additional_min: usize, additional_max: Option<usize>) {
    //     let (lower1, upper1) = self.collector1.size_hint();

    //     // Both have the same theme: the 2nd collector reserves the left-over amount.
    //     let (reserve_lower1, reserve_lower2) = if additional_min > lower1 {
    //         (lower1, additional_min - lower1)
    //     } else {
    //         (additional_min, 0)
    //     };

    //     let (reserve_upper1, reserve_upper2) = match (additional_max, upper1) {
    //         (Some(additional_max), Some(upper1)) if additional_max > upper1 => {
    //             (Some(upper1), Some(additional_max - upper1))
    //         }
    //         (additional_max, _) => (additional_max, Some(0)),
    //     };

    //     self.collector1.reserve(reserve_lower1, reserve_upper1);
    //     self.collector2.reserve(reserve_lower2, reserve_upper2);
    // }

    // fn size_hint(&self) -> (usize, Option<usize>) {
    //     let (lower1, upper1) = self.collector1.size_hint();
    //     let (lower2, upper2) = self.collector2.size_hint();

    //     (
    //         lower1.saturating_add(lower2),
    //         (|| upper1?.checked_add(upper2?))(),
    //     )
    // }

    // fn inactivity_hint(&self) -> Option<usize> {
    //     match (
    //         self.collector1.inactivity_hint(),
    //         self.collector2.inactivity_hint(),
    //     ) {
    //         (Some(count1), Some(count2)) => Some(count1.min(count2)),
    //         (Some(count), None) | (None, Some(count)) => Some(count),
    //         (None, None) => None,
    //     }
    // }

    // fn skip_till_active(&mut self, max: Option<usize>) {
    //     match (
    //         self.collector1.inactivity_hint(),
    //         self.collector2.inactivity_hint(),
    //     ) {
    //         (Some(count1), Some(count2)) => {
    //             let max = match max {
    //                 Some(max) => max.min(count1.min(count2)),
    //                 None => count1.min(count2),
    //             };

    //             self.collector1.skip_till_active(Some(max));
    //             self.collector2.skip_till_active(Some(max));
    //         }
    //         (Some(_), None) => {
    //             self.collector1.skip_till_active(max);
    //         }
    //         (None, Some(_)) => {
    //             self.collector2.skip_till_active(max);
    //         }
    //         (None, None) => {}
    //     }
    // }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.break_hint()?;

        let mut items = items.into_iter();

        match items.try_for_each(|mut item| {
            // We don't need to check like the `collect` implementation.
            // `self.break_hint()?` has already handled it,
            // and we trust that both underlying collectors
            // return `Break` as soon as it can't afford more items.
            if self.collector1.collect(&mut item).is_break() {
                ControlFlow::Break(Which::First(item))
            } else {
                self.collector2.collect(item).map_break(|_| Which::Second)
            }
        }) {
            ControlFlow::Break(Which::First(item)) => {
                self.collector2.collect_many(iter::once(item).chain(items))
            }
            ControlFlow::Break(Which::Second) => {
                // Sadly, we cannot use `collect_many` since we have an iterator of `T`,
                // but the collector expects `&mut T`, and we have no way to
                // map from `T` to `&mut T`.
                items.try_for_each(|mut item| self.collector1.collect(&mut item))
            }
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
        }
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        if self.break_hint().is_break() {
            return self.finish();
        }

        let mut items = items.into_iter();

        match items.try_for_each(|mut item| {
            // We don't need to check like the `collect` implementation.
            // `self.break_hint()?` has already handled it,
            // and we trust that both underlying collectors
            // return `Break` as soon as it can't afford more items.
            if self.collector1.collect(&mut item).is_break() {
                ControlFlow::Break(Which::First(item))
            } else {
                self.collector2.collect(item).map_break(|_| Which::Second)
            }
        }) {
            // If one of the collectors has stopped, we can avoid cloning
            // for the rest of the items!
            ControlFlow::Break(Which::First(item)) => (
                self.collector1.finish(),
                self.collector2
                    .collect_then_finish(iter::once(item).chain(items)),
            ),
            ControlFlow::Break(Which::Second) => {
                let _ = items.try_for_each(|mut item| self.collector1.collect(&mut item));
                self.finish()
            }
            ControlFlow::Continue(_) => self.finish(),
        }
    }
}

enum Which<T> {
    First(T),
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
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                vec![]
                    .into_collector()
                    .copying()
                    .take(first_count)
                    .tee_funnel(vec![].into_collector().take(second_count))
            },
            should_break_pred: |iter| iter.count() >= first_count.max(second_count),
            pred: |iter, output, remaining| {
                let first = iter.clone().take(first_count).collect::<Vec<_>>();
                let second = iter.clone().take(second_count).collect::<Vec<_>>();
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
