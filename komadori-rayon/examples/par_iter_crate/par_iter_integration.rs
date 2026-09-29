mod unindexed_fast;
mod unindexed_slow;

use par_iter::prelude::*;

use komadori_rayon::prelude::*;

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

    fn feed_into_indexed<C>(self, collector: C) -> C::Output
    where
        Self: IndexedParallelIterator,
        C: IntoParallelCollector<Self::Item>,
    {
        let len = self.len();
        let mut collector = collector.into_par_collector();

        // We early exit also if the iterator is empty.
        if collector.max_afford(len) == 0 {
            return collector.finish();
        }

        let (consumer, commit) = collector.take_parts(len);
        let consumer = unindexed_fast::adapt_consumer(consumer);
        let output = self.drive_unindexed(consumer);
        commit(output);
        collector.finish()
    }
}
impl<I> ParIterParallelIteratorExt for I where I: ParallelIterator {}
