use std::{fmt::Debug, ops::ControlFlow};

use crate::{
    collector::assert_collector,
    collector::{Collector, CollectorBase},
};

/// A collector that outputs the n-th item (0-based) satisfying a predicate.
///
/// If no matching item has been found, its [`Output`] is `None`.
/// When the collector encounters an item that makes the predicate return `true`,
/// it returns [`Break`], and the [`Output`] becomes `Some` containing
/// the n-th matching item.
///
/// This collector corresponds to [`Iterator::position()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::Position};
///
/// let mut collector = Position::new(|s| s == "stop");
///
/// assert!(collector.collect("noble").is_continue());
/// assert!(collector.collect("singer").is_continue());
///
/// // Found!
/// assert!(collector.collect("stop").is_break());
///
/// assert_eq!(collector.finish(), Some(2));
/// ```
///
/// ```
/// use komadori::{prelude::*, iter::Position};
///
/// let mut collector = Position::new(|s| s == "stop");
///
/// assert!(collector.collect("a").is_continue());
/// assert!(collector.collect("b").is_continue());
/// assert!(collector.collect("c").is_continue());
///
/// assert_eq!(collector.finish(), None);
/// ```
///
/// [`Break`]: std::ops::ControlFlow::Break
/// [`Output`]: CollectorBase::Output
#[derive(Clone)]
pub struct Position<P> {
    idx: usize,
    pred: Option<P>,
}

impl<P> Position<P> {
    /// Creates a new instance of this collector with a predicate.
    #[inline]
    pub fn new<T>(pred: P) -> Self
    where
        P: FnMut(T) -> bool,
    {
        assert_collector::<_, T>(Self {
            idx: 0,
            pred: Some(pred),
        })
    }
}

impl<P> CollectorBase for Position<P> {
    type Output = Option<usize>;

    fn finish(self) -> Self::Output {
        self.pred.is_none().then_some(self.idx)
    }

    fn break_hint(&self) -> ControlFlow<()> {
        if self.pred.is_some() {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}

impl<P, T> Collector<T> for Position<P>
where
    P: FnMut(T) -> bool,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if self.pred.take_if(|pred| pred(item)).is_some() {
            ControlFlow::Break(())
        } else {
            self.idx += 1;
            ControlFlow::Continue(())
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        items.into_iter().try_for_each(|item| self.collect(item))
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let pred = self
            .pred
            .expect("`Position::collect_then_finish()` called after stopping accumulating");

        items.into_iter().position(pred).map(|pos| pos + self.idx)
    }
}

impl<P> Debug for Position<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Position")
            .field("idx", &self.idx)
            .field("pred", &std::any::type_name::<P>())
            .finish()
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
            // We mustn't make the collector stops accumulating
            starting_nums in propvec(..=0, ..=2),
        ) {
            all_collect_methods_impl(nums, starting_nums)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>, starting_nums: Vec<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                let mut collector = Position::new(|num| num > 0);
                assert!(
                    collector
                        .collect_many(starting_nums.iter().copied())
                        .is_continue()
                );
                collector
            },
            should_break_pred: |mut iter| iter.any(|num| num > 0),
            pred: |mut iter, output, remaining| {
                if starting_nums
                    .iter()
                    .copied()
                    .chain(&mut iter)
                    .position(|num| num > 0)
                    != output
                {
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
