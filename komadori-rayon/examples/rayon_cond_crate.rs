use komadori::prelude::*;
use komadori_rayon::prelude::*;
use rayon::prelude::*;
use rayon_cond::CondIterator;

fn main() {
    let nums = [1, 2, 3, 4, 5];
    println!("Sum of {nums:?}: {}", sum(&nums));

    let nums = [1; 1_000_000];
    println!("Sum of one millions of 1s: {}", sum(&nums));
}

fn sum(nums: &[i32]) -> i32 {
    const PAR_THRESHOLD: usize = 100_000;
    bridge_unindexed(
        CondIterator::new(nums, nums.len() >= PAR_THRESHOLD),
        0.into_par_sum(),
    )
}

fn bridge_unindexed<T, O>(
    iter: CondIterator<impl ParallelIterator<Item = T>, impl Iterator<Item = T>>,
    collector: impl IntoUnindexedParallelCollector<T, Output = O>,
) -> O {
    match iter {
        CondIterator::Parallel(iter) => iter.feed_into(collector),
        CondIterator::Serial(iter)
            if let Some(len) = {
                let (lower, upper) = iter.size_hint();
                upper.filter(|&upper| upper == lower)
            } =>
        {
            let mut collector = collector.into_par_collector();
            let (consumer, commit) = collector.take_parts(len);
            let output = iter.feed_into(consumer);
            commit(output);
            collector.finish()
        }
        CondIterator::Serial(iter) => {
            let mut collector = collector.into_par_collector();
            let (consumer, commit) = collector.take_unindexed_parts();
            let output = iter.feed_into(consumer);
            commit(output);
            collector.finish()
        }
    }
}
