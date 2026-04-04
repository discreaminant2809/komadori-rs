use std::{fmt::Debug, ops::ControlFlow};

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

/// A parallel collector that transforms the final accumulated result.
///
/// This `struct` is created by [`ParallelCollectorBase::map_output()`].
/// See its documentation for more.
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

impl<'this, C, F> DefineConsumer<'this> for MapOutput<C, F>
where
    C: DefineConsumer<'this>,
{
    type Consumer = C::Consumer;
}

impl<'this, C, F> DefineUnindexedConsumer<'this> for MapOutput<C, F>
where
    C: DefineUnindexedConsumer<'this>,
{
    type UnindexedConsumer = C::UnindexedConsumer;
}

impl<C, F, R> ParallelCollectorBase for MapOutput<C, F>
where
    C: ParallelCollectorBase,
    F: FnOnce(C::Output) -> R,
{
    type Output = R;

    #[inline]
    fn finish(self) -> Self::Output {
        (self.f)(self.collector.finish())
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.collector.break_hint()
    }

    #[inline]
    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        self.collector.parts(len)
    }

    #[inline]
    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(<<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output),
    ) {
        self.collector.take_parts(len)
    }
}

impl<C, F, R> UnindexedParallelCollectorBase for MapOutput<C, F>
where
    C: UnindexedParallelCollectorBase,
    F: FnOnce(C::Output) -> R,
{
    #[inline]
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        self.collector.parts_unindexed()
    }

    #[inline]
    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ),
    ) {
        self.collector.take_parts_unindexed()
    }
}
