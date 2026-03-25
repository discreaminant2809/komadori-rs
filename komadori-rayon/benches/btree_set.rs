use std::{collections::BTreeSet, hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use rayon::prelude::*;

fn reduce(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random::<i32>())
        .take(100_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);
    let expected = btree_set_seq(&nums);

    let mut group = criterion.benchmark_group("btree_set");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(btree_set_komadori_unindexed);
    bench_fn!(btree_set_rayon_unindexed);
    bench_fn!(btree_set_seq);

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

fn btree_set_seq(nums: &[i32]) -> BTreeSet<i32> {
    nums.iter().copied().collect()
}

fn btree_set_rayon_unindexed(nums: &[i32]) -> BTreeSet<i32> {
    ForceUnindexed(nums.par_iter().copied()).collect()
}

fn btree_set_komadori_unindexed(nums: &[i32]) -> BTreeSet<i32> {
    let _ = nums;
    // ForceUnindexed(nums.par_iter().copied()).feed_into(BTreeSet::new())
    todo!()
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
