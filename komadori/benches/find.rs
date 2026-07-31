#![allow(deprecated)]

use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{
    iter::{Find, First},
    prelude::*,
};
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn find(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    macro_rules! bench_fn {
        ($expected:expr, $group:ident.$fn_name:ident($nums:expr)) => {
            assert_eq!($expected, $fn_name($nums));
            $group.bench_function(stringify!($fn_name), |bencher| {
                bencher.iter(|| $fn_name(black_box($nums)));
            });
        };
    }

    let mut nums: Box<_> = std::iter::repeat_with(|| rng.random_range(1..=i32::MAX))
        .take(500_000)
        .collect();
    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);
    nums[400_000] = 0;
    let mut group = criterion.benchmark_group("find_found_late");
    let expected = iterator(&nums);
    bench_fn!(expected, group.manual(&nums));
    bench_fn!(expected, group.komadori_first_filter(&nums));
    bench_fn!(expected, group.iterator(&nums));
    bench_fn!(expected, group.komadori_manual_collect_find(&nums));
    group.finish();

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(1..=i32::MAX))
        .take(500_000)
        .collect();
    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);
    let mut group = criterion.benchmark_group("find_not_found");
    let expected = iterator(&nums);
    bench_fn!(expected, group.manual(&nums));
    bench_fn!(expected, group.komadori_first_filter(&nums));
    bench_fn!(expected, group.iterator(&nums));
    bench_fn!(expected, group.komadori_manual_collect_find(&nums));
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = find
}
criterion_main!(benches);

#[unsafe(no_mangle)]
#[allow(clippy::manual_find)]
fn manual(nums: &[i32]) -> Option<i32> {
    for &num in nums {
        if num == 0 {
            return Some(num);
        }
    }

    None
}

#[unsafe(no_mangle)]
fn iterator(nums: &[i32]) -> Option<i32> {
    nums.iter().find(|&&num| num == 0).copied()
}

// Use manual `collect` because `tee_*` uses this method anyway.
// Not to mention `Find`'s `collect_then_finish` forwards to `find()`.
#[unsafe(no_mangle)]
fn komadori_manual_collect_find(nums: &[i32]) -> Option<i32> {
    let mut collector = Find::new(|&num| num == 0);
    let mut nums = nums.iter().copied();

    while let Some(num) = nums.next()
        && collector.collect(num).is_continue()
    {}

    collector.finish()
}

#[unsafe(no_mangle)]
fn komadori_first_filter(nums: &[i32]) -> Option<i32> {
    nums.iter()
        .copied()
        .feed_into(First::new().filter(|&num| num == 0))
}
