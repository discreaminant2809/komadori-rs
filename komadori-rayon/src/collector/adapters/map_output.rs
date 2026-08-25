use std::{fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
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

impl<'this, C, F> DefineSerial<'this> for MapOutput<C, F>
where
    C: DefineSerial<'this>,
{
    type Serial = unique::Serial<'this, Self, C::Serial>;
}

impl<'this, C, F> DefineUnindexedSerial<'this> for MapOutput<C, F>
where
    C: DefineUnindexedSerial<'this>,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, C::UnindexedSerial>;
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
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request)
    }

    #[inline]
    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify(self.collector.parts(len))
    }

    #[inline]
    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        unique::take_uniquify(self.collector.take_parts(len))
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
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify(self.collector.parts_unindexed())
    }

    #[inline]
    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        unique_unindexed::take_uniquify(self.collector.take_parts_unindexed())
    }
}
