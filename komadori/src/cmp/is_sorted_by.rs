use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector};

use super::{IsSorted, IsSortedBase, IsSortedStore};

/// A collector that determines whether items are collected in sorted order
/// using the given comparator function.
///
/// The [`Output`] remains `true` as long as each collected item
/// is greater than or equal to the previously collected item.
/// When [`collect()`] or similar methods encounter an item
/// that is less than the previously collected item,
/// they return [`Break(())`], and the [`Output`] becomes `false`.
///
/// If no items or only one item were collected, the [`Output`] is `true`.
///
/// This collector is constructed by [`IsSorted::by()`].
///
/// This collector corresponds to [`Iterator::is_sorted_by()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::IsSorted};
///
/// // Strict ordering!
/// let mut collector = IsSorted::by(|a, b| a < b);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert!(collector.finish());
/// ```
///
/// ```
/// use komadori::{prelude::*, cmp::IsSorted};
///
/// let mut collector = IsSorted::by(|a, b| a < b);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
///
/// // Not strictly sorted!
/// assert!(collector.collect(2).is_break());
///
/// assert!(!collector.finish());
/// ```
///
/// [`collect()`]: Collector::collect
/// [`Output`]: CollectorBase::Output
/// [`Break(())`]: ControlFlow::Break
#[derive(Clone)]
pub struct IsSortedBy<T, F> {
    base: IsSortedBase<T, Store<F>>,
}

#[derive(Clone)]
struct Store<F> {
    compare: F,
}

impl<T> IsSorted<T> {
    /// Creates a new instance of [`IsSortedBy`] with a given comparator function.
    pub fn by<F>(compare: F) -> IsSortedBy<T, F>
    where
        F: FnMut(&T, &T) -> bool,
    {
        assert_collector::<_, T>(IsSortedBy {
            base: IsSortedBase::new(Store { compare }),
        })
    }
}

impl<T, F> CollectorBase for IsSortedBy<T, F> {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.base.finish()
    }
}

impl<T, F> Collector<T> for IsSortedBy<T, F>
where
    F: FnMut(&T, &T) -> bool,
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

impl<F, T> Debug for IsSortedBy<T, F>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IsSortedBy")
            .field(
                "state",
                &self.base.debug_state(|ds, _| {
                    ds.field("compare", &std::any::type_name::<F>());
                }),
            )
            .finish()
    }
}

impl<F, T> IsSortedStore<T, T> for Store<F>
where
    F: FnMut(&T, &T) -> bool,
{
    #[inline]
    fn map(&mut self, item: T) -> T {
        item
    }

    #[inline]
    fn store(&mut self, prev: &mut T, item: T) -> bool {
        if (self.compare)(prev, &item) {
            *prev = item;
            true
        } else {
            false
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::super::IsSorted;
    use super::*;

    proptest! {
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=3),
            starting_num in any::<Option<i32>>(),
        ) {
            all_collect_methods_impl(nums, starting_num)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>, starting_num: Option<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                let mut collector = IsSorted::by(is_strictly_sorted);
                assert!(collector.collect_many(starting_num).is_continue());
                collector
            },
            should_break_pred: |_| {
                !starting_num
                    .into_iter()
                    .chain(nums.iter().copied())
                    .is_sorted_by(is_strictly_sorted)
            },
            pred: |mut iter, output, remaining| {
                if starting_num
                    .into_iter()
                    .chain(&mut iter)
                    .is_sorted_by(is_strictly_sorted)
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

    fn is_strictly_sorted(a: &i32, b: &i32) -> bool {
        a < b
    }
}
