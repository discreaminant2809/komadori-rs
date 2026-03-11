use std::{fmt::Debug, ops::ControlFlow};

use itertools::Itertools;

use crate::collector::{Collector, CollectorBase};

/// A collector that calls a closure on each item before collecting.
///
/// This `struct` is created by [`CollectorBase::update()`]. See its documentation for more.
#[derive(Clone)]
pub struct Update<C, F> {
    collector: C,
    f: F,
}

impl<C, F> Update<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self { collector, f }
    }
}

impl<C, F> CollectorBase for Update<C, F>
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

impl<C, T, F> Collector<T> for Update<C, F>
where
    C: Collector<T>,
    F: FnMut(&mut T),
{
    fn collect(&mut self, mut item: T) -> ControlFlow<()> {
        (self.f)(&mut item);
        self.collector.collect(item)
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collector
            .collect_many(items.into_iter().update(&mut self.f))
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().update(self.f))
    }
}

impl<C: Debug, F> Debug for Update<C, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Update")
            .field("collector", &self.collector)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use itertools::Itertools;
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    // Precondition:
    // - `Vec::IntoCollector`
    proptest! {
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
                    // Be careful of overflowing!
                    .update(|num: &mut i32| *num = num.wrapping_add(1))
            },
            should_break_pred: |_| nums.len() >= take_count,
            pred: |mut iter, output, remaining| {
                if iter
                    .by_ref()
                    .update(|num| *num = num.wrapping_add(1))
                    .take(take_count)
                    .ne(output)
                {
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
