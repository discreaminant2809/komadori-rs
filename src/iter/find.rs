use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector};

/// A collector that searches for the first item satisfying a predicate.
///
/// If no matching item has been found, its [`Output`] is `None`.
/// When the collector encounters an item that makes the predicate return `true`,
/// it returns [`Break(())`], and the [`Output`] becomes `Some` containing that item.
///
/// This collector corresponds to [`Iterator::find()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::Find};
///
/// let mut collector = Find::new(|&x| x % 3 == 0);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(5).is_continue());
///
/// // Found!
/// assert!(collector.collect(6).is_break());
///
/// assert_eq!(collector.finish(), Some(6));
/// ```
///
/// ```
/// use komadori::{prelude::*, iter::Find};
///
/// let mut collector = Find::new(|&x| x % 3 == 0);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(5).is_continue());
/// assert!(collector.collect(-2).is_continue());
///
/// assert_eq!(collector.finish(), None);
/// ```
///
/// [`Break(())`]: std::ops::ControlFlow::Break
/// [`Output`]: CollectorBase::Output
#[derive(Clone)]
pub struct Find<T, F> {
    state: State<T, F>,
}

#[derive(Clone)]
enum State<T, F> {
    Searching(F),
    Found(T),
}

impl<T, F> Find<T, F>
where
    F: FnMut(&T) -> bool,
{
    /// Creates an intance of this collector with a given predicate.
    #[inline]
    pub const fn new(pred: F) -> Self {
        assert_collector::<_, T>(Self {
            state: State::Searching(pred),
        })
    }
}

impl<T, F> CollectorBase for Find<T, F> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        if let State::Found(item) = self.state {
            Some(item)
        } else {
            None
        }
    }

    // Technically, we don't need to override it
    // since when this collector stops the method is useless anyway.
    // But we will have a support of `FUSED` const variable later,
    // making this neccessary.
    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        if matches!(self.state, State::Found(_)) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

impl<T, F> Collector<T> for Find<T, F>
where
    F: FnMut(&T) -> bool,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if let State::Searching(ref mut pred) = self.state {
            if pred(&item) {
                self.state = State::Found(item);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        } else {
            ControlFlow::Break(())
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        if let State::Searching(ref mut pred) = self.state {
            if let Some(item) = items.into_iter().find(pred) {
                self.state = State::Found(item);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        } else {
            ControlFlow::Break(())
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.state {
            State::Searching(pred) => items.into_iter().find(pred),
            State::Found(item) => Some(item),
        }
    }
}

impl<T: Debug, F> Debug for Find<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let item = if let State::Found(ref item) = self.state {
            Some(item)
        } else {
            None
        };

        f.debug_struct("Find").field("found", &item).finish()
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
            collector_factory: || Find::new(|&num| num > 0),
            should_break_pred: |mut iter| iter.any(|num| num > 0),
            pred: |mut iter, output, remaining| {
                if iter.find(|&num| num > 0) != output {
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
