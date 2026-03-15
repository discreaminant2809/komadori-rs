use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// Creates a collector that alternates the behavior of
/// [`break_hint()`](CollectorBase::break_hint).
///
/// This `struct` is created by [`CollectorBase::alt_break_hint()`].
/// See its documentation for more.
#[derive(Clone)]
pub struct AltBreakHint<C, F> {
    collector: C,
    f: F,
}

impl<C, F> AltBreakHint<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self { collector, f }
    }
}

impl<C, F> CollectorBase for AltBreakHint<C, F>
where
    C: CollectorBase,
    F: Fn(&C) -> ControlFlow<()>,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        (self.f)(&self.collector)
    }
}

impl<C, T, F> Collector<T> for AltBreakHint<C, F>
where
    C: Collector<T>,
    F: Fn(&C) -> ControlFlow<()>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.collector.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collector.collect_many(items)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.collector.collect_then_finish(items)
    }
}

impl<C, F> Debug for AltBreakHint<C, F>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AltBreakHint")
            .field("collector", &self.collector)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}
