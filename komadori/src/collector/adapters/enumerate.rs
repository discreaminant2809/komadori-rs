use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// Creates a collector that feeds the underlying collector with the current count
/// alongside with the item.
///
/// This `struct` is created by [`CollectorBase::enumerate()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Enumerate<C> {
    collector: C,
    idx: usize,
}

impl<C> Enumerate<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self { collector, idx: 0 }
    }
}

impl<C> CollectorBase for Enumerate<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        self.collector.break_hint()
    }
}

impl<C, T> Collector<T> for Enumerate<C>
where
    C: Collector<(usize, T)>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        let idx = self.idx;
        self.idx += 1;
        self.collector.collect((idx, item))
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collector.collect_many(
            // Be careful! We have to `zip(items, indices)`, not `zip(indices, items)`.
            // the iterator will pull out one index prematurely even tho `items` are exhausted,
            // skipping one index for the next call of collect-related method!
            items
                .into_iter()
                .zip(std::iter::repeat_with(|| {
                    let idx = self.idx;
                    self.idx += 1;
                    idx
                }))
                .map(|(item, idx)| (idx, item)),
        )
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        // This is fine, unlike `collect_many()`.
        // We get rid of the collector anyway!
        self.collector.collect_then_finish((self.idx..).zip(items))
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {

    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    // Precondition:
    // - `Vec::IntoCollector`
    // - `CollectorBase::take()`
    proptest! {
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=4),
            (take_count, starting_nums) in (0_usize..=5).prop_flat_map(|take_count| {
                let starting_nums = if take_count == 0 {
                    Just(vec![]).boxed()
                } else {
                    propvec(any::<i32>(), 0..take_count).boxed()
                };

                (Just(take_count), starting_nums)
            }),
        ) {
            all_collect_methods_impl(nums, starting_nums, take_count)?;
        }
    }

    fn all_collect_methods_impl(
        nums: Vec<i32>,
        starting_nums: Vec<i32>,
        take_count: usize,
    ) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                let mut collector = vec![].into_collector().take(take_count).enumerate();
                // If the number of starting nums is 0, it may be that the `take_count` is 0.
                // We shouldn't collect in this case.
                if !starting_nums.is_empty() {
                    let _ = collector.collect_many(starting_nums.iter().copied());
                }
                collector
            },
            should_break_pred: |_| nums.len() + starting_nums.len() >= take_count,
            pred: |mut iter, output, remaining| {
                let expected: Vec<_> = starting_nums
                    .iter()
                    .copied()
                    .chain(&mut iter)
                    .enumerate()
                    .take(take_count)
                    .collect();

                if expected != output {
                    Err(PredError::IncorrectOutput)
                } else if iter.ne(remaining) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
