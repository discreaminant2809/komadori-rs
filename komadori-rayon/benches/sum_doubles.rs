use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::prelude::*;
use komadori_rayon::prelude::*;
use rand::{prelude::*, rngs::Xoshiro128PlusPlus};
use rayon::prelude::*;

fn sum_doubles(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = Xoshiro128PlusPlus::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(-10_000..=10_000))
        .take(1_000_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("sum_doubles");
    let expected = serial_1_pass(&nums);

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(serial_1_pass);
    bench_fn!(rayon_2_pass);
    bench_fn!(rayon_2_pass_join);
    bench_fn!(rayon_atomic);
    bench_fn!(rayon_fold_reduce);
    bench_fn!(rayon_extend);
    bench_fn!(rayon_komadori);
    bench_fn!(rayon_komadori_indexed);

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
fn serial_1_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    nums.iter()
        .copied()
        .feed_into((0.into_sum(), vec![].into_collector().map(|num| num * 2)))
}

#[unsafe(no_mangle)]
fn rayon_2_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    let sum = nums.par_iter().sum();
    let doubles = nums.par_iter().map(|&num| num * 2).collect();
    (sum, doubles)
}

#[unsafe(no_mangle)]
fn rayon_2_pass_join(nums: &[i32]) -> (i32, Vec<i32>) {
    rayon::join(
        || nums.par_iter().sum(),
        || nums.par_iter().map(|&num| num * 2).collect(),
    )
}

#[unsafe(no_mangle)]
fn rayon_atomic(nums: &[i32]) -> (i32, Vec<i32>) {
    use std::sync::atomic::{AtomicI32, Ordering};

    let sum = AtomicI32::new(0);
    let v = nums
        .par_iter()
        .map(|&num| {
            sum.fetch_add(num, Ordering::Relaxed);
            num * 2
        })
        .collect();

    (sum.into_inner(), v)
}

#[unsafe(no_mangle)]
fn rayon_fold_reduce(nums: &[i32]) -> (i32, Vec<i32>) {
    #[inline]
    fn id() -> (i32, Vec<i32>) {
        (0, vec![])
    }

    nums.par_iter()
        .fold(id, |(sum, mut v), &num| {
            v.push(num * 2);
            (sum + num, v)
        })
        .reduce(id, |(sum1, mut v1), (sum2, mut v2)| {
            v1.append(&mut v2);
            (sum1 + sum2, v1)
        })
}

#[unsafe(no_mangle)]
fn rayon_extend(nums: &[i32]) -> (i32, Vec<i32>) {
    #[derive(Default)]
    struct SumExtendI32 {
        sum: i32,
    }

    impl ParallelExtend<i32> for SumExtendI32 {
        fn par_extend<I>(&mut self, par_iter: I)
        where
            I: IntoParallelIterator<Item = i32>,
        {
            self.sum += par_iter.into_par_iter().sum::<i32>();
        }
    }

    let (sum, v): (SumExtendI32, Vec<_>) = nums.par_iter().map(|&num| (num, num * 2)).unzip();
    (sum.sum, v)
}

#[unsafe(no_mangle)]
fn rayon_komadori(nums: &[i32]) -> (i32, Vec<i32>) {
    nums.par_iter().copied().feed_into((
        0.into_par_sum(),
        vec![].into_par_collector().map(|num| num * 2),
    ))
}

#[unsafe(no_mangle)]
fn rayon_komadori_indexed(nums: &[i32]) -> (i32, Vec<i32>) {
    nums.par_iter().copied().feed_into_indexed((
        0.into_par_sum(),
        vec![].into_par_collector().map(|num| num * 2),
    ))
}
