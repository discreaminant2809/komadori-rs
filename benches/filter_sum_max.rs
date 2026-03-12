use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{cmp::Max, prelude::*};
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn filter(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(0..1_000))
        .take(100_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("filter_sum_max");
    let expected = filter_sum_max_iter(&nums);

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(filter_sum_max_iter);
    bench_fn!(filter_sum_max_komadori);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = filter
}
criterion_main!(benches);

#[unsafe(no_mangle)]
fn filter_sum_max_iter(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter().fold((0, None), |(mut sum, mut max), &num| {
        if num % 2 == 0 {
            sum += num;
        }

        if let Some(max) = &mut max {
            *max = (*max).max(num);
        } else {
            max = Some(num);
        }

        (sum, max)
    })
}

#[unsafe(no_mangle)]
fn filter_sum_max_komadori(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter()
        .copied()
        .feed_into(i32::adding().filter(|&num| num % 2 == 0).tee(Max::new()))
}
