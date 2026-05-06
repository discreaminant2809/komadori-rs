use std::{
    fmt::Debug,
    ops::ControlFlow,
    sync::atomic::{AtomicUsize, Ordering},
};

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{DefineSerial, DefineUnindexedSerial},
    },
    helpers::{unique, unique_unindexed},
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

impl<'this, C> DefineSerial<'this> for Take<C>
where
    C: DefineSerial<'this>,
{
    type Serial = unique::Serial<'this, Self, indexed::Serial<C::Serial>>;
}

impl<'this, C> DefineUnindexedSerial<'this> for Take<C>
where
    C: DefineUnindexedSerial<'this>,
{
    type UnindexedSerial =
        unique_unindexed::Serial<'this, Self, unindexed::Serial<'this, C::UnindexedSerial>>;
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
        impl crate::collector::plumbing::Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        let remaining = self.remaining.get_mut();
        let max_len = if *remaining < len {
            std::mem::take(remaining)
        } else {
            *remaining -= len;
            len
        };
        let break_hint = if *remaining == 0 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        };

        // We "lie" to the underlying parallel collector that
        // we only have this amount left.
        let (inner_max_len, consumer, commit) = self.collector.parts(max_len);
        // Only meaningful when we have "nested take()."
        // In this case we can choose a new len of the underlying
        // if appropriate.
        let max_len = inner_max_len.min(max_len);

        unique::uniquify((
            max_len,
            indexed::Consumer::new(consumer, max_len),
            move |output| {
                commit(output)?;
                break_hint
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

        unique::take_uniquify((max_len, indexed::Consumer::new(consumer, max_len), commit))
    }
}

impl<C> UnindexedParallelCollectorBase for Take<C>
where
    C: UnindexedParallelCollectorBase,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl crate::collector::plumbing::UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        unique_unindexed::uniquify((unindexed::Consumer::new(consumer, &self.remaining), commit))
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl crate::collector::plumbing::UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique_unindexed::take_uniquify((unindexed::Consumer::new(consumer, &self.remaining), commit))
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

#[allow(missing_debug_implementations)]
mod indexed {
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

    pub type Serial<C> = komadori::collector::Take<C>;

    impl<C> IntoCollectorBase for Consumer<C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            // We have to limit by ourselves.
            // Some collectors may be fed more items than neccessary,
            // since we lied to the underlying collector.
            self.consumer.into_collector().take(self.n)
        }
    }

    impl<C> plumbing::Consumer for Consumer<C>
    where
        C: plumbing::Consumer,
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

#[allow(missing_debug_implementations)]
mod unindexed {
    use std::{
        ops::ControlFlow,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer<'a, C> {
        consumer: C,
        remaining: &'a AtomicUsize,
    }

    pub struct Serial<'a, C> {
        collector: C,
        remaining: &'a AtomicUsize,
    }

    impl<'a, C> Consumer<'a, C> {
        #[inline]
        pub(super) fn new(consumer: C, remaining: &'a AtomicUsize) -> Self {
            Self { consumer, remaining }
        }
    }

    impl<'a, C> IntoCollectorBase for Consumer<'a, C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<'a, C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector: self.consumer.into_collector(),
                remaining: self.remaining,
            }
        }
    }

    impl<C> plumbing::Consumer for Consumer<'_, C>
    where
        C: UnindexedConsumer,
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

    impl<C> UnindexedConsumer for Consumer<'_, C>
    where
        C: UnindexedConsumer,
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

    impl<C> CollectorBase for Serial<'_, C>
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
    impl<C, T> Collector<T> for Serial<'_, C>
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

            self.collector
                .collect_many(items.into_iter().take_while(|_| should_take(self.remaining)))
        }

        fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
            if self.break_hint().is_break() {
                self.collector.finish()
            } else {
                self.collector
                    .collect_then_finish(items.into_iter().take_while(|_| should_take(self.remaining)))
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
