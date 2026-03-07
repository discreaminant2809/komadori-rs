use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// A collector that both filters and maps each item before collecting.
///
/// This `struct` is created by [`CollectorBase::filter_map()`].
/// See its documentation for more.
pub struct FilterMap<C, P> {
    collector: C,
    pred: P,
}

impl<C, P> FilterMap<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self { collector, pred }
    }
}

impl<C, P> CollectorBase for FilterMap<C, P>
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

impl<C, T, P, R> Collector<T> for FilterMap<C, P>
where
    C: Collector<R>,
    P: FnMut(T) -> Option<R>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if let Some(item) = (self.pred)(item) {
            self.collector.collect(item)
        } else {
            self.collector.break_hint()
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collector
            .collect_many(items.into_iter().filter_map(&mut self.pred))
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().filter_map(self.pred))
    }
}

impl<C, P> Debug for FilterMap<C, P>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterMap")
            .field("collector", &self.collector)
            .field("pred", &std::any::type_name::<P>())
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
                    .filter_map(|num: i32| num.checked_add(i32::MAX))
            },
            should_break_pred: |iter| {
                iter.filter_map(|num| num.checked_add(i32::MAX)).count() >= take_count
            },
            pred: |mut iter, output, remaining| {
                let expected = iter
                    .by_ref()
                    .filter_map(|num| num.checked_add(i32::MAX))
                    .take(take_count);

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
