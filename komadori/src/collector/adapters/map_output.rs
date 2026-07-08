use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// A collector that transforms the final accumulated result.
///
/// This `struct` is created by [`CollectorBase::map_output()`]. See its documentation for more.
#[derive(Clone)]
pub struct MapOutput<C, F> {
    collector: C,
    f: F,
}

impl<C, F> MapOutput<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self { collector, f }
    }
}

impl<C, T, F> CollectorBase for MapOutput<C, F>
where
    C: CollectorBase,
    F: FnOnce(C::Output) -> T,
{
    type Output = T;

    #[inline]
    fn finish(self) -> Self::Output {
        (self.f)(self.collector.finish())
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.collector.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request)
    }
}

impl<C, T, F, R> Collector<T> for MapOutput<C, F>
where
    C: Collector<T>,
    F: FnOnce(C::Output) -> R,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.collector.collect(item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        // SAFETY: The caller has reserved at least 1 item.
        unsafe { self.collector.assume_reserved_collect(item) }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collector.collect_many(items)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        (self.f)(self.collector.collect_then_finish(items))
    }
}

impl<C, F> Debug for MapOutput<C, F>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapOutput")
            .field("collector", &self.collector)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}
