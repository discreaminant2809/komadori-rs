use std::{
    fmt::Debug,
    ops::ControlFlow,
    sync::atomic::{AtomicBool, Ordering},
};

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

/// A parallel collector that accumulates items until it encounters
/// an items that makess a given predicate `false` at *any* time.
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::take_any_while()`].
/// See its documentation for more.
pub struct TakeAnyWhile<C, P> {
    collector: C,
    take_pred: TakePred<P>,
}

struct TakePred<P> {
    pred: P,
    stopped: AtomicBool,
}

impl<C, P> TakeAnyWhile<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self {
            collector,
            take_pred: TakePred {
                pred,
                stopped: AtomicBool::new(false),
            },
        }
    }
}

impl<C, P> Debug for TakeAnyWhile<C, P>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TakeAnyWhile")
            .field("collector", &self.collector)
            .field("pred", &std::any::type_name::<P>())
            .field("stopped", &self.take_pred.stopped.load(Ordering::Relaxed))
            .finish()
    }
}

// `AtomicBool` doesn't implement `Clone`, so we can't derive.
impl<C, P> Clone for TakeAnyWhile<C, P>
where
    C: Clone,
    P: Clone,
{
    fn clone(&self) -> Self {
        Self {
            collector: self.collector.clone(),
            take_pred: TakePred {
                pred: self.take_pred.pred.clone(),
                stopped: AtomicBool::new(self.take_pred.stopped.load(Ordering::Relaxed)),
            },
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.collector.clone_from(&source.collector);
        self.take_pred.pred.clone_from(&source.take_pred.pred);
        self.take_pred.stopped.store(
            source.take_pred.stopped.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
    }
}

impl<P> TakePred<P> {
    fn should_take<T>(&self, item: &T) -> bool
    where
        P: Fn(&T) -> bool,
    {
        if self.stopped.load(Ordering::Relaxed) {
            false
        } else if (self.pred)(item) {
            true
        } else {
            self.stopped.store(true, Ordering::Relaxed);
            false
        }
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        if self.stopped.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

impl<'this, C, P> DefineConsumer<'this> for TakeAnyWhile<C, P>
where
    C: DefineUnindexedConsumer<'this>,
    P: Sync,
{
    type Consumer = <Self as DefineUnindexedConsumer<'this>>::UnindexedConsumer;
}

impl<C, P> ParallelCollectorBase for TakeAnyWhile<C, P>
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
        self.take_pred.break_hint()?;
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

impl<'this, C, P> DefineUnindexedConsumer<'this> for TakeAnyWhile<C, P>
where
    C: DefineUnindexedConsumer<'this>,
    P: Sync,
{
    type UnindexedConsumer = consumer::Consumer<'this, C::UnindexedConsumer, P>;
}

impl<C, P> UnindexedParallelCollectorBase for TakeAnyWhile<C, P>
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
        (consumer::Consumer::new(consumer, &self.take_pred), commit)
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
        (consumer::Consumer::new(consumer, &self.take_pred), commit)
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing::{ConsumerBase, UnindexedConsumerBase};

    use super::TakePred;

    pub struct Consumer<'a, C, P> {
        consumer: C,
        take_pred: &'a TakePred<P>,
    }

    pub struct IntoCollector<'a, C, P> {
        collector: C,
        take_pred: &'a TakePred<P>,
    }

    impl<'a, C, P> Consumer<'a, C, P> {
        #[inline]
        pub(super) fn new(consumer: C, take_pred: &'a TakePred<P>) -> Self {
            Self {
                consumer,
                take_pred,
            }
        }
    }

    impl<'a, C, P> IntoCollectorBase for Consumer<'a, C, P>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = IntoCollector<'a, C::IntoCollector, P>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            IntoCollector {
                collector: self.consumer.into_collector(),
                take_pred: self.take_pred,
            }
        }
    }

    impl<C, P> ConsumerBase for Consumer<'_, C, P>
    where
        C: UnindexedConsumerBase,
        P: Sync,
    {
        type Combiner = C::Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.take_pred.break_hint()?;
            self.consumer.break_hint()
        }
    }

    impl<C, P> UnindexedConsumerBase for Consumer<'_, C, P>
    where
        C: UnindexedConsumerBase,
        P: Sync,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                take_pred: self.take_pred,
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
    }

    impl<'a, C, P> CollectorBase for IntoCollector<'a, C, P>
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
            self.take_pred.break_hint()?;
            self.collector.break_hint()
        }
    }

    impl<C, P, T> Collector<T> for IntoCollector<'_, C, P>
    where
        C: Collector<T>,
        P: Fn(&T) -> bool,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            if self.take_pred.should_take(&item) {
                self.collector.collect(item)
            } else {
                self.collector.break_hint()
            }
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            let cf = self.collector.collect_many(
                items
                    .into_iter()
                    .take_while(|item| self.take_pred.should_take(item)),
            );

            self.take_pred.break_hint()?;
            cf
        }

        #[inline]
        fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
            self.collector.collect_then_finish(
                items
                    .into_iter()
                    .take_while(|item| self.take_pred.should_take(item)),
            )
        }
    }
}
