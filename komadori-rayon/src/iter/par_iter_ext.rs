use komadori::{collector::Fuse, prelude::*};
use rayon::{
    iter::plumbing::{
        Consumer as RayonConsumer, Folder, Reducer, UnindexedConsumer as RayonUnindexedConsumer,
    },
    prelude::*,
};

use crate::collector::{
    IntoParallelCollector, IntoUnindexedParallelCollector, ParallelCollectorBase,
    UnindexedParallelCollectorBase,
    plumbing::{Combiner, Consumer, UnindexedConsumer},
};

///
pub trait ParallelIteratorExt: ParallelIterator {
    ///
    fn feed_into<C>(self, collector: C) -> C::Output
    where
        C: IntoUnindexedParallelCollector<Self::Item>,
    {
        let collector = collector.into_par_collector();

        match self.opt_len() {
            None => {
                collector
                    .with_unindexed_consumer(|consumer, _| {
                        ((), unindexed_slow_path(self, consumer))
                    })
                    .1
            }
            Some(len) => {
                collector
                    .with_consumer(len, |_, consumer, _| {
                        ((), unindexed_fast_path(self, consumer))
                    })
                    .1
            }
        }
    }

    ///
    fn feed_into_indexed<C>(self, collector: C) -> C::Output
    where
        Self: IndexedParallelIterator,
        C: IntoParallelCollector<Self::Item>,
    {
        collector
            .into_par_collector()
            .with_consumer(self.len(), move |actual_len, consumer, _| {
                ((), indexed_path(self, consumer, actual_len))
            })
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

fn unindexed_slow_path<C, I>(items: I, consumer: C) -> C::Output
where
    I: ParallelIterator,
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

    items.drive_unindexed(ConsumerAdapter { consumer })
}

fn unindexed_fast_path<C, I>(items: I, consumer: C) -> C::Output
where
    I: ParallelIterator,
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

    items.drive_unindexed(ConsumerAdapter { consumer })
}

struct FeedIntoIndexed<I> {
    this: I,
}

fn indexed_path<C, I>(items: I, consumer: C, actual_len: usize) -> C::Output
where
    I: IndexedParallelIterator,
    C: Consumer<I::Item>,
{
    define_consumer_adapter_and_impl_consumer!();

    let consumer = ConsumerAdapter { consumer };
    if actual_len < items.len() {
        items.take(actual_len).drive(consumer)
    } else {
        items.drive(consumer)
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
