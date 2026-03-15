use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector};

/// A collector that accumulates items using a function.
///
/// This collector corresponds to [`Iterator::fold()`], except that
/// the accumulated value is mutated in place.
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::Fold};
///
/// let mut collector = Fold::new(0, |sum, num| *sum += num);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert_eq!(collector.finish(), 6);
/// ```
#[derive(Clone)]
pub struct Fold<A, F> {
    accum: A,
    f: F,
}

impl<A, F> Fold<A, F> {
    /// Creates a new instance of this collector with an initial value and an accumulator.
    pub const fn new<T>(init: A, f: F) -> Self
    where
        F: FnMut(&mut A, T),
    {
        assert_collector::<_, T>(Self { accum: init, f })
    }
}

impl<A, F> CollectorBase for Fold<A, F> {
    type Output = A;

    #[inline]
    fn finish(self) -> Self::Output {
        self.accum
    }
}

impl<A, T, F> Collector<T> for Fold<A, F>
where
    F: FnMut(&mut A, T),
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        (self.f)(&mut self.accum, item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        items
            .into_iter()
            .for_each(|item| (self.f)(&mut self.accum, item));
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        items.into_iter().for_each({
            let accum = &mut self.accum;
            move |item| (self.f)(accum, item)
        });

        self.accum
    }
}

impl<A: Debug, F> Debug for Fold<A, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fold")
            .field("accum", &self.accum)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::*;

    proptest! {
        /// Here, we will use the Kadane's Algorithm to test fold.
        /// [`Fold`](super::Fold)
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=9),
        ) {
            all_collect_methods_impl(nums)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                Fold::new(KADANE_INIT, |(sum, max_sum), num| {
                    kadane_fold(sum, max_sum, num)
                })
            },
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                let expected = iter.fold(KADANE_INIT, |(mut sum, mut max_sum), num| {
                    kadane_fold(&mut sum, &mut max_sum, num);
                    (sum, max_sum)
                });

                // We also check the `sum`
                if expected != output {
                    Err(PredError::IncorrectOutput)
                } else if remaining.next().is_some() {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }

    fn kadane_fold(sum: &mut i32, max_sum: &mut Option<i32>, num: i32) {
        *sum = num;
        *max_sum = (*max_sum).max(Some(*sum));
        *sum = (*sum).max(0);
    }

    const KADANE_INIT: (i32, Option<i32>) = (0, None);
}
