use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::prelude::*;
use rand::{prelude::*, rngs::Xoshiro128PlusPlus};

fn sum_doubles(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = Xoshiro128PlusPlus::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(-10_000..=10_000))
        .take(100_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("sum_doubles");
    let expected = manual_loop_1_pass(&nums);

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(komadori_1_pass);
    bench_fn!(map_1_pass);
    bench_fn!(inspect_1_pass);
    bench_fn!(manual_loop_1_pass);
    bench_fn!(extend_1_pass);
    bench_fn!(manual_loop_2_pass);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = sum_doubles
}
criterion_main!(benches);

#[unsafe(no_mangle)]
fn manual_loop_1_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    let mut sum = 0;
    let mut doubles: Vec<i32> = Vec::with_capacity(nums.len());

    for &num in nums {
        sum += num;
        doubles.push(num * 2);
    }

    (sum, doubles)
}

#[unsafe(no_mangle)]
fn manual_loop_2_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    (nums.iter().sum(), nums.iter().map(|&num| num * 2).collect())
}

#[unsafe(no_mangle)]
fn inspect_1_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    let mut sum = 0;
    let doubles = nums
        .iter()
        .inspect(|&&num| sum += num)
        .map(|&num| num * 2)
        .collect();

    (sum, doubles)
}

#[unsafe(no_mangle)]
fn map_1_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    let mut sum = 0;
    let doubles = nums
        .iter()
        .map(|&num| {
            sum += num;
            num * 2
        })
        .collect();

    (sum, doubles)
}

#[unsafe(no_mangle)]
fn komadori_1_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    nums.iter()
        .copied()
        .feed_into((0.into_sum(), vec![].into_collector().map(|num| num * 2)))
}

#[unsafe(no_mangle)]
fn extend_1_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    #[derive(Default)]
    struct SumI32(i32);

    impl Extend<i32> for SumI32 {
        fn extend<T: IntoIterator<Item = i32>>(&mut self, iter: T) {
            self.0 += iter.into_iter().sum::<i32>()
        }
    }

    let (SumI32(sum), doubles): (_, Vec<_>) = nums.iter().map(|&num| (num, num * 2)).unzip();
    (sum, doubles)
}
