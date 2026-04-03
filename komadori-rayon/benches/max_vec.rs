use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{cmp::Max, prelude::*};
use komadori_rayon::{cmp::ParMax, prelude::*};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use rayon::prelude::*;

fn reduce(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random::<i32>())
        .take(1_000_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);
    let expected = seq_one_pass(&nums);

    let mut group = criterion.benchmark_group("max_vec");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(rayon_komadori);
    bench_fn!(rayon_komadori_custom_bridge);
    bench_fn!(rayon_extend);
    bench_fn!(rayon_two_pass);
    bench_fn!(rayon_fold_reduce);
    bench_fn!(seq_one_pass);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = reduce
}
criterion_main!(benches);

fn seq_one_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    nums.iter()
        .copied()
        .feed_into(Max::new().map_output(Option::unwrap).tee(vec![]))
}

fn rayon_two_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    let max = nums.par_iter().copied().max().unwrap();
    let v = nums.par_iter().copied().collect();
    (max, v)
}

fn rayon_fold_reduce(nums: &[i32]) -> (i32, Vec<i32>) {
    #[inline]
    fn id() -> (i32, Vec<i32>) {
        (i32::MIN, vec![])
    }

    nums.par_iter()
        .fold(id, |(max, mut v), &num| {
            v.push(num);
            (max.max(num), { v })
        })
        .reduce(id, |(max1, mut v1), (max2, mut v2)| {
            v1.append(&mut v2);
            (max1.max(max2), v1)
        })
}

fn rayon_extend(nums: &[i32]) -> (i32, Vec<i32>) {
    #[derive(Default)]
    struct MaxExtendI32 {
        max: Option<i32>,
    }

    impl ParallelExtend<i32> for MaxExtendI32 {
        fn par_extend<I>(&mut self, par_iter: I)
        where
            I: IntoParallelIterator<Item = i32>,
        {
            self.max = self.max.max(par_iter.into_par_iter().max());
        }
    }

    let (max, v): (MaxExtendI32, Vec<_>) = nums.par_iter().map(|&num| (num, num)).unzip();
    (max.max.unwrap(), v)
}

fn rayon_komadori(nums: &[i32]) -> (i32, Vec<i32>) {
    let (max, v) = nums
        .par_iter()
        .copied()
        // FIXED: use `map_output()` when it's implemented
        .feed_into(ParMax::new().tee(vec![]));

    (max.unwrap(), v)
}

fn rayon_komadori_custom_bridge(nums: &[i32]) -> (i32, Vec<i32>) {
    let mut collector = ParMax::new().tee(vec![]);
    let (_, consumer, commit) = collector.take_parts(nums.len());

    let output = custom_bridge::bridge(nums.par_iter().copied(), consumer);
    commit(output);

    let (max, v) = collector.finish();
    (max.unwrap(), v)
}

mod custom_bridge {
    use komadori::prelude::{Collector, CollectorBase};
    use komadori_rayon::collector::plumbing::{Combiner, Consumer};
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
        C: Consumer<I::Item>,
    {
        let len = par_iter.len();
        return par_iter.with_producer(Callback { len, consumer });

        struct Callback<C> {
            len: usize,
            consumer: C,
        }

        impl<C, I> ProducerCallback<I> for Callback<C>
        where
            C: Consumer<I>,
        {
            type Output = C::Output;
            fn callback<P>(self, producer: P) -> C::Output
            where
                P: Producer<Item = I>,
            {
                bridge_producer_consumer(self.len, producer, self.consumer)
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

    fn bridge_producer_consumer<P, C>(len: usize, producer: P, consumer: C) -> C::Output
    where
        P: Producer,
        C: Consumer<P::Item>,
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
            C: Consumer<P::Item>,
        {
            if consumer.break_hint().is_break() {
                consumer.into_collector().finish()
            } else if splitter.try_split(len, migrated) {
                let mid = len / 2;
                let (left_producer, right_producer) = producer.split_at(mid);
                let ((left_consumer, combiner), right_consumer) =
                    (consumer.split_off_left_at(mid), consumer);
                let (mut left_result, right_result) = join_context(
                    |context| {
                        helper(
                            mid,
                            context.migrated(),
                            splitter,
                            left_producer,
                            left_consumer,
                        )
                    },
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
}
