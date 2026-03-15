use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

use super::{super::strategy::CloneStrategy, with_strategy::WithStrategy};

/// A collector that collects all outputs produced by an inner collector.
///
/// This `struct` is created by [`CollectorBase::nest()`]. See its documentation for more.
// Needed because the "Available on crate feature" does not show up on doc.rs
#[cfg_attr(docsrs, doc(cfg(feature = "unstable")))]
#[derive(Clone)]
pub struct Nest<CO, CI>(WithStrategy<CO, CloneStrategy<CI>>)
where
    CI: CollectorBase + Clone;

impl<CO, CI> Nest<CO, CI>
where
    CO: CollectorBase,
    CI: CollectorBase + Clone,
{
    pub(in crate::collector) fn new(outer: CO, inner: CI) -> Self {
        Self(WithStrategy::new(outer, CloneStrategy::new(inner)))
    }
}

impl<CO, CI> CollectorBase for Nest<CO, CI>
where
    CO: Collector<CI::Output>,
    CI: CollectorBase + Clone,
{
    type Output = CO::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0.finish()
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.0.break_hint()
    }
}

impl<CO, CI, T> Collector<T> for Nest<CO, CI>
where
    CO: Collector<CI::Output>,
    CI: Collector<T> + Clone,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.0.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.0.collect_many(items)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.0.collect_then_finish(items)
    }
}

impl<CO, CI> Debug for Nest<CO, CI>
where
    CO: Debug,
    CI: CollectorBase + Clone + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("Nest");
        self.0.debug_struct(&mut debug_struct);
        debug_struct.finish()
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
            nums in propvec(any::<i32>(), ..=10),
            row in ..=3_usize,
            column in 1..=3_usize,
        ) {
            all_collect_methods_impl(nums, row, column)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>, row: usize, column: usize) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                vec![]
                    .into_collector()
                    .take(row)
                    .nest(vec![].into_collector().take(column))
            },
            should_break_pred: |iter| iter.count() >= row * column,
            pred: |_, output, remaining| {
                if output
                    != nums
                        .chunks(column)
                        .take(row)
                        .map(Vec::from)
                        .collect::<Vec<_>>()
                {
                    Err(PredError::IncorrectOutput)
                } else if nums.iter().copied().skip(row * column).ne(remaining) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
