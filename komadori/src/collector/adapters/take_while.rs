use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// A collector that accumulates items as long as a predicate returns `true`.
///
/// This `struct` is created by [`CollectorBase::take_while()`]. See its documentation for more.
#[derive(Clone)]
pub struct TakeWhile<C, F> {
    collector: C,
    pred: F,
}

impl<C, F> TakeWhile<C, F> {
    pub(in crate::collector) fn new(collector: C, pred: F) -> Self {
        Self { collector, pred }
    }
}

impl<C, F> CollectorBase for TakeWhile<C, F>
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
        // Despite short-circuiting due to the predicate, we can't
        // do anything besides delegating to the underlying collector.
        self.collector.break_hint()
    }
}

impl<C, T, F> Collector<T> for TakeWhile<C, F>
where
    C: Collector<T>,
    F: FnMut(&T) -> bool,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if (self.pred)(&item) {
            self.collector.collect(item)
        } else {
            ControlFlow::Break(())
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // Be careful! The underlying collector may stop before the predicate return false.
        let mut all_true = true;
        let cf = self
            .collector
            .collect_many(items.into_iter().take_while(|item| {
                // We trust the implementation of the standard library and the collector.
                // They should short-circuit on the first false.
                all_true = (self.pred)(item);
                all_true
            }));

        if all_true { cf } else { ControlFlow::Break(()) }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().take_while(self.pred))
    }
}

impl<C: Debug, F> Debug for TakeWhile<C, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TakeWhile")
            .field("collector", &self.collector)
            .field("pred", &std::any::type_name::<F>())
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

    // Precondition:
    // - `Vec::IntoCollector`
    // - `Collector::take()`
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
                    .take_while(take_while_pred)
            },
            should_break_pred: |iter| {
                iter.clone().count() >= take_count || !iter.clone().all(|num| take_while_pred(&num))
            },
            pred: |mut iter, output, remaining| {
                if output
                    != iter
                        .by_ref()
                        .take_while(take_while_pred)
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

    fn take_while_pred(&num: &i32) -> bool {
        num > 0
    }
}
