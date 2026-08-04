use core::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector, finish_boxed_impl};

use super::{IsSorted, IsSortedBase, IsSortedStore};

/// A collector that determines whether items are collected in sorted order
/// using the given key extraction function.
///
/// The [`Output`] remains `true` as long as the key of each collected item
/// is greater than or equal to the the key of the previously collected item.
/// When [`collect()`] or similar methods encounter an item whose key
/// is less than the key of the previously collected item,
/// they return [`Break(())`], and the [`Output`] becomes `false`.
///
/// If no items or only one item were collected, the [`Output`] is `true`.
///
/// This collector is constructed by [`IsSorted::by_key()`].
///
/// This collector corresponds to [`Iterator::is_sorted_by_key()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::IsSorted};
///
/// let mut collector = IsSorted::by_key(i32::abs);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(-2).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert!(collector.finish());
/// ```
///
/// ```
/// use komadori::{prelude::*, cmp::IsSorted};
///
/// let mut collector = IsSorted::by_key(i32::abs);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(-3).is_continue());
///
/// // |2| < |-3|
/// assert!(collector.collect(2).is_break());
///
/// assert!(!collector.finish());
/// ```
///
/// [`collect()`]: Collector::collect
/// [`Output`]: CollectorBase::Output
/// [`Break(())`]: ControlFlow::Break
#[derive(Clone)]
pub struct IsSortedByKey<K, F> {
    base: IsSortedBase<K, Store<F>>,
}

#[derive(Clone)]
struct Store<F> {
    f: F,
}

impl<T> IsSorted<T> {
    /// Creates a new instance of [`IsSortedByKey`] with a given key extraction function.
    pub fn by_key<K, F>(f: F) -> IsSortedByKey<K, F>
    where
        F: FnMut(T) -> K,
        K: PartialOrd,
    {
        assert_collector::<_, T>(IsSortedByKey {
            base: IsSortedBase::new(Store { f }),
        })
    }
}

impl<K, F> CollectorBase for IsSortedByKey<K, F> {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.base.finish()
    }

    finish_boxed_impl! {}
}

impl<T, F, K> Collector<T> for IsSortedByKey<K, F>
where
    F: FnMut(T) -> K,
    K: PartialOrd,
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

impl<F, K> Debug for IsSortedByKey<K, F>
where
    K: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IsSortedByKey")
            .field(
                "state",
                &self.base.debug_state(|ds, _| {
                    ds.field("f", &core::any::type_name::<F>());
                }),
            )
            .finish()
    }
}

impl<F, T, K> IsSortedStore<T, K> for Store<F>
where
    F: FnMut(T) -> K,
    K: PartialOrd,
{
    #[inline]
    fn map(&mut self, item: T) -> K {
        (self.f)(item)
    }

    #[inline]
    fn store(&mut self, prev: &mut K, item: T) -> bool {
        let item_key = self.map(item);
        if item_key < *prev {
            false
        } else {
            *prev = item_key;
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
        collector: IsSorted::by_key(key_extractor),
        expected_f: |iter, _| {
            let res = iter.is_sorted_by_key(key_extractor);
            (res, !res)
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    fn key_extractor(a: i32) -> i32 {
        a.wrapping_add(i32::MAX)
    }
}
