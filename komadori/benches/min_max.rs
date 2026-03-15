use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use itertools::{Itertools, MinMaxResult};
use komadori::{
    cmp::{Max, Min, MinMax},
    prelude::*,
};
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn min_max(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(-10_000..=10_000))
        .take(500_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("min_max");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(min_max_fold);
    bench_fn!(min_max_itertools);
    bench_fn!(min_max_bc);
    bench_fn!(min_max_bc_tee);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = min_max,
}
criterion_main!(benches);

#[unsafe(no_mangle)]
fn min_max_itertools(nums: &[i32]) -> MinMaxResult<i32> {
    nums.iter().copied().minmax()
}

#[unsafe(no_mangle)]
fn min_max_bc(nums: &[i32]) -> MinMaxResult<i32> {
    nums.iter().copied().feed_into(MinMax::new())
}

#[unsafe(no_mangle)]
fn min_max_bc_tee(nums: &[i32]) -> MinMaxResult<i32> {
    nums.iter()
        .copied()
        .feed_into(
            Min::new()
                .tee(Max::new())
                .map_output(|(min, max)| match (min, max) {
                    (None, None) => MinMaxResult::NoElements,
                    (None, Some(item)) | (Some(item), None) => MinMaxResult::OneElement(item),
                    (Some(min), Some(max)) => MinMaxResult::MinMax(min, max),
                }),
        )
}

#[unsafe(no_mangle)]
fn min_max_fold(nums: &[i32]) -> MinMaxResult<i32> {
    let mut nums = nums.iter().copied();

    let Some(first) = nums.next() else {
        return MinMaxResult::NoElements;
    };

    let Some(second) = nums.next() else {
        return MinMaxResult::OneElement(first);
    };

    let (min, max) = if first < second {
        (first, second)
    } else {
        (second, first)
    };

    let (min, max) = nums.fold((min, max), |(min, max), num| (min.min(num), max.max(num)));

    MinMaxResult::MinMax(min, max)
}
