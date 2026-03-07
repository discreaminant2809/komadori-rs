use std::{cmp::Ordering, ops::ControlFlow};

use super::{MinBy, MinByKey, min_assign};

use crate::{
    collector::{Collector, CollectorBase, assert_collector},
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
        #[allow(deprecated)]
        assert_collector::<_, T>(MinBy::new(f))
    }

    /// Creates a new instance of [`MinByKey`] with a given key-extraction function.
    #[inline]
    pub const fn by_key<K, F>(f: F) -> MinByKey<T, K, F>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        #[allow(deprecated)]
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
}

impl<T: Ord> Collector<T> for Min<T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self.min {
            None => self.min = Some(item),
            Some(ref mut min) => min_assign(min, item),
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
    use std::cmp::Ordering;

    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::cmp::Min;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::super::test_utils::Id;

    proptest! {
        #[test]
        fn all_collect_methods_min(
            nums in propvec(any::<i32>(), ..5),
        ) {
            all_collect_methods_min_impl(nums)?;
        }
    }

    fn all_collect_methods_min_impl(nums: Vec<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
            collector_factory: || Min::new(),
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if !Id::full_eq_opt(iter.min(), output) {
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

    proptest! {
        #[test]
        fn all_collect_methods_min_by(
            nums in propvec(any::<i32>(), ..5),
        ) {
            all_collect_methods_min_by_impl(nums)?;
        }
    }

    fn all_collect_methods_min_by_impl(nums: Vec<i32>) -> TestCaseResult {
        fn comparator(Id { num: a, .. }: &Id, Id { num: b, .. }: &Id) -> Ordering {
            let (a, b) = (a.wrapping_add(i32::MAX), b.wrapping_add(i32::MAX));
            a.cmp(&b)
        }

        BasicCollectorTester {
            iter_factory: || nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
            collector_factory: || Min::by(comparator),
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if !Id::full_eq_opt(iter.min_by(comparator), output) {
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

    proptest! {
        #[test]
        fn all_collect_methods_min_by_key(
            nums in propvec(any::<i32>(), ..5),
        ) {
            all_collect_methods_min_by_key_impl(nums)?;
        }
    }

    fn all_collect_methods_min_by_key_impl(nums: Vec<i32>) -> TestCaseResult {
        fn key_extractor(Id { num, .. }: &Id) -> i32 {
            num.wrapping_add(i32::MAX)
        }

        BasicCollectorTester {
            iter_factory: || nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
            collector_factory: || Min::by_key(key_extractor),
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if !Id::full_eq_opt(iter.min_by_key(key_extractor), output) {
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
