use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that sets the [`Output`] to [`None`] when
/// a [`None`] item is encountered for the first time,
/// else the underlying collector collects the `item` inside
/// [`Some(item)`](Some).
///
/// This `struct` is created by [`CollectorBase::trying_options()`].
/// See its documentation for more.
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Clone)]
pub struct TryingOptions<C> {
    collector: Option<C>,
}

impl<C> TryingOptions<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self {
            collector: Some(collector),
        }
    }
}

impl<C> CollectorBase for TryingOptions<C>
where
    C: CollectorBase,
{
    type Output = Option<C::Output>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.map(CollectorBase::finish)
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.collector
            .as_ref()
            .map_or(ControlFlow::Break(()), |collector| collector.break_hint())
    }
}

impl<C, T> Collector<Option<T>> for TryingOptions<C>
where
    C: Collector<T>,
{
    fn collect(&mut self, item: Option<T>) -> ControlFlow<()> {
        match (item, &mut self.collector) {
            (None, collector) => {
                *collector = None;
                ControlFlow::Break(())
            }
            (Some(_), None) => ControlFlow::Break(()),
            (Some(item), Some(collector)) => collector.collect(item),
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = Option<T>>) -> ControlFlow<()> {
        match &mut self.collector {
            None => ControlFlow::Break(()),
            Some(collector) => {
                let mut any_none = false;
                let cf = collector.collect_many(items.into_iter().map_while(|item| {
                    any_none |= item.is_none();
                    item
                }));

                if any_none {
                    self.collector = None;
                    ControlFlow::Break(())
                } else {
                    cf
                }
            }
        }
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = Option<T>>) -> Self::Output {
        let mut collector = self.collector?;

        let mut any_none = false;
        let _ = collector.collect_many(items.into_iter().map_while(|item| {
            any_none |= item.is_none();
            item
        }));

        (!any_none).then(|| collector.finish())
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::option::of as prop_opt;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    // Precondition:
    // - `Vec::IntoCollector`
    proptest! {
        #[test]
        fn all_collect_methods(
            nums in propvec(prop_opt(any::<i32>()), ..=5),
            take_count in ..=5_usize,
        ) {
            all_collect_methods_impl(nums, take_count)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<Option<i32>>, take_count: usize) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || vec![].into_collector().take(take_count).trying_options(),
            should_break_pred: |mut iter| iter.len() >= take_count || iter.any(|num| num.is_none()),
            pred: |mut iter, output, remaining| {
                if iter.by_ref().take(take_count).collect::<Option<Vec<_>>>() != output {
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
