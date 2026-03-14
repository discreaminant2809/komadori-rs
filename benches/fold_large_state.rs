use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::{iter::Fold, prelude::*};
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn fold_large_state(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(0..1_000))
        .take(500_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("fold_large_state");

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    unsafe {
        bench_fn!(fold);
        bench_fn!(bc);
        bench_fn!(for_loop);
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = fold_large_state
}
criterion_main!(benches);

unsafe fn fold(nums: &[usize]) -> [usize; 1000] {
    nums.iter().fold([0; 1000], |mut counts, &num| {
        unsafe { *counts.get_unchecked_mut(num) += 1 };
        counts
    })
}

unsafe fn bc(nums: &[usize]) -> [usize; 1000] {
    nums.iter().feed_into(Fold::new([0; 1000], |counts, &num| {
        unsafe { *counts.get_unchecked_mut(num) += 1 };
    }))
}

unsafe fn for_loop(nums: &[usize]) -> [usize; 1000] {
    let mut counts = [0; 1000];

    for &num in nums {
        unsafe { *counts.get_unchecked_mut(num) += 1 };
    }

    counts
}
