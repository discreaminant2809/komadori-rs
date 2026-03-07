use std::{cmp::Ordering, ops::ControlFlow};

use super::{MaxBy, MaxByKey, max_assign};

use crate::{
    collector::{Collector, CollectorBase, assert_collector},
    iter::Fold,
};

/// A collector that computes the maximum value among the items it collects.
///
/// Its [`Output`](CollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the maximum item otherwise.
///
/// This collector corresponds to [`Iterator::max()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::Max};
///
/// let mut collector = Max::new();
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(3).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(5).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert_eq!(collector.finish(), Some(5));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use komadori::{prelude::*, cmp::Max};
///
/// assert_eq!(Max::<i32>::new().finish(), None);
/// ```
#[derive(Debug, Clone)]
pub struct Max<T> {
    // For `Debug` impl used by `MaxByKey`.
    pub(super) max: Option<T>,
}

impl<T> Max<T> {
    /// Creates a new instance of this collector.
    #[inline]
    pub const fn new() -> Self
    where
        T: Ord,
    {
        assert_collector(Self { max: None })
    }

    /// Creates a new instance of [`MaxBy`] with a given comparison function.
    #[inline]
    pub const fn by<F>(f: F) -> MaxBy<T, F>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        #[allow(deprecated)]
        assert_collector(MaxBy::new(f))
    }

    /// Creates a new instance of [`MaxByKey`] with a given key-extraction function.
    #[inline]
    pub const fn by_key<K, F>(f: F) -> MaxByKey<T, K, F>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        #[allow(deprecated)]
        assert_collector(MaxByKey::new(f))
    }
}

impl<T: Ord> Default for Max<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CollectorBase for Max<T> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.max
    }
}

impl<T: Ord> Collector<T> for Max<T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        // This one IS ~x27 slower (proven by benchmark)
        // self.max = self.max.take().max(Some(item));

        match self.max {
            None => self.max = Some(item),
            Some(ref mut max) => max_assign(max, item),
        }

        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match self.max {
            // If we haven't collected at all, we can avoid `chain()`'s overhead.
            // See the below also.
            None => self.max = items.into_iter().max(),
            Some(ref mut max) => {
                // We can't just `max.max(items.into_iter().max())`.
                // We have to preserve the order of which is compared to which.
                // This is basically `chain()`, which doesn't override `max()`!
                items.into_iter().for_each(move |item| {
                    max_assign(max, item);
                });
            }
        };

        ControlFlow::Continue(())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.max {
            None => items.into_iter().max(),
            // We don't use the std's `fold()` to account for large states.
            Some(max) => Some(Fold::new(max, max_assign).collect_then_finish(items)),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use std::cmp::Ordering;

    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::cmp::Max;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::super::test_utils::Id;

    proptest! {
        #[test]
        fn all_collect_methods_max(
            nums in propvec(any::<i32>(), ..5),
        ) {
            all_collect_methods_max_impl(nums)?;
        }
    }

    fn all_collect_methods_max_impl(nums: Vec<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
            collector_factory: || Max::new(),
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if !Id::full_eq_opt(iter.max(), output) {
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
        fn all_collect_methods_max_by(
            nums in propvec(any::<i32>(), ..5),
        ) {
            all_collect_methods_max_by_impl(nums)?;
        }
    }

    fn all_collect_methods_max_by_impl(nums: Vec<i32>) -> TestCaseResult {
        fn comparator(Id { num: a, .. }: &Id, Id { num: b, .. }: &Id) -> Ordering {
            let (a, b) = (a.wrapping_add(i32::MAX), b.wrapping_add(i32::MAX));
            a.cmp(&b)
        }

        BasicCollectorTester {
            iter_factory: || nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
            collector_factory: || Max::by(comparator),
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if !Id::full_eq_opt(iter.max_by(comparator), output) {
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
        fn all_collect_methods_max_by_key(
            nums in propvec(any::<i32>(), ..5),
        ) {
            all_collect_methods_max_by_key_impl(nums)?;
        }
    }

    fn all_collect_methods_max_by_key_impl(nums: Vec<i32>) -> TestCaseResult {
        fn key_extractor(Id { num, .. }: &Id) -> i32 {
            num.wrapping_add(i32::MAX)
        }

        BasicCollectorTester {
            iter_factory: || nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
            collector_factory: || Max::by_key(key_extractor),
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if !Id::full_eq_opt(iter.max_by_key(key_extractor), output) {
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
