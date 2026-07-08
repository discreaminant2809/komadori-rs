use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that stops accumulating after collecting the first `n` items.
///
/// This `struct` is created by [`CollectorBase::take()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Take<C> {
    collector: C,
    // Unspecified if the underlying collector stops accumulating.
    remaining: usize,
}

impl<C> Take<C> {
    pub(in crate::collector) fn new(collector: C, n: usize) -> Self {
        Self {
            collector,
            remaining: n,
        }
    }
}

impl<C> CollectorBase for Take<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.collector.reserve(self.remaining.min(additional));
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request).min(self.remaining)
    }
}

impl<C, T> Collector<T> for Take<C>
where
    C: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        // Must NOT remove it. The user may construct with `take(0)` and
        // because it hasn't yielded Break, it shouldn't panic!
        if self.remaining == 0 {
            return ControlFlow::Break(());
        }

        self.remaining -= 1;
        let cf = self.collector.collect(item);

        if self.remaining == 0 {
            ControlFlow::Break(())
        } else {
            cf
        }
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        // Must NOT remove it. The user may construct with `take(0)` and
        // because it hasn't yielded Break, it shouldn't panic!
        if self.remaining == 0 {
            return ControlFlow::Break(());
        }

        self.remaining -= 1;
        let cf = unsafe {
            // SAFETY: Since `remaining` > 0, we've reserved at least 1 item.
            self.collector.assume_reserved_collect(item)
        };

        if self.remaining == 0 {
            ControlFlow::Break(())
        } else {
            cf
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // FIXED: utilize specialization after it's stabilized.

        let mut items = items.into_iter();
        let (lower_sh, _) = items.size_hint();

        // Implementation note: we trust the iterator's hint, but only for safe code.
        // No reserve calls and unsafe code for now because
        // we don't use `assume_reserved_collect()`,
        // but it's worth keeping this in mind.

        // The collector may end early. We risk tracking the state wrong?
        // Worry not. By then, the `remaining` becomes useless
        // and acts as a *soft* fuse.
        if self.remaining <= lower_sh {
            let n = std::mem::take(&mut self.remaining);
            let _ = self.collector.collect_many(items.take(n));
            return ControlFlow::Break(());
        }

        self.remaining -= lower_sh;
        self.collector.collect_many(items.by_ref().take(lower_sh))?;

        // We don't know how many left after the lower bound,
        // so we carefully track the state with `inspect`.
        let cf = self.collector.collect_many(
            items
                .take(self.remaining)
                // Since the collector may not collect all `remaining` items
                .inspect(|_| self.remaining -= 1),
        );

        if self.remaining == 0 {
            ControlFlow::Break(())
        } else {
            cf
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        // No need to track the state anymore. We'll be gone!
        self.collector
            .collect_then_finish(items.into_iter().take(self.remaining))
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
        #[test]
        fn all_collect_methods(
            // We keep just enough "space" for the take count to land on
            // each size hint interval.
            // The "diagram" is as below (E = when the take count is equal to either lower or upper bound)
            // 0 1 2 E 4 5 6 E 8 9
            nums1 in propvec(any::<i32>(), ..=3),
            nums2 in propvec(any::<i32>(), ..=4),
            take_count in ..=9_usize,
        ) {
            all_collect_methods_impl(nums1, nums2, take_count)?;
        }
    }

    fn all_collect_methods_impl(
        nums1: Vec<i32>,
        nums2: Vec<i32>,
        take_count: usize,
    ) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || {
                nums1
                    .iter()
                    .copied()
                    .chain(nums2.iter().copied().filter(|&num| num > 0))
            },
            collector_factory: || vec![].into_collector().take(take_count),
            should_break_pred: |iter| iter.count() >= take_count,
            pred: |mut iter, output, remaining| {
                if output != iter.by_ref().take(take_count).collect::<Vec<_>>() {
                    Err(PredError::IncorrectOutput)
                } else if !remaining.eq(iter) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
