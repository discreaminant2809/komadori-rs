use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector_base};

/// A collector that counts the number of items it collects.
///
/// This collector corresponds to [`Iterator::count()`].
///
/// # Overflow Behavior
///
/// This collector does no guarding against overflows, so feeding it
/// more than [`usize::MAX`] items either produces the wrong result or panics.
/// If overflow checks are enabled, a panic is guaranteed.
/// This is similar to [`Iterator::count()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::Count};
///
/// let mut collector = Count::new();
///
/// assert!(collector.collect(3).is_continue());
/// assert!(collector.collect(7).is_continue());
/// assert!(collector.collect(0).is_continue());
/// assert!(collector.collect(-1).is_continue());
///
/// assert_eq!(collector.finish(), 4);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Count {
    count: usize,
}

impl Count {
    /// Creates a new instance of this collector with an initial count of 0.
    #[inline]
    pub const fn new() -> Self {
        assert_collector_base(Count { count: 0 })
    }

    #[inline]
    fn increment(&mut self) {
        // We don't care about overflow.
        // See: https://doc.rust-lang.org/1.90.0/src/core/iter/traits/iterator.rs.html#219-230
        self.count += 1;
    }
}

impl CollectorBase for Count {
    type Output = usize;

    #[inline]
    fn finish(self) -> usize {
        self.count
    }
}

impl<T> Collector<T> for Count {
    #[inline]
    fn collect(&mut self, _: T) -> ControlFlow<()> {
        self.increment();
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.count += items.into_iter().count();
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.count + items.into_iter().count()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::*;

    proptest! {
        #[test]
        fn all_collect_methods(
            count in ..=9_usize,
            starting_count in ..=9_usize,
        ) {
            all_collect_methods_impl(count, starting_count)?;
        }
    }

    fn all_collect_methods_impl(count: usize, starting_count: usize) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || std::iter::repeat_n((), count),
            collector_factory: || {
                let mut collector = Count::new();
                collector.count = starting_count;
                collector
            },
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if starting_count + iter.count() != output {
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
