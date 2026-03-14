use crate::collector::{Collector, CollectorBase};

use std::{fmt::Debug, ops::ControlFlow};

/// A collector that uses a closure to determine whether an item should be collected.
///
/// This `struct` is created by [`CollectorBase::filter()`]. See its documentation for more.
#[derive(Clone)]
pub struct Filter<C, F> {
    collector: C,
    pred: F,
}

impl<C, F> Filter<C, F> {
    pub(in crate::collector) fn new(collector: C, pred: F) -> Self {
        Self { collector, pred }
    }
}

impl<C, F> CollectorBase for Filter<C, F>
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

impl<C, F, T> Collector<T> for Filter<C, F>
where
    C: Collector<T>,
    F: FnMut(&T) -> bool,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if (self.pred)(&item) {
            self.collector.collect(item)
        } else {
            self.collector.break_hint()
        }
    }

    // Removed the overriden implementations cuz the items here are being consumed
    // without consulting the underlying collector's break hint during filtering.
    // Yes, the performance degrades, but it's because of `try_for_each()` and/or
    // LLVM noise (which could be fixed soon),
    // and in multiple reduction it still works well and performs similarly to fold().
}

impl<C: Debug, F> Debug for Filter<C, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filter")
            .field("collector", &self.collector)
            .finish()
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
            nums in propvec(any::<i32>(), ..=5),
            take_count in ..=5_usize,
        ) {
            all_collect_methods_impl(nums, take_count)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>, take_count: usize) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                vec![]
                    .into_collector()
                    .take(take_count)
                    .filter(|&num| num >= 0)
            },
            should_break_pred: |iter| iter.filter(|&num| num >= 0).count() >= take_count,
            pred: |mut iter, output, remaining| {
                let expected = iter.by_ref().filter(|&num| num >= 0).take(take_count);

                if expected.ne(output) {
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
