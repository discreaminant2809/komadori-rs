use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::prelude::*;
use rand::{prelude::*, rngs::Xoshiro128PlusPlus};

fn step_by(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = Xoshiro128PlusPlus::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(-10_000..=10_000))
        .take(100_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("step_by");
    let expected = std_iterator(&nums);

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(std_iterator);
    bench_fn!(komadori_way);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = step_by
}
criterion_main!(benches);

const STEP: usize = 2;

#[unsafe(no_mangle)]
fn std_iterator(nums: &[i32]) -> Vec<i32> {
    nums.iter().copied().step_by(STEP).collect()
}

#[unsafe(no_mangle)]
fn komadori_way(nums: &[i32]) -> Vec<i32> {
    nums.iter().feed_into(vec![].into_collector().step_by(STEP))
}
