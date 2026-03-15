use std::{fmt::Debug, ops::ControlFlow};

use itertools::Either;

use crate::collector::{Collector, CollectorBase, Fuse};

/// A collector that distributes items between two collectors based on a predicate.
///
/// This `struct` is created by [`CollectorBase::partition_map()`]. See its documentation for more.
#[derive(Clone)]
pub struct PartitionMap<CL, CR, F> {
    // `Fuse` is neccessary since we need to assess one's finishing state while assessing another,
    // like in `collect`.
    collector_left: Fuse<CL>,
    collector_right: Fuse<CR>,
    pred: F,
}

impl<CL, CR, F> PartitionMap<CL, CR, F>
where
    CL: CollectorBase,
    CR: CollectorBase,
{
    pub(in crate::collector) fn new(collector_left: CL, collector_right: CR, pred: F) -> Self {
        Self {
            collector_left: collector_left.fuse(),
            collector_right: collector_right.fuse(),
            pred,
        }
    }
}

// Put in a macro instead of function so that the short-circuit nature of `&&` is pertained.
macro_rules! cf_and {
    ($cf:expr, $pred:expr) => {
        // Can't swap, since we have to collect regardless.
        if $cf.is_break() && $pred.is_break() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
}

impl<CL, CR, F> CollectorBase for PartitionMap<CL, CR, F>
where
    CL: CollectorBase,
    CR: CollectorBase,
{
    type Output = (CL::Output, CR::Output);

    fn finish(self) -> Self::Output {
        (self.collector_left.finish(), self.collector_right.finish())
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        cf_and!(
            self.collector_left.break_hint(),
            self.collector_right.break_hint()
        )
    }
}

impl<CL, CR, F, T, L, R> Collector<T> for PartitionMap<CL, CR, F>
where
    CL: Collector<L>,
    CR: Collector<R>,
    F: FnMut(T) -> Either<L, R>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match (self.pred)(item) {
            Either::Left(item) => cf_and!(
                self.collector_left.collect(item),
                self.collector_right.break_hint()
            ),
            Either::Right(item) => cf_and!(
                self.collector_right.collect(item),
                self.collector_left.break_hint()
            ),
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // Avoid consuming one item prematurely.
        self.break_hint()?;

        let mut items = items.into_iter();

        match items.try_for_each(|item| match (self.pred)(item) {
            Either::Left(item) => self
                .collector_left
                .collect(item)
                .map_break(|_| Either::Left(())),
            Either::Right(item) => self
                .collector_right
                .collect(item)
                .map_break(|_| Either::Right(())),
        }) {
            ControlFlow::Break(Either::Left(())) => {
                cf_and!(
                    self.collector_right
                        .collect_many(items.filter_map(|item| (self.pred)(item).right())),
                    self.collector_left.break_hint()
                )
            }
            ControlFlow::Break(Either::Right(())) => {
                cf_and!(
                    self.collector_left
                        .collect_many(items.filter_map(|item| (self.pred)(item).left())),
                    self.collector_right.break_hint()
                )
            }
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
        }
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        // Avoid consuming one item prematurely.
        if self.break_hint().is_break() {
            return self.finish();
        }

        let mut items = items.into_iter();

        match items.try_for_each(|item| match (self.pred)(item) {
            Either::Left(item) => self
                .collector_left
                .collect(item)
                .map_break(|_| Either::Left(())),
            Either::Right(item) => self
                .collector_right
                .collect(item)
                .map_break(|_| Either::Right(())),
        }) {
            ControlFlow::Break(Either::Left(())) => (
                self.collector_left.finish(),
                self.collector_right
                    .collect_then_finish(items.filter_map(|item| (self.pred)(item).right())),
            ),
            ControlFlow::Break(Either::Right(())) => (
                self.collector_left
                    .collect_then_finish(items.filter_map(|item| (self.pred)(item).left())),
                self.collector_right.finish(),
            ),
            ControlFlow::Continue(_) => self.finish(),
        }
    }
}

impl<CL, CR, F> Debug for PartitionMap<CL, CR, F>
where
    CL: Debug,
    CR: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartitionMap")
            .field("collector_left", &self.collector_left)
            .field("collector_right", &self.collector_right)
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

    proptest! {
        /// Precondition:
        /// - [`crate::collector::Collector::take()`]
        /// - [`crate::vec::IntoCollector`]
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<Result<i32, i32>>(), ..=5),
            ok_count in ..=5_usize,
            err_count in ..=5_usize,
        ) {
            all_collect_methods_impl(nums, ok_count, err_count)?;
        }
    }

    fn all_collect_methods_impl(
        nums: Vec<Result<i32, i32>>,
        ok_count: usize,
        err_count: usize,
    ) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                vec![]
                    .into_collector()
                    .take(err_count)
                    .partition_map(From::from, vec![].into_collector().take(ok_count))
            },
            should_break_pred: |iter| {
                iter.clone().filter_map(Result::ok).count() >= ok_count
                    && iter.filter_map(Result::err).count() >= err_count
            },
            pred: |mut iter, output, remaining| {
                let (mut errs, mut oks) = (output.0.into_iter(), output.1.into_iter());
                let (mut ok_count, mut err_count) = (ok_count, err_count);

                while (ok_count > 0 || err_count > 0)
                    && let Some(res) = iter.next()
                {
                    match res {
                        Ok(num) if ok_count > 0 => {
                            ok_count -= 1;
                            if oks.next() != Some(num) {
                                return Err(PredError::IncorrectOutput);
                            }
                        }
                        Err(num) if err_count > 0 => {
                            err_count -= 1;
                            if errs.next() != Some(num) {
                                return Err(PredError::IncorrectOutput);
                            }
                        }
                        _ => {}
                    }
                }

                if errs.len() > 0 || oks.len() > 0 {
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
