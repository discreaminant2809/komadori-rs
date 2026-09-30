use std::{
    hint::black_box,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use criterion::{Criterion, criterion_group, criterion_main};
use komadori_rayon::{iter::ParReduce, ops::ParOr, prelude::*};
use rayon::prelude::*;

fn par_or(criterion: &mut Criterion) {
    let nums = vec![false; 1_000_000];

    let mut group = criterion.benchmark_group("par_or");
    let expected = false;

    macro_rules! bench_fn {
        ($fn_name:ident) => {
            group.bench_function(stringify!($fn_name), |bencher| {
                assert_eq!($fn_name(&nums), expected);
                bencher.iter(|| $fn_name(black_box(&nums)));
            });
        };
    }

    bench_fn!(current_implementation);
    bench_fn!(using_load_store);
    bench_fn!(using_fetch); // This approach is hundreds times slower!

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30))
        .sample_size(300);
    targets = par_or
}
criterion_main!(benches);

#[unsafe(no_mangle)]
fn using_load_store(bools: &[bool]) -> bool {
    bools.par_iter().copied().feed_into(
        ParReduce::new(|b1, b2| *b1 |= b2)
            .map_output(Option::unwrap_or_default)
            .map(|option: Option<()>| option.is_none())
            .try_fold_local(AtomicBool::new(false), |flag| {
                (!flag.load(Ordering::Relaxed)).then_some((
                    (),
                    |flag: &AtomicBool, _: &mut (), b| {
                        if flag.load(Ordering::Relaxed) {
                            None
                        } else if b {
                            flag.store(true, Ordering::Relaxed);
                            None
                        } else {
                            Some(())
                        }
                    },
                ))
            }),
    )
}

#[unsafe(no_mangle)]
fn using_fetch(bools: &[bool]) -> bool {
    bools.par_iter().copied().feed_into(
        ParReduce::new(|b1, b2| *b1 |= b2)
            .map_output(Option::unwrap_or_default)
            .map(|option: Option<()>| option.is_none())
            .try_fold_local(AtomicBool::new(false), |flag| {
                (!flag.load(Ordering::Relaxed)).then_some((
                    (),
                    |flag: &AtomicBool, _: &mut (), b| {
                        if flag.fetch_or(b, Ordering::Relaxed) || b {
                            None
                        } else {
                            Some(())
                        }
                    },
                ))
            }),
    )
}

#[unsafe(no_mangle)]
fn current_implementation(bools: &[bool]) -> bool {
    bools.par_iter().copied().feed_into(ParOr::new())
}
