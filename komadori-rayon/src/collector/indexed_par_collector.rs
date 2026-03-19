use std::ops::ControlFlow;

use super::{ParallelCollectorBase, plumbing::ConsumerFnOnce};

///
pub trait IndexedParallelCollector<T>: ParallelCollectorBase {
    ///
    fn with_consumer<F>(&mut self, len: usize, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: ConsumerFnOnce<T>;

    ///
    fn with_consumer_then_finish<F>(self, len: usize, f: F) -> (F::Output, Self::Output)
    where
        F: ConsumerFnOnce<T>,
    {
        let mut this = self;
        let (ret, _) = this.with_consumer(len, f);
        (ret, this.finish())
    }
}
