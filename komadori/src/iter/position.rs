use std::{fmt::Debug, ops::ControlFlow};

use crate::{
    collector::assert_collector,
    collector::{Collector, CollectorBase},
};

/// A collector that outputs the n-th item (0-based) satisfying a predicate.
///
/// If no matching item has been found, its [`Output`] is `None`.
/// When the collector encounters an item that makes the predicate return `true`,
/// it returns [`Break(())`], and the [`Output`] becomes `Some` containing
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
/// [`Break(())`]: std::ops::ControlFlow::Break
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

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.pred.is_none() { 0 } else { request }
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
    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(collector {
        iter_data: TriIterI32Data::strategy(),
        collector_data: any::<()>(),
        iter_f: TriIterI32Factory,
        collector_f: |_: &_| Position::new(find_pred),
        output_f: |mut iter, _| (&mut iter).position(find_pred),
        model_f: |_| BasicCollectorModel {
            state: ModelState::default(),
            advance_f: |state: &mut ModelState, num| {
                if state.found {
                } else if find_pred(num) {
                    state.found = true;
                } else {
                    state.idx += 1;
                }
            },
            max_afford_f: |state, request| if state.found { 0 } else { request },
            cf_f: |state| if state.found {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            },
            output_and_pred_f: |state: ModelState| (
                state.found.then_some(state.idx),
                PartialEq::eq
            )
        },
    });

    fn find_pred(num: i32) -> bool {
        num >= 0
    }

    #[derive(Default)]
    struct ModelState {
        idx: usize,
        found: bool,
    }
}
