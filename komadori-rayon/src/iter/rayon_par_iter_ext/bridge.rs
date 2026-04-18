//! Custom bridge implementation that use our consumers instead of `rayon`'s
//! for potentially more performance.
//!
//! See `max_vec` benchmark for why.
//!
//! Credit: <https://docs.rs/rayon/latest/src/rayon/iter/plumbing/mod.rs.html>

use crate::collector::plumbing::{Combiner, Consumer};
use komadori::prelude::*;
use rayon::{
    iter::{
        IndexedParallelIterator,
        plumbing::{Producer, ProducerCallback},
    },
    join_context,
};

pub fn bridge<I, C>(par_iter: I, consumer: C) -> C::Output
where
    I: IndexedParallelIterator,
    C: Consumer<IntoCollector: Collector<I::Item>>,
{
    let len = par_iter.len();
    return par_iter.with_producer(Callback { len, consumer });

    struct Callback<C> {
        len: usize,
        consumer: C,
    }

    impl<C, T> ProducerCallback<T> for Callback<C>
    where
        C: Consumer<IntoCollector: Collector<T>>,
    {
        type Output = C::Output;
        fn callback<P>(self, producer: P) -> C::Output
        where
            P: Producer<Item = T>,
        {
            bridge_producer_consumer(self.len, producer, self.consumer)
        }
    }
}

fn bridge_producer_consumer<P, C>(len: usize, producer: P, consumer: C) -> C::Output
where
    P: Producer,
    C: Consumer<IntoCollector: Collector<P::Item>>,
{
    let splitter = LengthSplitter::new(producer.min_len(), producer.max_len(), len);
    return helper(len, false, splitter, producer, consumer);

    fn helper<P, C>(
        len: usize,
        migrated: bool,
        mut splitter: LengthSplitter,
        producer: P,
        mut consumer: C,
    ) -> C::Output
    where
        P: Producer,
        C: Consumer<IntoCollector: Collector<P::Item>>,
    {
        if consumer.break_hint().is_break() {
            consumer.into_collector().finish()
        } else if splitter.try_split(len, migrated) {
            let mid = len / 2;
            let (left_producer, right_producer) = producer.split_at(mid);
            let ((left_consumer, combiner), right_consumer) = (consumer.split_off_left_at(mid), consumer);

            let (mut left_result, right_result) = join_context(
                |context| helper(mid, context.migrated(), splitter, left_producer, left_consumer),
                |context| {
                    helper(
                        len - mid,
                        context.migrated(),
                        splitter,
                        right_producer,
                        right_consumer,
                    )
                },
            );

            combiner.combine(&mut left_result, right_result);
            left_result
        } else {
            consumer
                .into_collector()
                .collect_then_finish(producer.into_iter())
        }
    }
}

#[derive(Clone, Copy)]
struct Splitter {
    splits: usize,
}

impl Splitter {
    #[inline]
    fn new() -> Splitter {
        Splitter {
            splits: rayon::current_num_threads(),
        }
    }

    #[inline]
    fn try_split(&mut self, stolen: bool) -> bool {
        let Splitter { splits } = *self;

        if stolen {
            self.splits = Ord::max(rayon::current_num_threads(), self.splits / 2);
            true
        } else if splits > 0 {
            self.splits /= 2;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy)]
struct LengthSplitter {
    inner: Splitter,
    min: usize,
}

impl LengthSplitter {
    #[inline]
    fn new(min: usize, max: usize, len: usize) -> LengthSplitter {
        let mut splitter = LengthSplitter {
            inner: Splitter::new(),
            min: Ord::max(min, 1),
        };
        let min_splits = len / Ord::max(max, 1);

        if min_splits > splitter.inner.splits {
            splitter.inner.splits = min_splits;
        }

        splitter
    }

    #[inline]
    fn try_split(&mut self, len: usize, stolen: bool) -> bool {
        len / 2 >= self.min && self.inner.try_split(stolen)
    }
}
