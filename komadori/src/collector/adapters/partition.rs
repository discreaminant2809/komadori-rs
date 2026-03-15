use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, Fuse};

/// A collector that distributes items between two collectors based on a predicate.
///
/// This `struct` is created by [`CollectorBase::partition()`]. See its documentation for more.
#[derive(Clone)]
pub struct Partition<CT, CF, F> {
    // `Fuse` is neccessary since we need to assess one's finishing state while assessing another,
    // like in `collect`.
    collector_if_true: Fuse<CT>,
    collector_if_false: Fuse<CF>,
    pred: F,
}

impl<CT, CF, F> Partition<CT, CF, F>
where
    CT: CollectorBase,
    CF: CollectorBase,
{
    pub(in crate::collector) fn new(
        collector_if_true: CT,
        collector_if_false: CF,
        pred: F,
    ) -> Self {
        Self {
            collector_if_true: Fuse::new(collector_if_true),
            collector_if_false: Fuse::new(collector_if_false),
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

impl<CT, CF, F> CollectorBase for Partition<CT, CF, F>
where
    CT: CollectorBase,
    CF: CollectorBase,
{
    type Output = (CT::Output, CF::Output);

    fn finish(self) -> Self::Output {
        (
            self.collector_if_true.finish(),
            self.collector_if_false.finish(),
        )
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        cf_and!(
            self.collector_if_true.break_hint(),
            self.collector_if_false.break_hint()
        )
    }
}

impl<CT, CF, F, T> Collector<T> for Partition<CT, CF, F>
where
    CT: Collector<T>,
    CF: Collector<T>,
    F: FnMut(&mut T) -> bool,
{
    fn collect(&mut self, mut item: T) -> ControlFlow<()> {
        if (self.pred)(&mut item) {
            cf_and!(
                self.collector_if_true.collect(item),
                self.collector_if_false.break_hint()
            )
        } else {
            cf_and!(
                self.collector_if_false.collect(item),
                self.collector_if_true.break_hint()
            )
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // Avoid consuming one item prematurely.
        self.break_hint()?;

        let mut items = items.into_iter();

        match items.try_for_each(|mut item| {
            if (self.pred)(&mut item) {
                self.collector_if_true.collect(item).map_break(|_| true)
            } else {
                self.collector_if_false.collect(item).map_break(|_| false)
            }
        }) {
            ControlFlow::Break(true) => {
                cf_and!(
                    self.collector_if_false
                        // Can't use `Iterator::filter` since it expects `&T`, not `&mut T` like us.
                        // `Iterator::filter_map` is lowkey great workaround in this case.
                        .collect_many(
                            items.filter_map(|mut item| (!(self.pred)(&mut item)).then_some(item)),
                        ),
                    self.collector_if_true.break_hint()
                )
            }
            ControlFlow::Break(false) => {
                cf_and!(
                    self.collector_if_true.collect_many(
                        items.filter_map(|mut item| (self.pred)(&mut item).then_some(item)),
                    ),
                    self.collector_if_false.break_hint()
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

        match items.try_for_each(|mut item| {
            #[allow(clippy::collapsible_else_if)] // we want it to be mirrored.
            if (self.pred)(&mut item) {
                if self.collector_if_true.collect(item).is_break() {
                    ControlFlow::Break(true)
                } else {
                    ControlFlow::Continue(())
                }
            } else {
                if self.collector_if_false.collect(item).is_break() {
                    ControlFlow::Break(false)
                } else {
                    ControlFlow::Continue(())
                }
            }
        }) {
            ControlFlow::Break(true) => (
                self.collector_if_true.finish(),
                self.collector_if_false.collect_then_finish(
                    items.filter_map(|mut item| (!(self.pred)(&mut item)).then_some(item)),
                ),
            ),
            ControlFlow::Break(false) => (
                self.collector_if_true.collect_then_finish(
                    items.filter_map(|mut item| (self.pred)(&mut item).then_some(item)),
                ),
                self.collector_if_false.finish(),
            ),
            ControlFlow::Continue(_) => self.finish(),
        }
    }
}

impl<CT: Debug, CF: Debug, F> Debug for Partition<CT, CF, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Partition")
            .field("collector_if_true", &self.collector_if_true)
            .field("collector_if_false", &self.collector_if_false)
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
            nums in propvec(any::<i32>(), ..=5),
            pos_count in ..=5_usize,
            non_pos_count in ..=5_usize,
        ) {
            all_collect_methods_impl(nums, pos_count, non_pos_count)?;
        }
    }

    fn all_collect_methods_impl(
        nums: Vec<i32>,
        pos_count: usize,
        non_pos_count: usize,
    ) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                vec![].into_collector().take(pos_count).partition(
                    |&mut num| num > 0,
                    vec![].into_collector().take(non_pos_count),
                )
            },
            should_break_pred: |iter| {
                iter.clone().filter(|&num| num > 0).count() >= pos_count
                    && iter.filter(|&num| num <= 0).count() >= non_pos_count
            },
            pred: |mut iter, output, remaining| {
                let (mut pos_nums, mut non_pos_nums) = (output.0.into_iter(), output.1.into_iter());
                let (mut pos_count, mut non_pos_count) = (pos_count, non_pos_count);

                while (pos_count > 0 || non_pos_count > 0)
                    && let Some(num) = iter.next()
                {
                    if pos_count > 0 && num > 0 {
                        pos_count -= 1;
                        if pos_nums.next() != Some(num) {
                            return Err(PredError::IncorrectOutput);
                        }
                    }

                    if non_pos_count > 0 && num <= 0 {
                        non_pos_count -= 1;
                        if non_pos_nums.next() != Some(num) {
                            return Err(PredError::IncorrectOutput);
                        }
                    }
                }

                if pos_nums.len() > 0 || non_pos_nums.len() > 0 {
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
