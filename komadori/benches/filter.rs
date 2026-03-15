use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori::prelude::*;
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn filter(criterion: &mut Criterion) {
    let seed = 0;
    let mut rng = StdRng::seed_from_u64(seed);

    let nums: Box<_> = std::iter::repeat_with(|| rng.random_range(0..1_000))
        .take(100_000)
        .collect();

    println!("Seed: {seed}");
    println!("First 10 elements: {:?}", &nums[..10]);

    let mut group = criterion.benchmark_group("filter");
    let expected = filter_iter(&nums);

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(filter_iter);
    bench_fn!(filter_komadori);
    bench_fn!(filter_komadori_each);
    // bench_fn!(filter_komadori_new);

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

fn filter_fn(&&num: &&i32) -> bool {
    num % 3 == 0 || num % 5 == 0
}

#[unsafe(no_mangle)]
fn filter_iter(nums: &[i32]) -> i32 {
    nums.iter().filter(filter_fn).sum()
}

#[unsafe(no_mangle)]
fn filter_komadori(nums: &[i32]) -> i32 {
    nums.iter().feed_into(i32::adding().filter(filter_fn))
}

#[unsafe(no_mangle)]
fn filter_komadori_each(nums: &[i32]) -> i32 {
    let mut collector = i32::adding().filter(filter_fn);
    for num in nums {
        let _ = collector.collect(num);
    }

    collector.finish()
}

// #[unsafe(no_mangle)]
// fn filter_komadori_new(nums: &[i32]) -> i32 {
//     nums.iter()
//         .feed_into(custom::Filter::new(i32::adding(), filter_fn))
// }

// mod custom {
//     use std::ops::ControlFlow;

//     use komadori::prelude::*;

//     pub struct Filter<C, F> {
//         collector: C,
//         pred: F,
//     }

//     impl<C, F> Filter<C, F> {
//         pub(super) fn new<T>(collector: C, pred: F) -> Self
//         where
//             C: Collector<T>,
//             F: FnMut(&T) -> bool,
//         {
//             Self { collector, pred }
//         }
//     }

//     impl<C, F> CollectorBase for Filter<C, F>
//     where
//         C: CollectorBase,
//     {
//         type Output = C::Output;

//         #[inline]
//         fn finish(self) -> Self::Output {
//             self.collector.finish()
//         }

//         #[inline]
//         fn break_hint(&self) -> ControlFlow<()> {
//             self.collector.break_hint()
//         }
//     }

//     impl<C, F, T> Collector<T> for Filter<C, F>
//     where
//         C: Collector<T>,
//         F: FnMut(&T) -> bool,
//     {
//         #[inline]
//         fn collect(&mut self, item: T) -> ControlFlow<()> {
//             if (self.pred)(&item) {
//                 self.collector.collect(item)
//             } else {
//                 self.collector.break_hint()
//             }
//         }

//         // None of those default methods. We wanna test pure collect() method
//     }
// }
