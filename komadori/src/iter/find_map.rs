use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector};

use super::Find;

/// A collector that searches for the first item which makes a predicate returns [`Some`].
///
/// If no matching item has been found, its [`Output`] is [`None`].
/// When the collector encounters an item that makes the predicate return [`Some`],
/// it returns [`Break(())`], and the [`Output`] becomes [`Some`] containing the mapped item.
///
/// This collector is constructed by [`Find::map()`](super::Find::map).
///
/// This collector corresponds to [`Iterator::find_map()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::Find};
///
/// let mut collector = Find::map(|s: &str| s.parse().ok());
///
/// assert!(collector.collect("noble").is_continue());
/// assert!(collector.collect("singer").is_continue());
///
/// // Found!
/// assert!(collector.collect("1").is_break());
///
/// assert_eq!(collector.finish(), Some(1));
/// ```
///
/// ```
/// use komadori::{prelude::*, iter::Find};
///
/// let mut collector = Find::map(|s: &str| s.parse::<i32>().ok());
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
pub struct FindMap<P, R> {
    state: State<P, R>,
}

#[derive(Clone)]
enum State<P, R> {
    Searching(P),
    Found(R),
}

impl<P, R> Find<P, R> {
    /// Creates a new instance of [`FindMap`] with a predicate.
    pub fn map<T>(pred: P) -> FindMap<P, R>
    where
        P: FnMut(T) -> Option<R>,
    {
        assert_collector::<_, T>(FindMap {
            state: State::Searching(pred),
        })
    }
}

impl<P, R> CollectorBase for FindMap<P, R> {
    type Output = Option<R>;

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
    fn max_afford(&self, request: usize) -> usize {
        if matches!(self.state, State::Found(_)) {
            0
        } else {
            request
        }
    }
}

impl<P, T, R> Collector<T> for FindMap<P, R>
where
    P: FnMut(T) -> Option<R>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if let State::Searching(ref mut pred) = self.state {
            if let Some(res) = pred(item) {
                self.state = State::Found(res);
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
            if let Some(res) = items.into_iter().find_map(pred) {
                self.state = State::Found(res);
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
            State::Searching(pred) => items.into_iter().find_map(pred),
            State::Found(res) => Some(res),
        }
    }
}

impl<P, R> Debug for FindMap<P, R>
where
    R: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FindMap")
            .field("state", &self.state)
            .field("f", &std::any::type_name::<P>())
            .finish()
    }
}

impl<P, R> Debug for State<P, R>
where
    R: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Searching(_) => f.debug_struct("Searching").finish(),
            Self::Found(res) => f.debug_tuple("Found").field(res).finish(),
        }
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
        collector_f: |_: &_| Find::map(find_map_pred),
        output_f: |mut iter, _| (&mut iter).find_map(find_map_pred),
        model_f: |_| BasicCollectorModel {
            state: None::<i32>,
            advance_f: |found: &mut _, num| {
                let num = find_map_pred(num);
                if num.is_some() {
                    *found = num;
                }
            },
            max_afford_f: |found, request| if found.is_some() { 0 } else { request },
            cf_f: |found| if found.is_some() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            },
            output_and_pred_f: |found| (found, PartialEq::eq)
        },
    });

    fn find_map_pred(num: i32) -> Option<i32> {
        num.checked_add(i32::MAX)
    }
}
