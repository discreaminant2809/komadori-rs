use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// A collector that collects elements in each iterator item provided by a closure.
///
/// This `struct` is created by [`CollectorBase::flat_map()`]. See its documentation for more.
#[derive(Clone)]
pub struct FlatMap<C, F> {
    collector: C,
    f: F,
}

impl<C, F> FlatMap<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self { collector, f }
    }
}

impl<C, F> CollectorBase for FlatMap<C, F>
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

impl<C, T, I, F> Collector<T> for FlatMap<C, F>
where
    C: Collector<I::Item>,
    F: FnMut(T) -> I,
    I: IntoIterator,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.collector.collect_many((self.f)(item))
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collector
            .collect_many(items.into_iter().flat_map(&mut self.f))
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().flat_map(self.f))
    }
}

impl<C: Debug, F> Debug for FlatMap<C, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlatMap")
            .field("collector", &self.collector)
            .field("f", &std::any::type_name::<F>())
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
            matrix in propvec(propvec(any::<i32>(), ..=3), ..=3),
            take_count in 0..=10_usize,
        ) {
            all_collect_methods_impl(matrix, take_count)?;
        }
    }

    fn all_collect_methods_impl(matrix: Vec<Vec<i32>>, take_count: usize) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || matrix.iter(),
            collector_factory: || vec![].into_collector().take(take_count).flat_map(flat_fn),
            should_break_pred: |iter| iter.flat_map(flat_fn).count() >= take_count,
            pred: |mut iter, output: Vec<i32>, remaining| {
                if iter.by_ref().flat_map(flat_fn).take(take_count).ne(&output) {
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

    #[allow(clippy::ptr_arg)]
    fn flat_fn(row: &Vec<i32>) -> impl Iterator<Item = &i32> {
        row.iter()
    }
}
