use std::{
    fmt::Debug,
    ops::ControlFlow,
    sync::atomic::{AtomicUsize, Ordering},
};

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

/// A parallel collector that stops accumulating after collecting `n` items,
/// or fewer if the underlying collector stops sooner.
///
/// This `struct` is created by [`ParallelCollectorBase::take()`].
/// See its documentation for more.
#[derive(Debug)]
pub struct Take<C> {
    collector: C,
    remaining: AtomicUsize,
}

impl<C> Take<C> {
    pub(in crate::collector) fn new(collector: C, n: usize) -> Self {
        Self {
            collector,
            remaining: n.into(),
        }
    }
}

impl<'this, C> DefineConsumer<'this> for Take<C>
where
    C: DefineConsumer<'this>,
{
    type Consumer = __adapter_take_indexed_internal::Consumer<C::Consumer>;
}

impl<C> ParallelCollectorBase for Take<C>
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
        if self.remaining.load(Ordering::Relaxed) == 0 {
            ControlFlow::Break(())
        } else {
            self.collector.break_hint()
        }
    }

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
        let remaining = self.remaining.get_mut();
        let max_len = if *remaining < len {
            std::mem::take(remaining)
        } else {
            *remaining -= len;
            len
        };

        // We "lie" to the underlying parallel collector that
        // we only have this amount left.
        let (inner_max_len, consumer, commit) = self.collector.parts(max_len);
        // Only meaningful when we have "nested take()."
        // In this case we can choose a new len of the underlying
        // if appropriate.
        let max_len = inner_max_len.min(max_len);

        (
            max_len,
            __adapter_take_indexed_internal::Consumer::new(consumer, max_len),
            commit,
        )
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(<<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output),
    ) {
        let remaining = self.remaining.get_mut();
        let max_len = if *remaining < len {
            std::mem::take(remaining)
        } else {
            *remaining -= len;
            len
        };

        // We "lie" to the underlying parallel collector that
        // we only have this amount left.
        let (inner_max_len, consumer, commit) = self.collector.take_parts(max_len);
        // Only meaningful when we have "nested take()."
        // In this case we can choose a new len of the underlying
        // if appropriate.
        let max_len = inner_max_len.min(max_len);

        (
            max_len,
            __adapter_take_indexed_internal::Consumer::new(consumer, max_len),
            commit,
        )
    }
}

impl<'this, C> DefineUnindexedConsumer<'this> for Take<C>
where
    C: DefineUnindexedConsumer<'this>,
{
    type UnindexedConsumer =
        __adapter_take_unindexed_internal::Consumer<'this, C::UnindexedConsumer>;
}

impl<C> UnindexedParallelCollectorBase for Take<C>
where
    C: UnindexedParallelCollectorBase,
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
            __adapter_take_unindexed_internal::Consumer::new(consumer, &self.remaining),
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
            __adapter_take_unindexed_internal::Consumer::new(consumer, &self.remaining),
            commit,
        )
    }
}

impl<C> Clone for Take<C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            collector: self.collector.clone(),
            remaining: self.remaining.load(Ordering::Relaxed).into(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.collector.clone_from(&source.collector);
        self.remaining
            .store(source.remaining.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

// impl<'this, C> DefineConsumer
#[doc(hidden)]
#[allow(missing_debug_implementations)]
mod __adapter_take_indexed_internal {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing;

    pub struct Consumer<C> {
        consumer: C,
        n: usize,
    }

    impl<C> Consumer<C> {
        #[inline]
        pub(super) fn new(consumer: C, n: usize) -> Self {
            Self { consumer, n }
        }
    }

    impl<C> IntoCollectorBase for Consumer<C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = komadori::collector::Take<C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            // We have to limit by ourselves.
            // Some collectors may be fed more items than neccessary,
            // since we lied to the underlying collector.
            self.consumer.into_collector().take(self.n)
        }
    }

    impl<C> plumbing::ConsumerBase for Consumer<C>
    where
        C: plumbing::ConsumerBase,
    {
        type Combiner = C::Combiner;

        fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
            let index = index.clamp(0, self.n);
            let (consumer, combiner) = self.consumer.split_off_left_at(index);
            self.n -= index;

            (Self { consumer, n: index }, combiner)
        }

        fn break_hint(&self) -> ControlFlow<()> {
            if self.n == 0 {
                ControlFlow::Break(())
            } else {
                self.consumer.break_hint()
            }
        }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
mod __adapter_take_unindexed_internal {
    use std::{
        ops::ControlFlow,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumerBase};

    pub struct Consumer<'a, C> {
        consumer: C,
        remaining: &'a AtomicUsize,
    }

    pub struct IntoCollector<'a, C> {
        collector: C,
        remaining: &'a AtomicUsize,
    }

    impl<'a, C> Consumer<'a, C> {
        #[inline]
        pub(super) fn new(consumer: C, remaining: &'a AtomicUsize) -> Self {
            Self {
                consumer,
                remaining,
            }
        }
    }

    impl<'a, C> IntoCollectorBase for Consumer<'a, C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = IntoCollector<'a, C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            IntoCollector {
                collector: self.consumer.into_collector(),
                remaining: self.remaining,
            }
        }
    }

    impl<C> plumbing::ConsumerBase for Consumer<'_, C>
    where
        C: plumbing::UnindexedConsumerBase,
    {
        type Combiner = C::Combiner;

        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            if self.remaining.load(Ordering::Relaxed) == 0 {
                ControlFlow::Break(())
            } else {
                self.consumer.break_hint()
            }
        }
    }

    impl<C> plumbing::UnindexedConsumerBase for Consumer<'_, C>
    where
        C: plumbing::UnindexedConsumerBase,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                remaining: self.remaining,
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
    }

    impl<C> CollectorBase for IntoCollector<'_, C>
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
            if self.remaining.load(Ordering::Relaxed) == 0 {
                ControlFlow::Break(())
            } else {
                self.collector.break_hint()
            }
        }
    }

    // The implementation is based on `rayon`
    // See: https://docs.rs/rayon/latest/src/rayon/iter/take_any.rs.html
    impl<C, T> Collector<T> for IntoCollector<'_, C>
    where
        C: Collector<T>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            if should_take(self.remaining) {
                self.collector.collect(item)
            } else {
                ControlFlow::Break(())
            }
        }

        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            self.break_hint()?;

            self.collector.collect_many(
                items
                    .into_iter()
                    .take_while(|_| should_take(self.remaining)),
            )
        }

        fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
            if self.break_hint().is_break() {
                self.collector.finish()
            } else {
                self.collector.collect_then_finish(
                    items
                        .into_iter()
                        .take_while(|_| should_take(self.remaining)),
                )
            }
        }
    }

    #[inline]
    fn should_take(remaining: &AtomicUsize) -> bool {
        remaining
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok()
    }
}
