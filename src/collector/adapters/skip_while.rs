use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// A collector that skips the first collected items that satisfy a predicate
/// before accumulating.
///
/// This `struct` is created by [`CollectorBase::skip_while()`]. See its documentation for more.
#[derive(Clone)]
pub struct SkipWhile<C, P> {
    collector: C,
    pred: Option<P>,
}

impl<C, P> SkipWhile<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self {
            collector,
            pred: Some(pred),
        }
    }
}

impl<C, P> CollectorBase for SkipWhile<C, P>
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

impl<C, P, T> Collector<T> for SkipWhile<C, P>
where
    C: Collector<T>,
    P: FnMut(&T) -> bool,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if self.pred.as_mut().is_some_and(|pred| pred(&item)) {
            self.collector.break_hint()
        } else {
            self.pred.take();
            self.collector.collect(item)
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        let Some(pred) = &mut self.pred else {
            return self.collector.collect_many(items);
        };

        // Edge case:
        self.collector.break_hint()?;

        let mut items = items.into_iter();
        match items.by_ref().try_for_each({
            let collector = &mut self.collector;
            move |item| {
                let skipping = pred(&item);
                collector.break_hint().map_break(|_| None)?;
                if skipping {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(Some(item))
                }
            }
        }) {
            // We've already checked for the break hint in the previous iteration.
            // We may not need to check anymore
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
            ControlFlow::Break(None) => ControlFlow::Break(()),
            ControlFlow::Break(Some(first)) => {
                self.pred.take();
                self.collector.collect(first)?;
                self.collector.collect_many(items)
            }
        }
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let Some(mut pred) = self.pred else {
            return self.collector.collect_then_finish(items);
        };

        // Edge case:
        if self.collector.break_hint().is_break() {
            return self.collector.finish();
        }

        let mut items = items.into_iter();
        match items.by_ref().try_for_each({
            let collector = &mut self.collector;
            move |item| {
                let skipping = pred(&item);
                collector.break_hint().map_break(|_| None)?;
                if skipping {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(Some(item))
                }
            }
        }) {
            ControlFlow::Continue(_) | ControlFlow::Break(None) => self.collector.finish(),
            ControlFlow::Break(Some(first)) => {
                if self.collector.collect(first).is_break() {
                    self.collector.finish()
                } else {
                    self.collector.collect_then_finish(items)
                }
            }
        }
    }
}

impl<C, P> Debug for SkipWhile<C, P>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkipWhile")
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

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};
    use crate::{mem::Dropping, prelude::*};

    // We need to use `take()` to simulate the break case when enough items are skipped.
    // Precondition:
    // - `Vec::IntoCollector`
    // - `Collector::take()`
    // - `Dropping`
    proptest! {
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=3),
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
                    .skip_while(skip_while_pred)
            },
            should_break_pred: |iter| {
                Dropping
                    .take(take_count)
                    .collect_many(iter.skip_while(skip_while_pred))
                    .is_break()
            },
            pred: |mut iter, output, remaining| {
                if output
                    != iter
                        .by_ref()
                        .skip_while(skip_while_pred)
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

    fn skip_while_pred(&num: &i32) -> bool {
        num < 0
    }
}
