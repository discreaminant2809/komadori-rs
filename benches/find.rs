use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{iter::Find, prelude::*};
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn find(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    macro_rules! bench_fn {
        ($group:ident.$fn_name:ident($nums:expr)) => {
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
    bench_fn!(group.iter_find_0(&nums));
    bench_fn!(group.bc_collect_find_0(&nums));
    bench_fn!(group.bc_collect_then_finish_find_0(&nums));
    group.finish();

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(1..=i32::MAX))
        .take(500_000)
        .collect();
    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);
    let mut group = criterion.benchmark_group("find_not_found");
    bench_fn!(group.iter_find_0(&nums));
    bench_fn!(group.bc_collect_find_0(&nums));
    bench_fn!(group.bc_collect_then_finish_find_0(&nums));
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

fn iter_find_0(nums: &[i32]) -> Option<i32> {
    nums.iter().find(|&&num| num == 0).copied()
}

// Use manual `collect` because `tee_*` uses this method anyway.
// Not to mention `Find`'s `collect_then_finish` forwards to `find()`.
fn bc_collect_find_0(nums: &[i32]) -> Option<i32> {
    let mut collector = Find::new(|&num| num == 0);
    let mut nums = nums.iter().copied();

    while let Some(num) = nums.next()
        && collector.collect(num).is_continue()
    {}

    collector.finish()
}

fn bc_collect_then_finish_find_0(nums: &[i32]) -> Option<i32> {
    nums.iter().feed_into(Find::new(|&&num| num == 0)).copied()
}
