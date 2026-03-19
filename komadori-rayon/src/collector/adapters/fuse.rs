use std::ops::ControlFlow;

use crate::collector::{
    IndexedParallelCollector, IntoParallelCollectorBase, ParallelCollector, ParallelCollectorBase,
    plumbing,
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
}

impl<C, T> IndexedParallelCollector<T> for Fuse<C>
where
    C: IndexedParallelCollector<T>,
{
    fn with_consumer<F>(&mut self, len: usize, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::ConsumerFnOnce<T>,
    {
        if self.break_hint.is_break() {
            return ().into_par_collector().with_consumer(0, f);
        }

        let (ret, cf) = self.collector.with_consumer(len, f);
        self.break_hint = cf;
        (ret, self.break_hint)
    }

    fn with_consumer_then_finish<F>(self, len: usize, f: F) -> (F::Output, Self::Output)
    where
        F: plumbing::ConsumerFnOnce<T>,
    {
        if self.break_hint.is_break() {
            (
                ().into_par_collector().with_consumer_then_finish(0, f).0,
                self.collector.finish(),
            )
        } else {
            self.collector.with_consumer_then_finish(len, f)
        }
    }
}

impl<C, T> ParallelCollector<T> for Fuse<C>
where
    C: ParallelCollector<T>,
{
    fn with_unindexed_consumer<F>(&mut self, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::UnindexedConsumerFnOnce<T>,
    {
        if self.break_hint.is_break() {
            return ().into_par_collector().with_unindexed_consumer(f);
        }

        let (ret, cf) = self.collector.with_unindexed_consumer(f);
        self.break_hint = cf;
        (ret, self.break_hint)
    }

    fn with_unindexed_consumer_then_finish<F>(self, f: F) -> (F::Output, Self::Output)
    where
        F: plumbing::UnindexedConsumerFnOnce<T>,
    {
        if self.break_hint.is_break() {
            (
                ().into_par_collector()
                    .with_unindexed_consumer_then_finish(f)
                    .0,
                self.collector.finish(),
            )
        } else {
            self.collector.with_unindexed_consumer_then_finish(f)
        }
    }
}
