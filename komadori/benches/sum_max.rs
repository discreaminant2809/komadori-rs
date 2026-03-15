use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{cmp::Max, iter::Fold, prelude::*};
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn sum_max(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(-10_000..=10_000))
        .take(500_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("sum_max");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(two_pass);
    bench_fn!(fold_w_initial);
    bench_fn!(fold_wo_initial);
    bench_fn!(bc_tee_with_max_unwrap);
    bench_fn!(bc_tee_with_fold);
    bench_fn!(bc_tee_with_max);
    bench_fn!(for_loop_w_initial);
    bench_fn!(for_loop_wo_initial);

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

fn fold_w_initial(nums: &[i32]) -> (i32, i32) {
    nums.iter()
        .copied()
        .fold((0, i32::MIN), |(sum, max), num| (sum + num, max.max(num)))
}

fn fold_wo_initial(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter()
        .copied()
        .fold((0, None), |(sum, max), num| (sum + num, max.max(Some(num))))
}

fn bc_tee_with_fold(nums: &[i32]) -> (i32, i32) {
    nums.iter()
        .copied()
        .feed_into(i32::adding().tee(Fold::new(i32::MIN, |max, num| *max = (*max).max(num))))
}

fn bc_tee_with_max(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter()
        .copied()
        .feed_into(i32::adding().tee(Max::new()))
}

fn bc_tee_with_max_unwrap(nums: &[i32]) -> (i32, i32) {
    nums.iter().copied().feed_into(
        i32::adding()
            .tee(Max::new())
            .map_output(|(sum, max)| (sum, max.unwrap_or(i32::MIN))),
    )
}

fn two_pass(nums: &[i32]) -> (i32, Option<i32>) {
    (nums.iter().sum(), nums.iter().copied().max())
}

fn for_loop_w_initial(nums: &[i32]) -> (i32, i32) {
    let mut sum = 0;
    let mut max = i32::MIN;

    for &num in nums {
        sum += num;
        max = max.max(num);
    }

    (sum, max)
}

fn for_loop_wo_initial(nums: &[i32]) -> (i32, Option<i32>) {
    let mut sum = 0;
    let mut max = None;

    for &num in nums {
        sum += num;
        max = max.max(Some(num));
    }

    (sum, max)
}
