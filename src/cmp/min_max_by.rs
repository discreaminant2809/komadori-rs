use std::{cmp::Ordering, fmt::Debug, ops::ControlFlow};

use itertools::MinMaxResult;

use crate::collector::{Collector, CollectorBase};

use super::{MinMax, MinMaxBase};

/// A collector that computes the minimum and maximum values among the items it collects
/// according to a comparison function.
///
/// Its [`Output`](CollectorBase::Output) is:
///
/// - [`MinMaxResult::NoElements`] if no items were collected.
/// - [`MinMaxResult::OneElement`] containing one item if exactly that item was collected.
/// - [`MinMaxResult::MinMax`] containing the minimum and the maximum items (in order)
///   if two or more items were collected.
///
///   If there are multiple equally minimum items, the first one collected is returned.
///   If there are multiple equally maximum items, the last one collected is returned.
///
/// This collector is constructed by [`MinMax::by()`](MinMax::by).
///
/// This collector corresponds to [`Itertools::minmax_by()`](itertools::Itertools::minmax_by).
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::MinMax};
/// use itertools::MinMaxResult;
///
/// assert_eq!(
///     [].into_iter().feed_into(MinMax::by(f64::total_cmp)),
///     MinMaxResult::NoElements,
/// );
/// assert_eq!(
///     [1.1].into_iter().feed_into(MinMax::by(f64::total_cmp)),
///     MinMaxResult::OneElement(1.1),
/// );
/// assert_eq!(
///     [1.1, -2.2, 3E4].into_iter().feed_into(MinMax::by(f64::total_cmp)),
///     MinMaxResult::MinMax(-2.2, 3E4),
/// );
/// ```
#[derive(Clone)]
pub struct MinMaxBy<T, F> {
    base: MinMaxBase<T, F>,
}

impl<T> MinMax<T> {
    /// Creates a new instance of [`MinMaxBy`] with a given comparison function.
    pub const fn by<F>(f: F) -> MinMaxBy<T, F>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        MinMaxBy {
            base: MinMaxBase::new(f),
        }
    }
}

impl<T, F> CollectorBase for MinMaxBy<T, F>
where
    F: FnMut(&T, &T) -> Ordering,
{
    type Output = MinMaxResult<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.base.finish()
    }
}

impl<T, F> Collector<T> for MinMaxBy<T, F>
where
    F: FnMut(&T, &T) -> Ordering,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.base.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.base.collect_many(items)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.base.collect_then_finish(items)
    }
}

impl<T, F> Debug for MinMaxBy<T, F>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinMaxBy")
            .field("state", self.base.debug_state())
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use itertools::Itertools;

    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::super::test_utils::Id;
    use super::*;

    proptest! {
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=3),
            starting_nums in propvec(any::<i32>(), ..=3),
        ) {
            all_collect_methods_impl(nums, starting_nums)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>, starting_nums: Vec<i32>) -> TestCaseResult {
        fn comparator(Id { num: a, .. }: &Id, Id { num: b, .. }: &Id) -> Ordering {
            let (a, b) = (a.wrapping_add(i32::MAX), b.wrapping_add(i32::MAX));
            a.cmp(&b)
        }

        BasicCollectorTester {
            iter_factory: || nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
            collector_factory: || {
                let mut collector = MinMax::by(comparator);
                let _ = collector.collect_many(
                    starting_nums
                        .iter()
                        .zip(nums.len()..)
                        .map(|(&num, id)| Id { id, num }),
                );
                collector
            },
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                let iter = starting_nums
                    .iter()
                    .zip(nums.len()..)
                    .map(|(&num, id)| Id { id, num })
                    .chain(iter);

                if !Id::full_eq_minmax_res(iter.minmax_by(comparator), output) {
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
}
