use crate::collector::plumbing::{
    Collector, CollectorBase, Consumer as KomadoriConsumer, IntoCollector, IntoCollectorBase,
};
use rayon::iter::plumbing::{Consumer, Folder, Reducer, UnindexedConsumer};

#[inline]
pub fn adapt_consumer<C, T>(consumer: C) -> impl UnindexedConsumer<T, Result = C::Output>
where
    C: KomadoriConsumer<IntoCollector: Collector<T>>,
{
    RayonConsumer {
        state: consumer,
        split_f: |mut consumer, idx| {
            let (left_consumer, combiner) = consumer.split_off_left_at(idx);
            (left_consumer, consumer, |mut left, right| {
                combiner(&mut left, right);
                left
            })
        },
        full_pred: |consumer| consumer.max_afford(1) == 0,
        collector_f: |consumer| consumer.into_collector(),
    }
}

struct RayonConsumer<S, SF, FP, CF, Red, I>
where
    S: Send,
    SF: FnOnce(S, usize) -> (S, S, Red) + Clone + Send,
    FP: Fn(&S) -> bool + Clone + Send,
    CF: FnOnce(S) -> I + Clone + Send,
    I: IntoCollectorBase<Output: Send>,
    Red: FnOnce(I::Output, I::Output) -> I::Output,
{
    state: S,
    split_f: SF,
    full_pred: FP,
    collector_f: CF,
}

struct RayonFolder<C> {
    // rayon does something like `if !folder.full() { folder = folder.consume(item) }`,
    // and if the collector in `folder.consume(item)` returns `Break(())`,
    // the usage of `folder.full()` in the next iteration is invalid.
    // So, we have to fuse.
    collector: komadori::collector::Fuse<C>,
}

struct RayonReducer<F>(F);

impl<S, SF, FP, CF, Red, I, T> Consumer<T> for RayonConsumer<S, SF, FP, CF, Red, I>
where
    S: Send,
    SF: FnOnce(S, usize) -> (S, S, Red) + Clone + Send,
    FP: Fn(&S) -> bool + Clone + Send,
    CF: FnOnce(S) -> I + Clone + Send,
    I: IntoCollector<T, Output: Send>,
    Red: FnOnce(I::Output, I::Output) -> I::Output,
{
    type Folder = RayonFolder<I::IntoCollector>;

    // This is why the adapter isn't as straghtforward.
    // We cannot name the closure returned via RPITIT.
    type Reducer = RayonReducer<Red>;

    type Result = I::Output;

    #[inline]
    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, combiner) = self.split_f.clone()(self.state, index);
        (
            Self {
                state: left,
                split_f: self.split_f.clone(),
                full_pred: self.full_pred.clone(),
                collector_f: self.collector_f.clone(),
            },
            Self {
                state: right,
                split_f: self.split_f,
                full_pred: self.full_pred,
                collector_f: self.collector_f,
            },
            RayonReducer(combiner),
        )
    }

    #[inline]
    fn into_folder(self) -> Self::Folder {
        RayonFolder {
            collector: (self.collector_f)(self.state).into_collector().fuse(),
        }
    }

    #[inline]
    fn full(&self) -> bool {
        (self.full_pred)(&self.state)
    }
}

impl<S, SF, FP, CF, Red, I, T> UnindexedConsumer<T> for RayonConsumer<S, SF, FP, CF, Red, I>
where
    S: Send,
    SF: FnOnce(S, usize) -> (S, S, Red) + Clone + Send,
    FP: Fn(&S) -> bool + Clone + Send,
    CF: FnOnce(S) -> I + Clone + Send,
    I: IntoCollector<T, Output: Send>,
    Red: FnOnce(I::Output, I::Output) -> I::Output,
{
    fn split_off_left(&self) -> Self {
        panic!("unindexed path used when `opt_len()` returned `Some(len)`")
    }

    fn to_reducer(&self) -> Self::Reducer {
        panic!("unindexed path used when `opt_len()` returned `Some(len)`")
    }
}

impl<C, T> Folder<T> for RayonFolder<C>
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

impl<F, O> Reducer<O> for RayonReducer<F>
where
    F: FnOnce(O, O) -> O,
{
    #[inline]
    fn reduce(self, left: O, right: O) -> O {
        self.0(left, right)
    }
}
