use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that restricts to the unindexed path only.
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::unindexed_only()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct UnindexedOnly<C> {
    collector: C,
}

impl<C> UnindexedOnly<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self { collector }
    }
}

impl<'a, C> DefineSerial<'a> for UnindexedOnly<C>
where
    C: DefineUnindexedSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, C::UnindexedSerial>;
}

impl<'a, C> DefineUnindexedSerial<'a> for UnindexedOnly<C>
where
    C: DefineUnindexedSerial<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, C::UnindexedSerial>;
}

impl<C> ParallelCollectorBase for UnindexedOnly<C>
where
    C: UnindexedParallelCollectorBase,
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

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        unique::uniquify((len, consumer, commit))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique::take_uniquify((len, consumer, commit))
    }
}

impl<C> UnindexedParallelCollectorBase for UnindexedOnly<C>
where
    C: UnindexedParallelCollectorBase,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify(self.collector.parts_unindexed())
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        unique_unindexed::take_uniquify(self.collector.take_parts_unindexed())
    }
}
