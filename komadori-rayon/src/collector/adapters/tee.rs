use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

use super::{__adapter_tee_internal, Fuse, TeeBase, Teer};

///
#[derive(Debug, Clone)]
pub struct Tee<C1, C2> {
    base: TeeBase<C1, C2, CopyTeer>,
}

#[derive(Clone)]
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct CopyTeer;

impl<C1, C2> Tee<C1, C2>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            base: TeeBase::new(collector1, collector2, CopyTeer),
        }
    }
}

impl<'this, C1, C2> DefineConsumer<'this> for Tee<C1, C2>
where
    C1: DefineConsumer<'this>,
    C2: DefineConsumer<'this>,
{
    type Consumer = __adapter_tee_internal::Consumer<
        <Fuse<C1> as DefineConsumer<'this>>::Consumer,
        <Fuse<C2> as DefineConsumer<'this>>::Consumer,
        CopyTeer,
    >;
}

impl<C1, C2> ParallelCollectorBase for Tee<C1, C2>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
{
    type Output = (C1::Output, C2::Output);

    #[inline]
    fn finish(self) -> Self::Output {
        self.base.finish()
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.base.break_hint()
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
        self.base.parts(len)
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
        self.base.take_parts(len)
    }
}

impl<'this, C1, C2> DefineUnindexedConsumer<'this> for Tee<C1, C2>
where
    C1: DefineUnindexedConsumer<'this>,
    C2: DefineUnindexedConsumer<'this>,
{
    type UnindexedConsumer = __adapter_tee_internal::Consumer<
        <Fuse<C1> as DefineUnindexedConsumer<'this>>::UnindexedConsumer,
        <Fuse<C2> as DefineUnindexedConsumer<'this>>::UnindexedConsumer,
        CopyTeer,
    >;
}

impl<C1, C2> UnindexedParallelCollectorBase for Tee<C1, C2>
where
    C1: UnindexedParallelCollectorBase,
    C2: UnindexedParallelCollectorBase,
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
        self.base.parts_unindexed()
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
        self.base.take_parts_unindexed()
    }
}

impl<T> Teer<T> for CopyTeer
where
    T: Copy,
{
    const ITEM_IS_COPY: bool = true;

    type PassDown<'a>
        = T
    where
        T: 'a;

    #[inline]
    fn pass_down(&mut self, item: &mut T) -> T {
        *item
    }

    #[inline]
    fn no_tee_collect(&mut self, collector: &mut impl Collector<T>, item: T) -> ControlFlow<()> {
        collector.collect(item)
    }

    #[inline]
    fn no_tee_collect_many(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: &mut impl Collector<T>,
    ) -> ControlFlow<()> {
        collector.collect_many(items)
    }

    #[inline]
    fn no_tee_collect_then_finish<O>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: komadori::collector::Fuse<impl Collector<T, Output = O>>,
    ) -> O {
        collector.collect_then_finish(items)
    }
}
