use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial},
    },
    helpers::{unique, unique_unindexed},
};

use super::Fuse;

/// A parallel collector that overrides the behavior in the indexed
/// or the unindexed path.
///
/// This `struct` is created by [`ParallelCollectorBase::also_unindexed()`],
/// and [`ParallelCollectorBase::also_indexed()`].
/// See their documentations for more.
#[derive(Debug, Clone)]
pub struct Also<I, U> {
    indexed: Fuse<I>,
    unindexed: Fuse<U>,
}

impl<I, U> Also<I, U>
where
    I: ParallelCollectorBase,
    U: ParallelCollectorBase,
{
    pub(in crate::collector) fn new(indexed: I, unindexed: U) -> Self {
        Self {
            indexed: indexed.fuse(),
            unindexed: unindexed.fuse(),
        }
    }
}

impl<'this, I, U> DefineSerial<'this> for Also<I, U>
where
    I: DefineSerial<'this>,
{
    type Serial = unique::Serial<'this, Self, <Fuse<I> as DefineSerial<'this>>::Serial>;
}

impl<'this, I, U> DefineUnindexedSerial<'this> for Also<I, U>
where
    U: DefineUnindexedSerial<'this>,
{
    type UnindexedSerial =
        unique_unindexed::Serial<'this, Self, <Fuse<U> as DefineUnindexedSerial<'this>>::UnindexedSerial>;
}

impl<I, U> ParallelCollectorBase for Also<I, U>
where
    I: ParallelCollectorBase,
    U: ParallelCollectorBase,
{
    type Output = (I::Output, U::Output);

    #[inline]
    fn finish(self) -> Self::Output {
        (self.indexed.finish(), self.unindexed.finish())
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        if self.indexed.break_hint().is_break() && self.unindexed.break_hint().is_break() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    #[inline]
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
        unique::uniquify(self.indexed.parts(len))
    }

    #[inline]
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
        unique::take_uniquify(self.indexed.take_parts(len))
    }
}

impl<I, U> UnindexedParallelCollectorBase for Also<I, U>
where
    I: ParallelCollectorBase,
    U: UnindexedParallelCollectorBase,
{
    #[inline]
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl crate::collector::plumbing::UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify(self.unindexed.parts_unindexed())
    }

    #[inline]
    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl crate::collector::plumbing::UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        unique_unindexed::take_uniquify(self.unindexed.take_parts_unindexed())
    }
}
