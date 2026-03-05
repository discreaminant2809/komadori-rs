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

        // We should ensure that once the iterator ends, we never `next` it again.
        // We don't want to resume it.

        let mut items = items.into_iter();
        // We trust the implementation of `size_hint`.
        let (lower_sh, _) = items.size_hint();

        if self.remaining <= lower_sh {
            let n = std::mem::replace(&mut self.remaining, 0);
            return if drop_n_items(&mut items, n) {
                self.collector.collect_many(items)
            } else {
                ControlFlow::Continue(())
            };
        }

        self.remaining -= lower_sh;

        // Be careful: beyond the lower bound,
        // the iterator may end before skipping all `self.remaining`.
        let mut is_some = drop_n_items(&mut items, lower_sh);
        while is_some && self.remaining > 0 {
            self.remaining -= 1;
            is_some = items.next().is_some();
        }

        if is_some {
            self.collector.collect_many(items)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        if self.break_hint().is_break() {
            return self.collector.finish();
        }

        let mut items = items.into_iter();

        // `Iterator::skip()` is more strict in TrustedLen implementation,
        // so we manually skip items to preserve the len trustworthiness of the iterator.
        if drop_n_items(&mut items, self.remaining) {
            self.collector.collect_then_finish(items)
        } else {
            self.collector.finish()
        }
    }
}

// Returns `true` if all n items were dropped (not ended earlier).
// Should consult the returning `bool` to prevent the iterator from "resuming."
fn drop_n_items(items: &mut impl Iterator, n: usize) -> bool {
    if n > 0 {
        items.nth(n - 1).is_some()
    } else {
        true
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
            take_count in ..=9_usize,
            skip_count in ..=9_usize,
        ) {
            all_collect_methods_impl(nums1, nums2, take_count,skip_count)?;
        }
    }

    fn all_collect_methods_impl(
        nums1: Vec<i32>,
        nums2: Vec<i32>,
        take_count: usize,
        skip_count: usize,
    ) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || {
                nums1
                    .iter()
                    .copied()
                    .chain(nums2.iter().copied().filter(|&num| num > 0))
            },
            collector_factory: || vec![].into_collector().take(take_count).skip(skip_count),
            should_break_pred: |iter| {
                Dropping
                    .take(take_count)
                    .collect_many(iter.skip(skip_count))
                    .is_break()
            },
            pred: |mut iter, output, remaining| {
                if output
                    != iter
                        .by_ref()
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
