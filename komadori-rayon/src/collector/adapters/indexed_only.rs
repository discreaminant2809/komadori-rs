use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase,
        plumbing::{Consumer, DefineSerial},
    },
    helpers::unique,
};

/// A parallel collector that restricts to the indexed path only.
///
/// This `struct` is created by [`ParallelCollectorBase::indexed_only()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct IndexedOnly<C> {
    collector: C,
}

impl<C> IndexedOnly<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self { collector }
    }
}

impl<'a, C> DefineSerial<'a> for IndexedOnly<C>
where
    C: DefineSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, C::Serial>;
}

impl<C> ParallelCollectorBase for IndexedOnly<C>
where
    C: ParallelCollectorBase,
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
        unique::uniquify(self.collector.parts(len))
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
        unique::take_uniquify(self.collector.take_parts(len))
    }
}

// Deliberately no implementation for UnindexedParallelCollectorBase
