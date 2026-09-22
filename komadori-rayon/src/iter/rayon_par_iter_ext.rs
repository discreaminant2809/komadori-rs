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
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::{prelude::*, cmp::ParMax};
    ///
    /// let mut nums = vec![9];
    /// let (_, max) = [4, 7, 6, 3]
    ///     .into_par_iter()
    ///     .filter(|&num| num % 2 == 0)
    ///     .feed_into((&mut nums, ParMax::new()));
    ///
    /// assert_eq!(nums, [9, 4, 6]);
    /// assert_eq!(max, Some(6));
    /// ```
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
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::{prelude::*, iter::ParFirst};
    ///
    /// let mut nums = vec![9];
    /// let (_, first_odd) = [4, 7, 3]
    ///     .into_par_iter()
    ///     .feed_into_indexed((
    ///         &mut nums,
    ///         ParFirst::new()
    ///             .filter(|&(_, num)| num % 2 != 0)
    ///             .enumerate(),
    ///     ));
    ///
    /// assert_eq!(nums, [9, 4, 7, 3]);
    /// assert_eq!(first_odd, Some((1, 7)));
    /// ```
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
    bridge::bridge(items, consumer)
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
