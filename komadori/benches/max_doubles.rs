use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{cmp::Max, prelude::*};
use rand::{prelude::*, rngs::Xoshiro128PlusPlus};

fn max_doubles(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = Xoshiro128PlusPlus::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(-1_000_000_000..=1_000_000_000))
        .take(100_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("max_doubles");
    let expected = manual_loop_1_pass(&nums);

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(manual_loop_1_pass);
    bench_fn!(manual_loop_2_pass);
    bench_fn!(komadori_1_pass);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = max_doubles
}
criterion_main!(benches);

#[unsafe(no_mangle)]
fn manual_loop_1_pass(nums: &[i32]) -> (Option<i32>, Vec<i32>) {
    let len = nums.len();
    let mut nums = nums.iter();
    let Some(&(mut max)) = nums.next() else {
        return (None, vec![]);
    };

    let mut doubles: Vec<i32> = Vec::with_capacity(len);
    doubles.push(max * 2);

    for &num in nums {
        max = max.max(num);
        doubles.push(num * 2);
    }

    (Some(max), doubles)
}

#[unsafe(no_mangle)]
fn manual_loop_2_pass(nums: &[i32]) -> (Option<i32>, Vec<i32>) {
    (
        nums.iter().copied().max(),
        nums.iter().map(|&num| num * 2).collect(),
    )
}

#[unsafe(no_mangle)]
fn komadori_1_pass(nums: &[i32]) -> (Option<i32>, Vec<i32>) {
    nums.iter()
        .copied()
        .feed_into(Max::new().tee(vec![].into_collector().map(|num| num * 2)))
}
