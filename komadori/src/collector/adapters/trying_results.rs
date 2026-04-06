use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that sets the [`Output`] to [`Err(e)`](Err) when
/// an [`Err(e)`](Err) item is encountered for the first time,
/// else the underlying collector collects the `item` inside
/// [`Ok(item)`](Ok).
///
/// This `struct` is created by [`CollectorBase::trying_results()`].
/// See its documentation for more.
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Clone)]
pub struct TryingResults<C, E> {
    collector: Result<C, E>,
}

impl<C, E> TryingResults<C, E> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self {
            collector: Ok(collector),
        }
    }
}

impl<C, E> CollectorBase for TryingResults<C, E>
where
    C: CollectorBase,
{
    type Output = Result<C::Output, E>;

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

impl<C, T, E> Collector<Result<T, E>> for TryingResults<C, E>
where
    C: Collector<T>,
{
    fn collect(&mut self, item: Result<T, E>) -> ControlFlow<()> {
        match (item, &mut self.collector) {
            (Err(e), collector) => {
                *collector = Err(e);
                ControlFlow::Break(())
            }
            (Ok(_), Err(_)) => ControlFlow::Break(()),
            (Ok(item), Ok(collector)) => collector.collect(item),
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = Result<T, E>>) -> ControlFlow<()> {
        match &mut self.collector {
            Err(_) => ControlFlow::Break(()),
            Ok(collector) => {
                let mut error = None;
                let cf = collector.collect_many(items.into_iter().map_while(|item| match item {
                    Ok(item) => Some(item),
                    Err(e) => {
                        error = Some(e);
                        None
                    }
                }));

                if let Some(error) = error {
                    self.collector = Err(error);
                    ControlFlow::Break(())
                } else {
                    cf
                }
            }
        }
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = Result<T, E>>) -> Self::Output {
        let mut collector = self.collector?;

        let mut error = None;
        // We wouldn't like to forward to `collect_then_finish()`, since
        // the "finish" part may be expensive and wasted if an Err item is encountered.
        let _ = collector.collect_many(items.into_iter().map_while(|item| match item {
            Ok(item) => Some(item),
            Err(e) => {
                error = Some(e);
                None
            }
        }));

        match error {
            None => Ok(collector.finish()),
            Some(e) => Err(e),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::result::maybe_ok;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    // Precondition:
    // - `Vec::IntoCollector`
    proptest! {
        #[test]
        fn all_collect_methods(
            nums in propvec(maybe_ok(any::<i32>(), any::<i32>()), ..=5),
            take_count in ..=5_usize,
        ) {
            all_collect_methods_impl(nums, take_count)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<Result<i32, i32>>, take_count: usize) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || vec![].into_collector().take(take_count).trying_results(),
            should_break_pred: |mut iter| iter.len() >= take_count || iter.any(|num| num.is_err()),
            pred: |mut iter, output, remaining| {
                if iter
                    .by_ref()
                    .take(take_count)
                    .collect::<Result<Vec<_>, i32>>()
                    != output
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
