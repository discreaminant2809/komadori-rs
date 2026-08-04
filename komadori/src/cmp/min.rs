use core::{cmp::Ordering, ops::ControlFlow};

use super::{MinBy, MinByKey, min_assign};

use crate::{
    collector::{Collector, CollectorBase, assert_collector, finish_boxed_impl},
    iter::Fold,
};

/// A collector that computes the minimum value among the items it collects.
///
/// Its [`Output`](CollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the minimum item otherwise.
///
/// This collector corresponds to [`Iterator::min()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::Min};
///
/// let mut collector = Min::new();
///
/// assert!(collector.collect(5).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(3).is_continue());
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert_eq!(collector.finish(), Some(1));
/// ```
///
/// Its output is `None` if it has not encountered any items.
///
/// ```
/// use komadori::{prelude::*, cmp::Min};
///
/// assert_eq!(Min::<i32>::new().finish(), None);
/// ```
#[derive(Debug, Clone)]
pub struct Min<T> {
    // For `Debug` impl for `MinByKey`.
    pub(super) min: Option<T>,
}

impl<T> Min<T> {
    /// Creates a new instance of this collector.
    #[inline]
    pub const fn new() -> Self
    where
        T: Ord,
    {
        assert_collector::<_, T>(Self { min: None })
    }

    /// Creates a new instance of [`MinBy`] with a given comparison function.
    #[inline]
    pub const fn by<F>(f: F) -> MinBy<T, F>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        assert_collector::<_, T>(MinBy::new(f))
    }

    /// Creates a new instance of [`MinByKey`] with a given key-extraction function.
    #[inline]
    pub const fn by_key<K, F>(f: F) -> MinByKey<T, K, F>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        assert_collector::<_, T>(MinByKey::new(f))
    }
}

impl<T: Ord> Default for Min<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CollectorBase for Min<T> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.min
    }

    finish_boxed_impl! {}
}

impl<T: Ord> Collector<T> for Min<T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if const { core::mem::size_of::<T>() <= 64 } {
            // Somehow it yields a little bit codegen (e.g. vectorization)!
            // For now, min-vec benefits from it.
            // However, for very large item types, this may become a `memcpy` fest,
            // so we check the size first.
            self.min = Some(match self.min.take() {
                None => item,
                Some(min) => min.min(item),
            });
        } else {
            match &mut self.min {
                min @ None => *min = Some(item),
                Some(min) => min_assign(min, item),
            }
        }

        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match self.min {
            // If we haven't collected at all, we can avoid `chain()`'s overhead.
            // See the below also.
            None => self.min = items.into_iter().min(),
            Some(ref mut min) => {
                // We can't just `min.min(items.into_iter().min())`.
                // We have to preserve the order of which is compared to which.
                // This is basically `chain()`, which doesn't override `min()`!
                items.into_iter().for_each(move |item| {
                    min_assign(min, item);
                });
            }
        };

        ControlFlow::Continue(())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.min {
            None => items.into_iter().min(),
            // We don't use the std's `fold()` to account for large states.
            Some(min) => Some(Fold::new(min, min_assign).collect_then_finish(items)),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use super::*;
    use crate::test_utils::prelude::*;

    use super::super::test_utils::Id;

    collector_test!(collector_small {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {},
        iter: nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
        collector: Min::new(),
        expected_f: |iter, _| (iter.min(), false),
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    collector_test!(collector_big {
        iter_data: {
            let mut nums = propvec(any::<i32>().prop_map(Big::new), ..=5);
        },
        other_data: {},
        iter: nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
        collector: Min::new(),
        expected_f: |iter, _| (iter.min(), false),
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    const _: () = assert!(size_of::<i32>() <= 64);
    const _: () = assert!(size_of::<Big>() > 64);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct Big {
        num: i32,
        _padding: [i32; 16],
    }

    impl Big {
        fn new(num: i32) -> Self {
            Self {
                num,
                _padding: Default::default(),
            }
        }
    }
}
