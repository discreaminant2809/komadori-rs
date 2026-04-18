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
    break_hint: ControlFlow<()>,
}

impl<C> Fuse<C>
where
    C: ParallelCollectorBase,
{
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self {
            break_hint: collector.break_hint(),
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
    fn break_hint(&self) -> ControlFlow<()> {
        self.break_hint
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl crate::collector::plumbing::Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        let (actual_len, consumer, commit) = self.collector.parts(len);
        unique::uniquify((
            actual_len,
            consumer::Consumer::new(consumer, self.break_hint),
            |output| {
                let cf = commit(output);
                if cf.is_break() {
                    self.break_hint = cf;
                }
                self.break_hint
            },
        ))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl crate::collector::plumbing::Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        let (actual_len, consumer, commit) = self.collector.take_parts(len);
        unique::take_uniquify((
            actual_len,
            consumer::Consumer::new(consumer, self.break_hint),
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
        unique_unindexed::uniquify((consumer::Consumer::new(consumer, self.break_hint), |output| {
            let cf = commit(output);
            if cf.is_break() {
                self.break_hint = cf;
            }
            self.break_hint
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
        unique_unindexed::take_uniquify((consumer::Consumer::new(consumer, self.break_hint), commit))
    }
}

mod consumer {
    use std::{cell::Cell, ops::ControlFlow};

    use komadori::prelude::*;

    use crate::collector::plumbing;

    #[allow(missing_debug_implementations)]
    pub struct Consumer<C> {
        pub(super) consumer: C,
        pub(super) break_hint: Cell<ControlFlow<()>>,
    }

    // We have to roll out our own Fuse because we
    // cannot set the cached hint inside komadori's Fuse.
    #[allow(missing_debug_implementations)]
    pub struct Serial<C> {
        collector: C,
        break_hint: ControlFlow<()>,
    }

    impl<C> Consumer<C> {
        pub(super) fn new(consumer: C, break_hint: ControlFlow<()>) -> Self {
            Self {
                consumer,
                break_hint: break_hint.into(),
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
                break_hint: self.break_hint.get(),
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

            let break_hint = (|| {
                self.break_hint.get()?;
                // Don't forget to re-assess the break hint of self!
                self.break_hint.set(self.consumer.break_hint());
                consumer.break_hint()
            })();

            (
                Self {
                    break_hint: break_hint.into(),
                    consumer,
                },
                combiner,
            )
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            if self.break_hint.get().is_continue() {
                self.break_hint.set(self.consumer.break_hint());
            }

            self.break_hint.get()
        }
    }

    impl<C> plumbing::UnindexedConsumer for Consumer<C>
    where
        C: plumbing::UnindexedConsumer,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            let consumer = self.consumer.split_off_left();

            let break_hint = (|| {
                self.break_hint.get()?;
                // Don't forget to re-assess the break hint of self!
                self.break_hint.set(self.consumer.break_hint());
                consumer.break_hint()
            })();

            Self {
                break_hint: break_hint.into(),
                consumer,
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
    }

    impl<C> Serial<C> {
        #[inline]
        fn collect_impl(&mut self, f: impl FnOnce(&mut C) -> ControlFlow<()>) -> ControlFlow<()> {
            self.break_hint?;
            self.break_hint = f(&mut self.collector);
            self.break_hint
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

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.break_hint
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
            if self.break_hint.is_break() {
                self.finish()
            } else {
                self.collector.collect_then_finish(items)
            }
        }
    }
}
