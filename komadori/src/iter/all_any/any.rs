use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector};

use super::raw_all_any::RawAllAny;

/// A collector that tests whether any collected item satisfies a predicate.
///
/// Its [`Output`] is initially `false` and remains `false` as long as every collected item
/// does not satisfy the predicate.
/// When the collector collects an item that makes the predicate `true`,
/// it returns [`Break(())`], and the [`Output`] becomes `true`.
///
/// This collector corresponds to [`Iterator::any()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::Any};
///
/// let mut collector = Any::new(|x| x < 0);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert!(!collector.finish());
/// ```
///
/// ```
/// use komadori::{prelude::*, iter::Any};
///
/// let mut collector = Any::new(|x| x < 0);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
///
/// // First matched item.
/// assert!(collector.collect(-1).is_break());
///
/// assert!(collector.finish());
/// ```
///
/// [`Break(())`]: std::ops::ControlFlow::Break
/// [`Output`]: CollectorBase::Output
#[derive(Clone)]
pub struct Any<F> {
    inner: RawAllAny<F, false>,
}

impl<F> Any<F> {
    /// Creates a new instance of this collector with the default output of `false`.
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

impl<F> CollectorBase for Any<F> {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.inner.get()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.inner.stopped() { 0 } else { request }
    }
}

impl<T, F> Collector<T> for Any<F>
where
    F: FnMut(T) -> bool,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.inner.collect_impl(|pred| pred(item))
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.inner.collect_impl(|pred| items.into_iter().any(pred))
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.inner
            .collect_then_finish_impl(|pred| items.into_iter().any(pred))
    }
}

impl<F> Debug for Any<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.debug_impl(f.debug_struct("Any"))
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
        collector_f: |_: &_| Any::new(|num| num >= 0),
        output_f: |mut iter, _| (&mut iter).any(|num| num >= 0),
        model_f: |_| BasicCollectorModel {
            state: false,
            advance_f: |any: &mut _, num| if num >= 0 {
                *any = true;
            },
            max_afford_f: |&any, request| if any { 0 } else { request },
            cf_f: |&any| if any {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            },
            output_and_pred_f: |any| (any, bool::eq)
        },
    });
}
