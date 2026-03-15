use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{iter::Find, prelude::*};
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn sum_find(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    // We play the worst case: no odd is found at all!
    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(-10_000..=10_000))
        .map(|num| num * 2)
        .take(500_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("sum_find");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(two_pass_find);
    bench_fn!(for_loop_find);
    bench_fn!(fold_find);
    bench_fn!(bc_find);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = sum_find
}
criterion_main!(benches);

fn two_pass_find(nums: &[i32]) -> (i32, Option<i32>) {
    (
        nums.iter().sum(),
        nums.iter().find(|&&num| num % 2 != 0).copied(),
    )
}

fn for_loop_find(nums: &[i32]) -> (i32, Option<i32>) {
    let mut sum = 0;
    let mut first_odd = None;

    for &num in nums {
        sum += num;
        first_odd = first_odd.or_else(|| (num % 2 != 0).then_some(num));
    }

    (sum, first_odd)
}

fn fold_find(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter().fold((0, None), |(sum, first_odd), &num| {
        (
            sum + num,
            first_odd.or_else(|| (num % 2 != 0).then_some(num)),
        )
    })
}

fn bc_find(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter()
        .copied()
        .feed_into(i32::adding().tee(Find::new(|&num| num % 2 != 0)))
}
