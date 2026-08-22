use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that can "safely" collect even after
/// the underlying collector has stopped accumulating,
/// without triggering undesired behaviors.
///
/// This `struct` is created by [`ParallelCollectorBase::fuse()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct Fuse<C> {
    collector: C,
    stopped: bool,
}

impl<C> Fuse<C>
where
    C: ParallelCollectorBase,
{
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self {
            stopped: collector.max_afford(1) == 0,
            collector,
        }
    }
}

impl<'this, C> DefineSerial<'this> for Fuse<C>
where
    C: DefineSerial<'this>,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<<C as DefineSerial<'this>>::Serial>>;
}

impl<'this, C> DefineUnindexedSerial<'this> for Fuse<C>
where
    C: DefineUnindexedSerial<'this>,
{
    type UnindexedSerial = unique_unindexed::Serial<
        'this,
        Self,
        consumer::Serial<<C as DefineUnindexedSerial<'this>>::UnindexedSerial>,
    >;
}

impl<C> ParallelCollectorBase for Fuse<C>
where
    C: ParallelCollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.stopped {
            0
        } else {
            self.collector.max_afford(request)
        }
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl crate::collector::plumbing::Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts(len);
        unique::uniquify((consumer::Consumer::new(consumer, self.stopped), |output| {
            set_stopped_and_ret_bh(&mut self.stopped, commit(output))
        }))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl crate::collector::plumbing::Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        let (consumer, commit) = self.collector.take_parts(len);
        unique::take_uniquify((
            consumer::Consumer::new(consumer, self.stopped),
            // We can't set the flag if we cannot obtain the signal from
            // the committer.
            commit,
        ))
    }
}

impl<C> UnindexedParallelCollectorBase for Fuse<C>
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
        let (consumer, commit) = self.collector.parts_unindexed();
        unique_unindexed::uniquify((consumer::Consumer::new(consumer, self.stopped), |output| {
            set_stopped_and_ret_bh(&mut self.stopped, commit(output))
        }))
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
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique_unindexed::take_uniquify((consumer::Consumer::new(consumer, self.stopped), commit))
    }
}

fn set_stopped_and_ret_bh(stopped: &mut bool, cf: ControlFlow<()>) -> ControlFlow<()> {
    if *stopped {
        ControlFlow::Break(())
    } else {
        *stopped = cf.is_break();
        cf
    }
}

mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing;

    #[allow(missing_debug_implementations)]
    pub struct Consumer<C> {
        consumer: C,
        stopped: bool,
    }

    // We have to roll out our own Fuse because we
    // cannot set the cached hint inside komadori's Fuse.
    #[allow(missing_debug_implementations)]
    pub struct Serial<C> {
        collector: C,
        stopped: bool,
    }

    impl<C> Consumer<C>
    where
        C: plumbing::Consumer,
    {
        pub(super) fn new(consumer: C, stopped: bool) -> Self {
            Self {
                stopped: stopped || consumer.max_afford(1) == 0,
                consumer,
            }
        }
    }
    impl<C> IntoCollectorBase for Consumer<C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector: self.consumer.into_collector(),
                stopped: self.stopped,
            }
        }
    }

    impl<C> plumbing::Consumer for Consumer<C>
    where
        C: plumbing::Consumer,
    {
        type Combiner = C::Combiner;

        #[inline]
        fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
            let (consumer, combiner) = self.consumer.split_off_left_at(index);
            (Self::new(consumer, self.stopped), combiner)
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            if self.stopped {
                0
            } else {
                self.consumer.max_afford(request)
            }
        }
    }

    impl<C> plumbing::UnindexedConsumer for Consumer<C>
    where
        C: plumbing::UnindexedConsumer,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            let consumer = self.consumer.split_off_left();
            Self::new(consumer, self.stopped)
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
    }

    impl<C> Serial<C> {
        #[inline]
        fn collect_impl(&mut self, f: impl FnOnce(&mut C) -> ControlFlow<()>) -> ControlFlow<()> {
            if self.stopped {
                ControlFlow::Break(())
            } else if f(&mut self.collector).is_continue() {
                ControlFlow::Continue(())
            } else {
                self.stopped = true;
                ControlFlow::Break(())
            }
        }
    }

    impl<C> CollectorBase for Serial<C>
    where
        C: CollectorBase,
    {
        type Output = C::Output;

        #[inline]
        fn finish(self) -> Self::Output {
            self.collector.finish()
        }

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn reserve(&mut self, additional: usize) {
            if !self.stopped {
                self.collector.reserve(additional);
            }
        }

        #[inline]
        fn max_afford(&self, amount: usize) -> usize {
            if self.stopped {
                0
            } else {
                self.collector.max_afford(amount)
            }
        }
    }

    impl<C, T> Collector<T> for Serial<C>
    where
        C: Collector<T>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            self.collect_impl(|collector| collector.collect(item))
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            self.collect_impl(|collector| collector.collect_many(items))
        }

        #[inline]
        fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
            if self.stopped {
                self.finish()
            } else {
                self.collector.collect_then_finish(items)
            }
        }

        #[inline]
        unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
            self.collect_impl(|collector| unsafe {
                // SAFETY: We've reserved for at least one item.
                collector.assume_reserved_collect(item)
            })
        }
    }
}
