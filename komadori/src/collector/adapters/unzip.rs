use std::ops::ControlFlow;

use crate::collector::{
    Collector, CollectorBase, Fuse, advanced_collect_many_default_impl, and_break,
};

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
    fn reserve(&mut self, additional: usize) {
        self.collector1.reserve(additional);
        self.collector2.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        // `max`, not `min`.
        // Even if one stops, the other still proceeds.
        self.collector1
            .max_afford(request)
            .max(self.collector2.max_afford(request))
    }
}

impl<C1, C2, T1, T2> Collector<(T1, T2)> for Unzip<C1, C2>
where
    C1: Collector<T1>,
    C2: Collector<T2>,
{
    #[inline]
    fn collect(&mut self, (item1, item2): (T1, T2)) -> ControlFlow<()> {
        let cf1 = self.collector1.collect(item1);
        let cf2 = self.collector2.collect(item2);
        and_break(cf1, cf2)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, (item1, item2): (T1, T2)) -> ControlFlow<()> {
        // SAFETY: The caller has reserved at least 1 item.
        let cf1 = unsafe { self.collector1.assume_reserved_collect(item1) };
        // SAFETY: The caller has reserved at least 1 item.
        let cf2 = unsafe { self.collector2.assume_reserved_collect(item2) };

        and_break(cf1, cf2)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = (T1, T2)>) -> ControlFlow<()> {
        advanced_collect_many_default_impl(self, items)
    }

    // No meaningful override for this method.
    // fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {}
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
