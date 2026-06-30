use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::prelude::*;
use komadori_rayon::prelude::*;
use rand::{RngExt, SeedableRng, rngs::Xoshiro128PlusPlus};
use rayon::prelude::*;

fn reduce(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = Xoshiro128PlusPlus::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random::<i32>())
        .take(500_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);
    let expected = vec_seq(&nums);

    let mut group = criterion.benchmark_group("vec");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    // Different placements may yield vastly different results.
    // It's important run multiple times with a different arrangement each time
    // to have a more informed performance judgment.
    bench_fn!(unindexed_parallel_to_serial);
    bench_fn!(parallel_to_serial);
    bench_fn!(vec_seq);
    bench_fn!(vec_rayon_unindexed);
    bench_fn!(vec_komadori_unindexed);
    bench_fn!(vec_rayon_indexed);
    bench_fn!(vec_komadori_indexed);

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

fn vec_seq(nums: &[i32]) -> Vec<i32> {
    nums.to_vec()
}

fn vec_rayon_indexed(nums: &[i32]) -> Vec<i32> {
    nums.par_iter().copied().collect()
}

fn vec_komadori_indexed(nums: &[i32]) -> Vec<i32> {
    nums.par_iter().copied().feed_into(vec![])
}

fn vec_rayon_unindexed(nums: &[i32]) -> Vec<i32> {
    ForceUnindexed(nums.par_iter().copied()).collect()
}

fn vec_komadori_unindexed(nums: &[i32]) -> Vec<i32> {
    ForceUnindexed(nums.par_iter().copied()).feed_into(vec![])
}

fn parallel_to_serial(nums: &[i32]) -> Vec<i32> {
    nums.iter()
        .feed_into(vec![].into_par_collector().into_collector())
}

#[unsafe(no_mangle)]
fn unindexed_parallel_to_serial(nums: &[i32]) -> Vec<i32> {
    nums.iter().feed_into(
        vec![]
            .into_par_collector()
            .unindexed_only() // Force the unindexed path.
            .into_collector(),
    )
}

struct ForceUnindexed<I>(I);

impl<I> ParallelIterator for ForceUnindexed<I>
where
    I: ParallelIterator,
{
    type Item = I::Item;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        self.0.drive_unindexed(consumer)
    }
}
