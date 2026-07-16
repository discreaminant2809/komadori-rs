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
}

impl<F> CollectorBase for All<F> {
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
    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(collector {
        iter_data: TriIterI32Data::strategy(),
        collector_data: any::<()>(),
        iter_f: TriIterI32Factory,
        collector_f: |_: &_| All::new(|num| num >= 0),
        output_f: |mut iter, _| (&mut iter).all(|num| num >= 0),
        model_f: |_| BasicCollectorModel {
            state: true,
            advance_f: |all: &mut _, num| if num < 0 {
                *all = false;
            },
            max_afford_f: |&all, request| if all { request } else { 0 },
            cf_f: |&all| if all {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            },
            output_and_pred_f: |all| (all, bool::eq)
        },
    });
}
