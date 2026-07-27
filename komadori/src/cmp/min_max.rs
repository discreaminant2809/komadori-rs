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

    use crate::test_utils::prelude::*;

    use super::super::test_utils::Id;

    use super::*;

    collector_test!(collector {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {},
        iter: nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
        collector: MinMax::new(),
        expected_f: |iter, _| (iter.minmax(), false),
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });
}
