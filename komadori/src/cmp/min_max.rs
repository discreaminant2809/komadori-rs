use std::{fmt::Debug, ops::ControlFlow};

use itertools::MinMaxResult;

use crate::collector::{Collector, CollectorBase};

use super::{MinMaxBase, OrdComparator};

/// A collector that computes the minimum and maximum values among the items it collects.
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
/// This collector corresponds to [`Itertools::minmax()`](itertools::Itertools::minmax).
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::MinMax};
/// use itertools::MinMaxResult;
///
/// assert_eq!(
///     [].into_iter().feed_into(MinMax::<i32>::new()),
///     MinMaxResult::NoElements,
/// );
/// assert_eq!(
///     [1].into_iter().feed_into(MinMax::new()),
///     MinMaxResult::OneElement(1),
/// );
/// assert_eq!(
///     [1, 3, 2].into_iter().feed_into(MinMax::new()),
///     MinMaxResult::MinMax(1, 3),
/// );
/// ```
#[derive(Clone)]
pub struct MinMax<T> {
    base: MinMaxBase<T, OrdComparator>,
}

impl<T> MinMax<T> {
    /// Creates a new instance of this collector.
    #[inline]
    pub const fn new() -> Self
    where
        T: Ord,
    {
        Self {
            base: MinMaxBase::new(OrdComparator),
        }
    }

    pub(super) fn debug_state(&self) -> &impl Debug
    where
        T: Debug,
    {
        self.base.debug_state()
    }
}

impl<T> CollectorBase for MinMax<T>
where
    T: Ord,
{
    type Output = MinMaxResult<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.base.finish()
    }
}

impl<T> Collector<T> for MinMax<T>
where
    T: Ord,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.base.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.base.collect_many(items)
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.base.collect_then_finish(items)
    }
}

impl<T> Default for MinMax<T>
where
    T: Ord,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Debug for MinMax<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinMax")
            .field("state", self.base.debug_state())
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
        BasicCollectorTester {
            iter_factory: || nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
            collector_factory: || {
                let mut collector = MinMax::new();
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

                if !Id::full_eq_minmax_res(iter.minmax(), output) {
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
