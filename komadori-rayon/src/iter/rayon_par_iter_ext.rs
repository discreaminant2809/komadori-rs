mod bridge;

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

/// Extends `rayon`'s [`ParallelIterator`] and [`IndexedParallelIterator`] with
/// methods to work with parallel collectors.
///
/// This trait is automatically implemented for all `rayon`
/// [`ParallelIterator`] and [`IndexedParallelIterator`] types.
pub trait RayonParallelIteratorExt: ParallelIterator {
    /// Feeds items from this iterator into the provided parallel collector
    /// till the collector stops accumulating or the iterator is exhausted,
    /// and returns the collector’s output.
    ///
    /// The collector must be convertible to
    /// [`UnindexedParallelCollector`](crate::collector::UnindexedParallelCollector).
    /// If you have a collector that only works with the indexed path,
    /// or you want the indexed path explicitly,
    /// use [`feed_into_indexed()`](Self::feed_into_indexed) which can prevent
    /// accidental fallback to the unindexed path and sometimes provide
    /// better performance.
    /// However, this method is already efficient enough since it can utilize
    /// the indexed path whenever possible.
    ///
    /// To use this method, import the [`RayonParallelIteratorExt`] trait.
    fn feed_into<C>(self, collector: C) -> C::Output
    where
        C: IntoUnindexedParallelCollector<Self::Item>,
    {
        let mut collector = collector.into_par_collector();

        match self.opt_len() {
            None => {
                let (consumer, commit) = collector.take_parts_unindexed();
                commit(unindexed_slow_path(self, consumer));
                collector.finish()
            }
            Some(len) => {
                // Sadly, we can't do anything usefully with the actual len
                // for unindexed parallel iterator.
                let (_, consumer, commit) = collector.take_parts(len);
                commit(unindexed_fast_path(self, consumer));
                collector.finish()
            }
        }
    }

    /// Feeds items from this iterator into the provided parallel collector
    /// till the collector stops accumulating or the iterator is exhausted,
    /// and returns the collector’s output.
    ///
    /// This is the indexed version of [`feed_into()`](Self::feed_into),
    /// and is sometimes faster.
    ///
    /// The collector must be convertible to
    /// [`ParallelCollector`](crate::collector::ParallelCollector).
    /// If you do not strictly require the indexed path,
    /// use [`feed_into()`](Self::feed_into),
    /// which is already efficient enough since it can utilize
    /// the indexed path whenever possible.
    ///
    /// To use this method, import the [`RayonParallelIteratorExt`] trait.
    fn feed_into_indexed<C>(self, collector: C) -> C::Output
    where
        Self: IndexedParallelIterator,
        C: IntoParallelCollector<Self::Item>,
    {
        let mut collector = collector.into_par_collector();

        let (actual_len, consumer, commit) = collector.take_parts(self.len());
        commit(indexed_path(self, consumer, actual_len));
        collector.finish()
    }
}
impl<I> RayonParallelIteratorExt for I where I: ParallelIterator {}

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
                self.consumer.break_hint().is_break()
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

fn indexed_path<C, I>(items: I, consumer: C, actual_len: usize) -> C::Output
where
    I: IndexedParallelIterator,
    C: Consumer<IntoCollector: Collector<I::Item>>,
{
    if actual_len < items.len() {
        bridge::bridge(items.take(actual_len), consumer)
    } else {
        bridge::bridge(items, consumer)
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
