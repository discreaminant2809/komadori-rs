use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// A collector with a custom collection logic.
///
/// This `struct` is created by [`CollectorBase::unbatching()`]. See its documentation for more.
#[derive(Clone)]
pub struct Unbatching<C, F> {
    collector: C,
    f: F,
}

impl<C, F> Unbatching<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self { collector, f }
    }
}

impl<C, F> CollectorBase for Unbatching<C, F>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.collector.break_hint()
    }
}

impl<C, T, F> Collector<T> for Unbatching<C, F>
where
    C: CollectorBase,
    F: FnMut(&mut C, T) -> ControlFlow<()>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        (self.f)(&mut self.collector, item)
    }

    // Can't meaningfully override `collect_many` and `collect_then_finish`.
}

impl<C: Debug, F> Debug for Unbatching<C, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Unbatching")
            .field("collector", &self.collector)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}
