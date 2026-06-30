use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::prelude::*;
use komadori_rayon::prelude::*;
use rand::{prelude::*, rngs::Xoshiro128PlusPlus};
use rayon::prelude::*;

fn filter_vec_sum(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = Xoshiro128PlusPlus::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(-10_000..10_000))
        .take(100_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);
    let expected = serial_direct(&nums);

    let mut group = criterion.benchmark_group("filter_vec_sum");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(serial_direct);
    bench_fn!(serial_from_parallel);
    bench_fn!(komadori_basic);
    bench_fn!(komadori_fii);
    bench_fn!(rayon_extend);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = filter_vec_sum
}
criterion_main!(benches);

#[unsafe(no_mangle)]
fn serial_direct(nums: &[i32]) -> (Vec<i32>, i32) {
    nums.iter()
        .copied()
        .feed_into(vec![].into_collector().filter(filter_pred).tee(0.into_sum()))
}

#[unsafe(no_mangle)]
fn serial_from_parallel(nums: &[i32]) -> (Vec<i32>, i32) {
    nums.iter().copied().feed_into(
        // Use a parallel collector as a (serial) collector!
        vec![]
            .into_par_collector()
            .filter(filter_pred)
            .tee(0.into_par_sum())
            .into_collector(),
    )
}

fn rayon_extend(nums: &[i32]) -> (Vec<i32>, i32) {
    #[derive(Default)]
    struct VecI32Filter(Vec<i32>);

    impl ParallelExtend<i32> for VecI32Filter {
        fn par_extend<I>(&mut self, par_iter: I)
        where
            I: IntoParallelIterator<Item = i32>,
        {
            self.0.par_extend(par_iter.into_par_iter().filter(filter_pred));
        }
    }

    #[derive(Default)]
    struct SumI32(i32);

    impl ParallelExtend<i32> for SumI32 {
        fn par_extend<I>(&mut self, par_iter: I)
        where
            I: IntoParallelIterator<Item = i32>,
        {
            self.0 += par_iter.into_par_iter().sum::<i32>()
        }
    }

    let (VecI32Filter(nums), SumI32(sum)) = nums.par_iter().map(|&num| (num, num)).unzip();
    (nums, sum)
}

fn komadori_basic(nums: &[i32]) -> (Vec<i32>, i32) {
    nums.par_iter().copied().feed_into(
        vec![]
            .into_par_collector()
            .filter(filter_pred)
            .tee(0.into_par_sum()),
    )
}

fn komadori_fii(nums: &[i32]) -> (Vec<i32>, i32) {
    nums.par_iter().copied().feed_into_indexed(
        vec![]
            .into_par_collector()
            .filter(filter_pred)
            .tee(0.into_par_sum()),
    )
}

#[inline]
fn filter_pred(&num: &i32) -> bool {
    num >= 0
}
