use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that feeds the underlying collector with
/// the mutable reference to the item, "pretending" the collector
/// accepts owned items.
///
/// This `struct` is created by [`CollectorBase::funnel()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct Funnel<C>(C);

impl<C> Funnel<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self(collector)
    }
}

impl<C> CollectorBase for Funnel<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0.finish()
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.0.break_hint()
    }
}

impl<C, T> Collector<T> for Funnel<C>
where
    C: for<'a> Collector<&'a mut T>,
{
    #[inline]
    fn collect(&mut self, mut item: T) -> ControlFlow<()> {
        self.0.collect(&mut item)
    }

    // Impossible to override `collect_many` and `collect_then_finish`
}
