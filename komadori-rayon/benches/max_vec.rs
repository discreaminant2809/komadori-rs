use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{cmp::Max, prelude::*};
use komadori_rayon::{cmp::ParMax, prelude::*};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use rayon::prelude::*;

fn reduce(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random::<i32>())
        .take(1_000_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);
    let expected = seq_one_pass(&nums);

    let mut group = criterion.benchmark_group("max_vec");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(rayon_komadori);
    bench_fn!(rayon_komadori_indexed);
    bench_fn!(rayon_extend);
    bench_fn!(rayon_two_pass);
    bench_fn!(rayon_fold_reduce);
    bench_fn!(rayon_atomic);
    bench_fn!(seq_one_pass);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = reduce
}
criterion_main!(benches);

fn seq_one_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    nums.iter()
        .copied()
        .feed_into(Max::new().map_output(Option::unwrap).tee(vec![]))
}

fn rayon_two_pass(nums: &[i32]) -> (i32, Vec<i32>) {
    let max = nums.par_iter().copied().max().unwrap();
    let v = nums.par_iter().copied().collect();
    (max, v)
}

fn rayon_atomic(nums: &[i32]) -> (i32, Vec<i32>) {
    use std::sync::atomic::{AtomicI32, Ordering};

    let max = AtomicI32::new(i32::MIN);
    let v = nums
        .par_iter()
        .copied()
        .inspect(|&num| {
            max.fetch_max(num, Ordering::Relaxed);
        })
        .collect();

    (max.into_inner(), v)
}

fn rayon_fold_reduce(nums: &[i32]) -> (i32, Vec<i32>) {
    #[inline]
    fn id() -> (i32, Vec<i32>) {
        (i32::MIN, vec![])
    }

    nums.par_iter()
        .fold(id, |(max, mut v), &num| {
            v.push(num);
            (max.max(num), { v })
        })
        .reduce(id, |(max1, mut v1), (max2, mut v2)| {
            v1.append(&mut v2);
            (max1.max(max2), v1)
        })
}

fn rayon_extend(nums: &[i32]) -> (i32, Vec<i32>) {
    #[derive(Default)]
    struct MaxExtendI32 {
        max: Option<i32>,
    }

    impl ParallelExtend<i32> for MaxExtendI32 {
        fn par_extend<I>(&mut self, par_iter: I)
        where
            I: IntoParallelIterator<Item = i32>,
        {
            self.max = self.max.max(par_iter.into_par_iter().max());
        }
    }

    let (max, v): (MaxExtendI32, Vec<_>) = nums.par_iter().map(|&num| (num, num)).unzip();
    (max.max.unwrap(), v)
}

fn rayon_komadori(nums: &[i32]) -> (i32, Vec<i32>) {
    let (max, v) = nums
        .par_iter()
        .copied()
        // FIXED: use `map_output()` when it's implemented
        .feed_into(ParMax::new().tee(vec![]));

    (max.unwrap(), v)
}

fn rayon_komadori_indexed(nums: &[i32]) -> (i32, Vec<i32>) {
    let (max, v) = nums
        .par_iter()
        .copied()
        // FIXED: use `map_output()` when it's implemented
        .feed_into_indexed(ParMax::new().tee(vec![]));

    (max.unwrap(), v)
}
