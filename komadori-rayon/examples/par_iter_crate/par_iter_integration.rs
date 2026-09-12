use komadori::{collector::Fuse, prelude::*};
use par_iter::{
    iter::plumbing::{
        Consumer as RayonConsumer, Folder, Reducer, UnindexedConsumer as RayonUnindexedConsumer,
    },
    prelude::*,
};

use komadori_rayon::{
    collector::plumbing::{Combiner, Consumer, UnindexedConsumer},
    prelude::*,
};

pub trait ParIterParallelIteratorExt: ParallelIterator {
    fn feed_into<C>(self, collector: C) -> C::Output
    where
        C: IntoUnindexedParallelCollector<Self::Item>,
    {
        let mut collector = collector.into_par_collector();

        match self.opt_len() {
            None if collector.max_afford(1) == 0 => collector.finish(),
            None => {
                let (consumer, commit) = collector.take_unindexed_parts();
                commit(unindexed_slow_path(self, consumer));
                collector.finish()
            }

            // We early exit also if the iterator is empty.
            Some(len) if collector.max_afford(len) == 0 => collector.finish(),
            Some(len) => {
                let (consumer, commit) = collector.take_parts(len);
                commit(unindexed_fast_path(self, consumer));
                collector.finish()
            }
        }
    }

    fn feed_into_indexed<C>(self, collector: C) -> C::Output
    where
        Self: IndexedParallelIterator,
        C: IntoParallelCollector<Self::Item>,
    {
        let mut collector = collector.into_par_collector();
        // We early exit also if the iterator is empty.
        if collector.max_afford(self.len()) == 0 {
            return collector.finish();
        }

        let (consumer, commit) = collector.take_parts(self.len());
        commit(indexed_path(self, consumer));
        collector.finish()
    }
}
impl<I> ParIterParallelIteratorExt for I where I: ParallelIterator {}

macro_rules! define_consumer_adapter_and_impl_consumer {
    () => {
        struct ConsumerAdapter<C> {
            consumer: C,
        }

        impl<C, T> RayonConsumer<T> for ConsumerAdapter<C>
        where
            C: Consumer<IntoCollector: Collector<T>>,
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
                self.consumer.max_afford(1) == 0
            }
        }
    };
}

fn unindexed_slow_path<C, I>(items: I, consumer: C) -> C::Output
where
    I: ParallelIterator,
    C: UnindexedConsumer<IntoCollector: Collector<I::Item>>,
{
    define_consumer_adapter_and_impl_consumer!();

    impl<C, T> RayonUnindexedConsumer<T> for ConsumerAdapter<C>
    where
        C: UnindexedConsumer<IntoCollector: Collector<T>>,
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

    items.drive_unindexed(ConsumerAdapter { consumer })
}

fn unindexed_fast_path<C, I>(items: I, consumer: C) -> C::Output
where
    I: ParallelIterator,
    C: Consumer<IntoCollector: Collector<I::Item>>,
{
    define_consumer_adapter_and_impl_consumer!();

    impl<C, T> RayonUnindexedConsumer<T> for ConsumerAdapter<C>
    where
        C: Consumer<IntoCollector: Collector<T>>,
    {
        fn split_off_left(&self) -> Self {
            panic!("unindexed path used when opt_len() returned Some(len)")
        }

        fn to_reducer(&self) -> Self::Reducer {
            panic!("unindexed path used when opt_len() returned Some(len)")
        }
    }

    items.drive_unindexed(ConsumerAdapter { consumer })
}

fn indexed_path<C, I>(items: I, consumer: C) -> C::Output
where
    I: IndexedParallelIterator,
    C: Consumer<IntoCollector: Collector<I::Item>>,
{
    define_consumer_adapter_and_impl_consumer!();

    items.drive(ConsumerAdapter { consumer })
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
        self.collector.max_afford(1) == 0
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
