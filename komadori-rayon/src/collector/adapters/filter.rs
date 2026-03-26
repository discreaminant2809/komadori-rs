use std::{fmt::Debug, ops::ControlFlow};

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

/// A parallel collector that uses a closure to determine whether
/// an item should be accumulated.
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::filter()`].
/// See its documentation for more.
#[derive(Clone)]
pub struct Filter<C, P> {
    collector: C,
    pred: P,
}

impl<C, P> Filter<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self { collector, pred }
    }
}

impl<C, P> Debug for Filter<C, P>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filter")
            .field("collector", &self.collector)
            .field("pred", &std::any::type_name::<P>())
            .finish()
    }
}

impl<'this, C, P> DefineConsumer<'this> for Filter<C, P>
where
    C: DefineUnindexedConsumer<'this>,
    P: Sync,
{
    type Consumer = <Self as DefineUnindexedConsumer<'this>>::UnindexedConsumer;
}

impl<C, P> ParallelCollectorBase for Filter<C, P>
where
    C: UnindexedParallelCollectorBase,
    P: Sync,
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
        let (consumer, commit) = self.parts_unindexed();
        (len, consumer, commit)
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
        let (consumer, commit) = self.take_parts_unindexed();
        (len, consumer, commit)
    }
}

impl<'this, C, P> DefineUnindexedConsumer<'this> for Filter<C, P>
where
    C: DefineUnindexedConsumer<'this>,
    P: Sync,
{
    type UnindexedConsumer = __adapter_filter_internal::Consumer<C::UnindexedConsumer, &'this P>;
}

impl<C, P> UnindexedParallelCollectorBase for Filter<C, P>
where
    C: UnindexedParallelCollectorBase,
    P: Sync,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        (
            __adapter_filter_internal::Consumer::new(consumer, &self.pred),
            commit,
        )
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        (
            __adapter_filter_internal::Consumer::new(consumer, &self.pred),
            commit,
        )
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub mod __adapter_filter_internal {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing;

    pub struct Consumer<C, P> {
        consumer: C,
        pred: P,
    }

    pub struct Combiner<C> {
        combiner: C,
    }

    // Can't utilize from komadori's filter(), since it requires item type right away.
    pub struct IntoCollector<C, P> {
        collector: C,
        pred: P,
    }

    impl<C, P> Consumer<C, P> {
        #[inline]
        pub(super) fn new(consumer: C, pred: P) -> Self {
            Self { consumer, pred }
        }
    }

    impl<C, P> IntoCollectorBase for Consumer<C, P>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = IntoCollector<C::IntoCollector, P>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            IntoCollector {
                collector: self.consumer.into_collector(),
                pred: self.pred,
            }
        }
    }

    impl<C, P> plumbing::ConsumerBase for Consumer<C, P>
    where
        C: plumbing::ConsumerBase,
        P: Clone + Send,
    {
        type Combiner = Combiner<C::Combiner>;

        #[inline]
        fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
            let (consumer, combiner) = self.consumer.split_off_left_at(index);
            (
                Self {
                    consumer,
                    pred: self.pred.clone(),
                },
                Combiner { combiner },
            )
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.consumer.break_hint()
        }
    }

    impl<C, P> CollectorBase for IntoCollector<C, P>
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
            self.collector.break_hint()
        }
    }

    impl<C, P> plumbing::UnindexedConsumerBase for Consumer<C, P>
    where
        C: plumbing::UnindexedConsumerBase,
        P: Clone + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                pred: self.pred.clone(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner {
                combiner: self.consumer.to_combiner(),
            }
        }
    }

    impl<C, O> plumbing::Combiner<O> for Combiner<C>
    where
        C: plumbing::Combiner<O>,
    {
        #[inline]
        fn combine(self, left: &mut O, right: O) {
            self.combiner.combine(left, right);
        }
    }

    impl<C, P, T> Collector<T> for IntoCollector<C, P>
    where
        C: Collector<T>,
        P: FnMut(&T) -> bool,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            if (self.pred)(&item) {
                self.collector.collect(item)
            } else {
                self.collector.break_hint()
            }
        }

        // Removed the overriden implementations cuz the items here are being consumed
        // without consulting the underlying collector's break hint during filtering.
        // Yes, the performance degrades, but it's because of `try_for_each()` and/or
        // LLVM noise (which could be fixed soon),
        // and in multiple reduction it still works well and performs similarly to fold().
    }
}
