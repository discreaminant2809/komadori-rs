use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector};

use super::raw_all_any::RawAllAny;

/// A collector that tests whether all collected items satisfy a predicate.
///
/// Its [`Output`] is initially `true` and remains `true` as long as every collected item
/// satisfies the predicate.
/// When the collector collects an item that makes the predicate `false`,
/// it returns [`Break(())`], and the [`Output`] becomes `false`.
///
/// This collector corresponds to [`Iterator::all()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::All};
///
/// let mut collector = All::new(|x| x > 0);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert!(collector.finish());
/// ```
///
/// ```
/// use komadori::{prelude::*, iter::All};
///
/// let mut collector = All::new(|x| x > 0);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
///
/// // First mismatched item.
/// assert!(collector.collect(-1).is_break());
///
/// assert!(!collector.finish());
/// ```
///
/// [`Break(())`]: std::ops::ControlFlow::Break
/// [`Output`]: CollectorBase::Output
#[derive(Clone)]
pub struct All<F> {
    inner: RawAllAny<F, true>,
}

impl<F> All<F> {
    /// Creates a new instance of this collector with the default output of `true`.
    #[inline]
    pub const fn new<T>(pred: F) -> Self
    where
        F: FnMut(T) -> bool,
    {
        assert_collector::<_, T>(Self {
            inner: RawAllAny::new(pred),
        })
    }

    /// Returns the current result of the accumulation.
    #[inline]
    pub const fn get(&self) -> bool {
        self.inner.get()
    }
}

impl<F> CollectorBase for All<F> {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.get()
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.inner.break_hint()
    }
}

impl<T, F> Collector<T> for All<F>
where
    F: FnMut(T) -> bool,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.inner.collect_impl(|pred| pred(item))
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.inner.collect_impl(|pred| items.into_iter().all(pred))
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.inner
            .collect_then_finish_impl(|pred| items.into_iter().all(pred))
    }
}

impl<F> Debug for All<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.debug_impl(f.debug_struct("All"))
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::*;

    proptest! {
        /// [`All`](super::All)
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=5),
        ) {
            all_collect_methods_impl(nums)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || All::new(|num| num > 0),
            should_break_pred: |mut iter| iter.any(|num| num <= 0),
            pred: |mut iter, output, remaining| {
                if iter.all(|num| num > 0) != output {
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
}
