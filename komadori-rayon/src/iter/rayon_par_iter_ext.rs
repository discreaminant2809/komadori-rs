mod indexed;
mod unindexed_fast;
mod unindexed_slow;

use rayon::prelude::*;

use crate::collector::{
    IntoParallelCollector, IntoUnindexedParallelCollector, ParallelCollectorBase,
    UnindexedParallelCollectorBase,
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
                let consumer = unindexed_slow::adapt_consumer(consumer);
                let output = self.drive_unindexed(consumer);
                commit(output);
                collector.finish()
            }

            // We early exit also if the iterator is empty.
            Some(len) if collector.max_afford(len) == 0 => collector.finish(),
            Some(len) => {
                let (consumer, commit) = collector.take_parts(len);
                let consumer = unindexed_fast::adapt_consumer(consumer);
                let output = self.drive_unindexed(consumer);
                commit(output);
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
        let output = indexed::bridge(self, consumer);
        commit(output);
        collector.finish()
    }
}
impl<I> RayonParallelIteratorExt for I where I: ParallelIterator {}
