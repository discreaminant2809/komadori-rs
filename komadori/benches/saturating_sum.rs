use std::{hint::black_box, num::Saturating, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::prelude::*;
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn sum_max(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    // 1K * 500K = 500M < u32::MAX => No overflow.
    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(..=1_000_u32))
        .take(500_000)
        .collect();
    let nums = &nums[..];

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("saturating_sum");
    let expected = iter_way(nums);

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(nums), expected);
                bencher.iter(|| $fn_name(black_box(nums)));
            });
        };
    }

    bench_fn!(komadori_way);
    bench_fn!(iter_way);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = sum_max
}
criterion_main!(benches);

#[unsafe(no_mangle)]
fn iter_way(nums: &[u32]) -> u32 {
    nums.iter()
        .map(|&num| Saturating(num))
        .sum::<Saturating<_>>()
        .0
}

#[unsafe(no_mangle)]
fn komadori_way(nums: &[u32]) -> u32 {
    nums.iter()
        .map(|&num| Saturating(num))
        .feed_into(Saturating(0_u32).into_sum())
        .0
}
