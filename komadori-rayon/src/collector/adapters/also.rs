use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

use super::Fuse;

/// A parallel collector that overrides the behavior in the indexed
/// or the unindexed path.
///
/// This `struct` is created by [`ParallelCollectorBase::also_unindexed()`],
/// and [`UnindexedParallelCollectorBase::also_indexed()`].
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

impl<'this, I, U> DefineConsumer<'this> for Also<I, U>
where
    I: DefineConsumer<'this>,
{
    type Consumer = <Fuse<I> as DefineConsumer<'this>>::Consumer;
}

impl<'this, I, U> DefineUnindexedConsumer<'this> for Also<I, U>
where
    U: DefineUnindexedConsumer<'this>,
{
    type UnindexedConsumer = <Fuse<U> as DefineUnindexedConsumer<'this>>::UnindexedConsumer;
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
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        self.indexed.parts(len)
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
        self.indexed.take_parts(len)
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
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        self.unindexed.parts_unindexed()
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
        self.unindexed.take_parts_unindexed()
    }
}
