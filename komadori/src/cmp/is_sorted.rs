use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector};

use super::{IsSortedBase, IsSortedStore};

/// A collector that determines whether items are collected in sorted order.
///
/// The [`Output`] remains `true` as long as each collected item
/// is greater than or equal to the previously collected item.
/// When [`collect()`] or similar methods encounter an item
/// that is less than the previously collected item,
/// they return [`Break(())`], and the [`Output`] becomes `false`.
///
/// If no items or only one item were collected, the [`Output`] is `true`.
///
/// This collector corresponds to [`Iterator::is_sorted()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::IsSorted};
///
/// let mut collector = IsSorted::new();
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
/// let mut collector = IsSorted::new();
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// // Not sorted!
/// assert!(collector.collect(2).is_break());
///
/// assert!(!collector.finish());
/// ```
///
/// [`collect()`]: Collector::collect
/// [`Output`]: CollectorBase::Output
/// [`Break(())`]: ControlFlow::Break
#[derive(Clone)]
pub struct IsSorted<T> {
    base: IsSortedBase<T, Store>,
}

#[derive(Clone)]
struct Store;

impl<T> IsSorted<T>
where
    T: PartialOrd,
{
    /// Creates a new instance of this collector.
    #[inline]
    pub fn new() -> Self {
        assert_collector::<_, T>(Self {
            base: IsSortedBase::new(Store),
        })
    }
}

impl<T> CollectorBase for IsSorted<T> {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.base.finish()
    }
}

impl<T> Collector<T> for IsSorted<T>
where
    T: PartialOrd,
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

impl<T: PartialOrd> Default for IsSorted<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Debug for IsSorted<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IsSorted")
            .field("state", &self.base.debug_state(|_, _| {}))
            .finish()
    }
}

impl<T> IsSortedStore<T, T> for Store
where
    T: PartialOrd,
{
    #[inline]
    fn map(&mut self, item: T) -> T {
        item
    }

    #[inline]
    fn store(&mut self, prev: &mut T, item: T) -> bool {
        if item < *prev {
            false
        } else {
            *prev = item;
            true
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(collector {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {},
        iter: nums.iter().copied(),
        collector: IsSorted::new(),
        expected_f: |iter, _| {
            let res = iter.is_sorted();
            (res, !res)
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });
}
