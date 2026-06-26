use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

impl<'a, C> DefineSerial<'a> for &mut C
where
    C: DefineSerial<'a> + ?Sized,
{
    type Serial = unique::Serial<'a, Self, C::Serial>;
}

impl<'a, C> DefineUnindexedSerial<'a> for &mut C
where
    C: DefineUnindexedSerial<'a> + ?Sized,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, C::UnindexedSerial>;
}

/// A mutable reference to a parallel collector is also a parallel collector.
///
/// Note that even in the [`take_parts()`](ParallelCollectorBase::take_parts)
/// and [`take_parts_unindexed()`](UnindexedParallelCollectorBase::take_parts_unindexed)
/// methods, the underlying parallel collectors will **not** be "taken,"
/// and can still be used afterwards.
///
/// However, it is difficult to know whether the parallel collector
/// has stopped collecting or not in this usage.
/// Use [`fuse()`](ParallelCollectorBase::fuse) whenever possible.
///
/// # Examples
///
/// ```
/// use komadori_rayon::{prelude::*, iter::ParCount};
/// use rayon::prelude::*;
///
/// let mut collector = ParCount::new();
/// [1, 2, 3]
///     .into_par_iter()
///     .feed_into(&mut collector);
///
/// // You can still use the parallel collector!
/// let count = (0..100)
///     .into_par_iter()
///     .feed_into(collector);
///
/// assert_eq!(count, 103);
/// ```
impl<C> ParallelCollectorBase for &mut C
where
    C: ParallelCollectorBase + ?Sized,
{
    type Output = ();

    fn finish(self) -> Self::Output {}

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
        unique::uniquify(C::parts(self, len))
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
        // Explicitly override it to strengthen the invariant in the doc,
        // and to shield from the change in the default implementation.
        let (len, consumer, commit) = C::parts(self, len);
        unique::take_uniquify((len, consumer, |output| {
            let _ = commit(output);
        }))
    }
}

impl<C> UnindexedParallelCollectorBase for &mut C
where
    C: UnindexedParallelCollectorBase + ?Sized,
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
        unique_unindexed::uniquify(C::parts_unindexed(self))
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
        // Explicitly override it to strengthen the invariant in the doc,
        // and to shield from the change in the default implementation.
        let (consumer, commit) = C::parts_unindexed(self);
        unique_unindexed::take_uniquify((consumer, |output| {
            let _ = commit(output);
        }))
    }
}
