use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

///
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

impl<'this, C> DefineConsumer<'this> for Fuse<C>
where
    C: DefineConsumer<'this>,
{
    type Consumer = __adapter_fuse_internal::Consumer<<C as DefineConsumer<'this>>::Consumer>;
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
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        let (actual_len, consumer, commit) = self.collector.parts(len);
        (
            actual_len,
            __adapter_fuse_internal::Consumer {
                consumer,
                break_hint: self.break_hint.into(),
            },
            |output| {
                let cf = commit(output);
                if cf.is_break() {
                    self.break_hint = cf;
                }
                self.break_hint
            },
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
        let (actual_len, consumer, commit) = self.collector.take_parts(len);
        (
            actual_len,
            __adapter_fuse_internal::Consumer {
                consumer,
                break_hint: self.break_hint.into(),
            },
            // We can't set the flag if we cannot obtain the signal from
            // the committer.
            commit,
        )
    }
}

impl<'this, C> DefineUnindexedConsumer<'this> for Fuse<C>
where
    C: DefineUnindexedConsumer<'this>,
{
    type UnindexedConsumer =
        __adapter_fuse_internal::Consumer<<C as DefineUnindexedConsumer<'this>>::UnindexedConsumer>;
}

impl<C> UnindexedParallelCollectorBase for Fuse<C>
where
    C: UnindexedParallelCollectorBase,
{
    fn parts_unindexed<'a>(
            &'a mut self,
        ) -> (
            <Self as crate::collector::plumbing::DefineUnindexedConsumer<'a>>::UnindexedConsumer,
            impl FnOnce(
                <<Self as crate::collector::plumbing::DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
            ) -> ControlFlow<()>,
    ){
        let (consumer, commit) = self.collector.parts_unindexed();
        (
            __adapter_fuse_internal::Consumer {
                consumer,
                break_hint: self.break_hint.into(),
            },
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
            __adapter_fuse_internal::Consumer {
                consumer,
                break_hint: self.break_hint.into(),
            },
            commit,
        )
    }
}

#[doc(hidden)]
pub mod __adapter_fuse_internal {
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
    pub struct IntoCollector<C> {
        collector: C,
        break_hint: ControlFlow<()>,
    }

    impl<C> IntoCollectorBase for Consumer<C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = IntoCollector<C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            IntoCollector {
                collector: self.consumer.into_collector(),
                break_hint: self.break_hint.get(),
            }
        }
    }

    impl<C> plumbing::ConsumerBase for Consumer<C>
    where
        C: plumbing::ConsumerBase,
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

    impl<C> plumbing::UnindexedConsumerBase for Consumer<C>
    where
        C: plumbing::UnindexedConsumerBase,
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

    impl<C> IntoCollector<C> {
        #[inline]
        fn collect_impl(&mut self, f: impl FnOnce(&mut C) -> ControlFlow<()>) -> ControlFlow<()> {
            self.break_hint?;
            self.break_hint = f(&mut self.collector);
            self.break_hint
        }
    }

    impl<C> CollectorBase for IntoCollector<C>
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

    impl<C, T> Collector<T> for IntoCollector<C>
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
