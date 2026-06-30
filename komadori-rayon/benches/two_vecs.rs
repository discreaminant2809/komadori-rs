use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori_rayon::prelude::*;
use rand::{prelude::*, rngs::Xoshiro128PlusPlus};
use rayon::prelude::*;

fn reduce(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = Xoshiro128PlusPlus::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random::<i32>())
        .take(1_000_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);
    let expected = two_vecs_seq(&nums);

    let mut group = criterion.benchmark_group("two_vecs");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(two_vecs_komadori);
    bench_fn!(two_vecs_rayon);
    bench_fn!(two_vecs_seq);

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

fn two_vecs_seq(nums: &[i32]) -> (Vec<i32>, Vec<i32>) {
    nums.iter().map(|&num| (num, num)).unzip()
}

fn two_vecs_komadori(nums: &[i32]) -> (Vec<i32>, Vec<i32>) {
    nums.par_iter()
        .copied()
        .feed_into_indexed(vec![].into_par_collector().tee(vec![]))
}

fn two_vecs_rayon(nums: &[i32]) -> (Vec<i32>, Vec<i32>) {
    nums.par_iter().map(|&num| (num, num)).unzip()
}
