use komadori::{collector::Fuse, prelude::*};
use rayon::{
    iter::plumbing::{
        Consumer as RayonConsumer, Folder, Reducer, UnindexedConsumer as RayonUnindexedConsumer,
    },
    prelude::*,
};

use crate::collector::{
    IndexedParallelCollector, IntoIndexedParallelCollector, IntoParallelCollector,
    ParallelCollector,
    plumbing::{Combiner, Consumer, ConsumerFnOnce, UnindexedConsumer, UnindexedConsumerFnOnce},
};

///
pub trait ParallelIteratorExt: ParallelIterator {
    ///
    fn feed_into<C>(self, collector: C) -> C::Output
    where
        C: IntoParallelCollector<Self::Item>,
    {
        let collector = collector.into_par_collector();

        match self.opt_len() {
            Some(len) => {
                collector
                    .with_consumer_then_finish(len, FeedIntoUnindexed { this: self })
                    .1
            }
            None => {
                collector
                    .with_unindexed_consumer_then_finish(FeedIntoUnindexed { this: self })
                    .1
            }
        }
    }

    ///
    fn feed_into_indexed<C>(self, collector: C) -> C::Output
    where
        Self: IndexedParallelIterator,
        C: IntoIndexedParallelCollector<Self::Item>,
    {
        collector
            .into_par_collector()
            .with_consumer_then_finish(self.len(), FeedIntoIndexed { this: self })
            .1
    }
}
impl<I> ParallelIteratorExt for I where I: ParallelIterator {}

macro_rules! define_consumer_adapter_and_impl_consumer {
    () => {
        struct ConsumerAdapter<C> {
            consumer: C,
        }

        impl<C, T> RayonConsumer<T> for ConsumerAdapter<C>
        where
            C: Consumer<T>,
        {
            type Folder = FolderAdapter<C::IntoCollector>;

            type Reducer = ReducerAdapter<C::Combiner>;

            type Result = C::Output;

            #[inline]
            fn split_at(mut self, index: usize) -> (Self, Self, Self::Reducer) {
                let (left, combiner) = self.consumer.split_off_left_at(index);
                (Self { consumer: left }, self, ReducerAdapter { combiner })
            }

            #[inline]
            fn into_folder(self) -> Self::Folder {
                FolderAdapter {
                    collector: self.consumer.into_collector().fuse(),
                }
            }

            #[inline]
            fn full(&self) -> bool {
                self.consumer.break_hint().is_break()
            }
        }
    };
}

struct FeedIntoUnindexed<I> {
    this: I,
}

impl<I> UnindexedConsumerFnOnce<I::Item> for FeedIntoUnindexed<I>
where
    I: ParallelIterator,
{
    type Output = ();

    fn call_once<C>(self, consumer: C) -> (Self::Output, C::Output)
    where
        C: UnindexedConsumer<I::Item>,
    {
        define_consumer_adapter_and_impl_consumer!();

        impl<C, T> RayonUnindexedConsumer<T> for ConsumerAdapter<C>
        where
            C: UnindexedConsumer<T>,
        {
            #[inline]
            fn split_off_left(&self) -> Self {
                Self {
                    consumer: self.consumer.split_off_left(),
                }
            }

            #[inline]
            fn to_reducer(&self) -> Self::Reducer {
                ReducerAdapter {
                    combiner: self.consumer.to_combiner(),
                }
            }
        }

        ((), self.this.drive_unindexed(ConsumerAdapter { consumer }))
    }
}

impl<I> ConsumerFnOnce<I::Item> for FeedIntoUnindexed<I>
where
    I: ParallelIterator,
{
    type Output = ();

    fn call_once<C>(self, _: Option<usize>, consumer: C) -> (Self::Output, C::Output)
    where
        C: Consumer<I::Item>,
    {
        define_consumer_adapter_and_impl_consumer!();

        impl<C, T> RayonUnindexedConsumer<T> for ConsumerAdapter<C>
        where
            C: Consumer<T>,
        {
            fn split_off_left(&self) -> Self {
                panic!("unindexed path used when opt_len() returned Some(len)")
            }

            fn to_reducer(&self) -> Self::Reducer {
                panic!("unindexed path used when opt_len() returned Some(len)")
            }
        }

        ((), self.this.drive_unindexed(ConsumerAdapter { consumer }))
    }
}

struct FeedIntoIndexed<I> {
    this: I,
}

impl<I> ConsumerFnOnce<I::Item> for FeedIntoIndexed<I>
where
    I: IndexedParallelIterator,
{
    type Output = ();

    fn call_once<C>(self, actual_len: Option<usize>, consumer: C) -> (Self::Output, C::Output)
    where
        C: Consumer<I::Item>,
    {
        define_consumer_adapter_and_impl_consumer!();

        let consumer = ConsumerAdapter { consumer };
        let output = match actual_len {
            Some(actual_len) if actual_len < self.this.len() => {
                self.this.take(actual_len).drive(consumer)
            }
            _ => self.this.drive(consumer),
        };

        ((), output)
    }
}

struct FolderAdapter<C> {
    // rayon does something like `if !folder.full() { folder = folder.consume(item) }`,
    // and if the collector in `folder.consume(item)` returns `Break(())`,
    // the usage of `folder.full()` in the next iteration is invalid.
    // So, we have to fuse.
    collector: Fuse<C>,
}

impl<C, T> Folder<T> for FolderAdapter<C>
where
    C: Collector<T>,
{
    type Result = C::Output;

    #[inline]
    fn consume(mut self, item: T) -> Self {
        let _ = self.collector.collect(item);
        self
    }

    #[inline]
    fn complete(self) -> Self::Result {
        self.collector.finish()
    }

    #[inline]
    fn full(&self) -> bool {
        self.collector.break_hint().is_break()
    }

    #[inline]
    fn consume_iter<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let _ = self.collector.collect_many(iter);
        self
    }
}

struct ReducerAdapter<C> {
    combiner: C,
}

impl<C, O> Reducer<O> for ReducerAdapter<C>
where
    C: Combiner<O>,
{
    #[inline]
    fn reduce(self, mut left: O, right: O) -> O {
        self.combiner.combine(&mut left, right);
        left
    }
}
