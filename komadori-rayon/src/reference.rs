use std::ops::ControlFlow;

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
/// Note that even in [`take_parts()`](ParallelCollectorBase::take_parts)
/// and [`take_unindexed_parts()`](UnindexedParallelCollectorBase::take_unindexed_parts)
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
/// // We should fuse. We can't know whether the parallel collector
/// // stops when using via `feed_into()`.
/// let mut collector = ParCount::new().fuse();
///
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
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify(C::parts(self, len))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        // Explicitly override it to strengthen the invariant in the doc,
        // and to shield from the change in the default implementation.
        let (consumer, commit) = C::parts(self, len);
        unique::take_uniquify((consumer, |output| {
            let _ = commit(output);
        }))
    }
}

/// See [this implementation for more](ParallelCollectorBase#impl-ParallelCollectorBase-for-%26mut+C).
impl<C> UnindexedParallelCollectorBase for &mut C
where
    C: UnindexedParallelCollectorBase + ?Sized,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify(C::unindexed_parts(self))
    }

    fn take_unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        // Explicitly override it to strengthen the invariant in the doc,
        // and to shield from the change in the default implementation.
        let (consumer, commit) = C::unindexed_parts(self);
        unique_unindexed::take_uniquify((consumer, |output| {
            let _ = commit(output);
        }))
    }
}
