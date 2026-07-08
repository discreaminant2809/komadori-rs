use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::prelude::*;
use rand::{prelude::*, rngs::Xoshiro128PlusPlus};

fn max_vec(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = Xoshiro128PlusPlus::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(0..1_000))
        .take(100_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("max_vec");
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
    bench_fn!(manual_loop_1_pass_unsafe);
    bench_fn!(extend_1_pass);
    bench_fn!(manual_loop_1_pass);
    bench_fn!(manual_loop_2_pass);

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = max_vec
}
criterion_main!(benches);

#[unsafe(no_mangle)]
fn manual_loop_1_pass(nums: &[i32]) -> (Option<i32>, Vec<i32>) {
    let mut nums = nums.iter();
    let Some(&(mut max)) = nums.next() else {
        return (None, vec![]);
    };

    let mut v = Vec::with_capacity(nums.len() + 1);
    v.push(max);

    for &num in nums {
        max = max.max(num);
        v.push(num);
    }

    (Some(max), v)
}

#[unsafe(no_mangle)]
fn manual_loop_1_pass_unsafe(nums: &[i32]) -> (Option<i32>, Vec<i32>) {
    let len = nums.len();
    let mut nums = nums.iter();
    let Some(&(mut max)) = nums.next() else {
        return (None, vec![]);
    };

    let mut v: Vec<i32> = Vec::with_capacity(len);
    unsafe {
        v.as_mut_ptr().add(v.len()).write(max);
        v.set_len(v.len() + 1);
    }

    for &num in nums {
        max = max.max(num);
        unsafe {
            v.as_mut_ptr().add(v.len()).write(num);
            v.set_len(v.len() + 1);
        }
    }

    (Some(max), v)
}

#[unsafe(no_mangle)]
fn manual_loop_2_pass(nums: &[i32]) -> (Option<i32>, Vec<i32>) {
    (nums.iter().copied().max(), nums.to_vec())
}

// Hilariously that this DOES NOT get vectorized...
// #[unsafe(no_mangle)]
// fn komadori_1_pass(nums: &[i32]) -> (Option<i32>, Vec<i32>) {
//     nums.iter().copied().feed_into(Max::new().tee(vec![]))
// }

// While this does!
#[unsafe(no_mangle)]
fn komadori_1_pass(nums: &[i32]) -> (Option<i32>, Vec<i32>) {
    use komadori::iter::Fold;

    nums.iter().copied().feed_into(
        Fold::new(None::<i32>, |max, num| match max {
            Some(max) => *max = (*max).max(num),
            max => *max = Some(num),
        })
        .tee(vec![]),
    )
}

#[unsafe(no_mangle)]
fn extend_1_pass(nums: &[i32]) -> (Option<i32>, Vec<i32>) {
    #[derive(Default)]
    struct MaxI32(Option<i32>);

    impl Extend<i32> for MaxI32 {
        fn extend<T: IntoIterator<Item = i32>>(&mut self, iter: T) {
            match &mut self.0 {
                Some(max) => {
                    for num in iter {
                        *max = (*max).max(num);
                    }
                }
                None => self.0 = iter.into_iter().max(),
            }
        }
    }

    let (MaxI32(max), v): (_, Vec<_>) = nums.iter().map(|&num| (num, num)).unzip();
    (max, v)
}
