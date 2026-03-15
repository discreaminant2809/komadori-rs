use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{iter::Reduce, prelude::*};
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn reduce(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    let nums: Box<[Vector1K]> =
        std::iter::repeat_with(|| std::array::from_fn(|_| rng.random_range(0..1_000)))
            .take(100)
            .collect();

    println!("Seed: {seed}");
    println!("First 2 elements: {:?}", &nums[..2]);
    let expected = reduce_for_loop(&nums);

    let mut group = criterion.benchmark_group("reduce");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(reduce_iter);
    bench_fn!(reduce_komadori);
    bench_fn!(reduce_for_loop);

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

fn reduce_iter(nums: &[Vector1K]) -> Option<Vector1K> {
    nums.iter().copied().reduce(|mut a, b| {
        add_assign_vector_1k(&mut a, b);
        a
    })
}

fn reduce_komadori(nums: &[Vector1K]) -> Option<Vector1K> {
    nums.iter()
        .copied()
        .feed_into(Reduce::new(add_assign_vector_1k))
}

fn reduce_for_loop(nums: &[Vector1K]) -> Option<Vector1K> {
    let mut nums = nums.iter().copied();
    let mut first = nums.next()?;

    for num in nums {
        add_assign_vector_1k(&mut first, num);
    }

    Some(first)
}

type Vector1K = [i32; 1000];

fn add_assign_vector_1k(a: &mut Vector1K, b: Vector1K) {
    for (num_a, num_b) in a.iter_mut().zip(b) {
        *num_a += num_b;
    }
}
