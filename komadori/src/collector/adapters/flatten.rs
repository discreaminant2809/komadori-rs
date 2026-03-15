use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that flattens items by one level of nesting before collecting.
///
/// This `struct` is created by [`CollectorBase::flatten()`]. See its documentation for more.
#[derive(Clone, Debug)]
pub struct Flatten<C> {
    collector: C,
}

impl<C> Flatten<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self { collector }
    }
}

impl<C> CollectorBase for Flatten<C>
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

impl<C, I> Collector<I> for Flatten<C>
where
    C: Collector<I::Item>,
    I: IntoIterator,
{
    #[inline]
    fn collect(&mut self, item: I) -> ControlFlow<()> {
        self.collector.collect_many(item)
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = I>) -> ControlFlow<()> {
        self.collector.collect_many(items.into_iter().flatten())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = I>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().flatten())
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
            matrix in propvec(propvec(any::<i32>(), ..=3), ..=3),
            take_count in 0..=10_usize,
        ) {
            all_collect_methods_impl(matrix, take_count)?;
        }
    }

    fn all_collect_methods_impl(matrix: Vec<Vec<i32>>, take_count: usize) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || matrix.iter(),
            collector_factory: || vec![].into_collector().take(take_count).flatten(),
            should_break_pred: |iter| iter.flatten().count() >= take_count,
            pred: |mut iter, output: Vec<i32>, remaining| {
                if iter.by_ref().flatten().take(take_count).ne(&output) {
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
