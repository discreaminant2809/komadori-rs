#![allow(deprecated)]

use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{
    iter::{Find, First},
    prelude::*,
};
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn sum_find(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    // We play the worst case: no odd is found at all!
    let mut nums: Box<_> = std::iter::repeat_with(|| rng.random_range(-10_000..=10_000))
        .map(|num| num * 2)
        .take(500_000)
        .collect();

    // Or, if you want to add an odd value, modify the below line.
    nums[0] = 1;

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("sum_find");
    let expected = two_pass(&nums);

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            assert_eq!(expected, $fn_name(&nums));
            group.bench_function(stringify!($fn_name), |bencher| {
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(komadori_find);
    bench_fn!(komadori_first_filter);
    bench_fn!(two_pass);
    bench_fn!(for_loop);
    bench_fn!(fold);

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

#[unsafe(no_mangle)]
fn two_pass(nums: &[i32]) -> (i32, Option<i32>) {
    (
        nums.iter().sum(),
        nums.iter().find(|&&num| num % 2 != 0).copied(),
    )
}

#[unsafe(no_mangle)]
fn for_loop(nums: &[i32]) -> (i32, Option<i32>) {
    let mut sum = 0;
    let mut first_odd = None;

    for &num in nums {
        sum += num;
        if first_odd.is_none() && num % 2 != 0 {
            first_odd = Some(num);
        }
    }

    (sum, first_odd)
}

#[unsafe(no_mangle)]
fn fold(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter().fold((0, None), |(sum, first_odd), &num| {
        (
            sum + num,
            first_odd.or_else(|| (num % 2 != 0).then_some(num)),
        )
    })
}

#[unsafe(no_mangle)]
fn komadori_first_filter(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter()
        .copied()
        .feed_into((0.into_sum(), First::new().filter(|&num| num % 2 != 0)))
}

#[unsafe(no_mangle)]
fn komadori_find(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter()
        .copied()
        .feed_into((0.into_sum(), Find::new(|&num| num % 2 != 0)))
}
