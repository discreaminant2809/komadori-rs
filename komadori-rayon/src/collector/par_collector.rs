use std::ops::ControlFlow;

use super::{IndexedParallelCollector, plumbing::UnindexedConsumerFnOnce};

///
pub trait ParallelCollector<T>: IndexedParallelCollector<T> {
    ///
    fn with_unindexed_consumer<F>(&mut self, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: UnindexedConsumerFnOnce<T>;

    ///
    fn with_unindexed_consumer_then_finish<F>(self, f: F) -> (F::Output, Self::Output)
    where
        F: UnindexedConsumerFnOnce<T>,
    {
        let mut this = self;
        let (ret, _) = this.with_unindexed_consumer(f);
        (ret, this.finish())
    }
}
