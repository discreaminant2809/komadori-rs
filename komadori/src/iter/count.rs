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
    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(collector {
        iter_data: {
            let mut n = ..=10_usize;
        },
        other_data: {},
        iter: std::iter::repeat_n((), n),
        collector: Count::new(),
        expected_f: |iter| (iter.count(), false),
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });
}
