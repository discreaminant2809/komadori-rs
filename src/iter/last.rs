use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, assert_collector};

/// A collector that stores the last item it collects.
///
/// If no items have been collected, its [`Output`] is `None`;
/// otherwise, it is `Some` containing the most recently collected item.
///
/// This collector corresponds to [`Iterator::last()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::Last};
///
/// let mut collector = Last::new();
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert_eq!(collector.finish(), Some(3));
/// ```
///
/// ```
/// use komadori::{prelude::*, iter::Last};
///
/// assert_eq!(Last::<i32>::new().finish(), None);
/// ```
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Clone)]
pub struct Last<T> {
    value: Option<T>,
}

impl<T> Last<T> {
    /// Creates an intance of this collector.
    #[inline]
    pub const fn new() -> Self {
        assert_collector::<_, T>(Last { value: None })
    }
}

impl<T> CollectorBase for Last<T> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.value
    }
}

impl<T> Collector<T> for Last<T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.value = Some(item);
        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // We need a bit complication here since we may risk assigning `None` to `self.value` being `Some`.
        match (&mut self.value, items.into_iter().last()) {
            (Some(value), Some(last)) => *value = last,
            // DO NOT update here. `items` don't have a value to "inherit" the last spot.
            (Some(_), None) => {}
            (None, last) => self.value = last,
        }

        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        items.into_iter().last().or(self.value)
    }
}

impl<T> Default for Last<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::option::of as prop_opt;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::*;

    proptest! {
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=9),
            starting_num in prop_opt(any::<i32>()),
        ) {
            all_collect_methods_impl(nums, starting_num)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>, starting_num: Option<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                let mut collector = Last::new();
                collector.value = starting_num;
                collector
            },
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if starting_num.into_iter().chain(iter).last() != output {
                    Err(PredError::IncorrectOutput)
                } else if remaining.ne([]) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
