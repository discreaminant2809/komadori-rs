use crate::collector::{Collector, CollectorBase};

use std::ops::ControlFlow;

/// A collector that [`clone`](Clone::clone)s every collected item.
///
/// This `struct` is created by [`CollectorBase::cloning()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Cloning<C>(C);

impl<C> Cloning<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self(collector)
    }
}

impl<C> CollectorBase for Cloning<C>
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

impl<'a, C, T> Collector<&'a T> for Cloning<C>
where
    C: Collector<T>,
    T: Clone,
{
    #[inline]
    fn collect(&mut self, item: &'a T) -> ControlFlow<()> {
        self.0.collect(item.clone())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'a T>) -> ControlFlow<()> {
        self.0.collect_many(items.into_iter().cloned())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = &'a T>) -> Self::Output {
        self.0.collect_then_finish(items.into_iter().cloned())
    }
}

impl<'a, C, T> Collector<&'a mut T> for Cloning<C>
where
    C: Collector<T>,
    T: Clone,
{
    #[inline]
    fn collect(&mut self, item: &'a mut T) -> ControlFlow<()> {
        self.0.collect(item.clone())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'a mut T>) -> ControlFlow<()> {
        self.0
            .collect_many(items.into_iter().map(|item| &*item).cloned())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = &'a mut T>) -> Self::Output {
        self.0
            .collect_then_finish(items.into_iter().map(|item| &*item).cloned())
    }
}
