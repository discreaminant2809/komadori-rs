use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::ParallelCollectorBase;

use super::{DefinePassDown, TeeBase, Teer};

/// A parallel collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`ParallelCollectorBase::tee()`].
/// See its documentation for more.
pub type Tee<C1, C2> = TeeBase<C1, C2, CopyTeer>;

#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct CopyTeer(());

pub(in crate::collector) fn tee<C1, C2>(collector1: C1, collector2: C2) -> Tee<C1, C2>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
{
    TeeBase::new(collector1, collector2, CopyTeer(()))
}

impl<'this, T> DefinePassDown<'this, T> for CopyTeer
where
    T: Copy,
{
    type PassDown = T;
}

impl<T> Teer<T> for CopyTeer
where
    T: Copy,
{
    const ITEM_IS_COPY: bool = true;

    #[inline]
    fn pass_down(&mut self, item: &mut T) -> T {
        *item
    }

    #[inline]
    fn no_tee_collect(&mut self, collector: &mut impl Collector<T>, item: T) -> ControlFlow<()> {
        collector.collect(item)
    }

    #[inline]
    fn no_tee_collect_many(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: &mut impl Collector<T>,
    ) -> ControlFlow<()> {
        collector.collect_many(items)
    }

    #[inline]
    fn no_tee_collect_then_finish<O>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: impl Collector<T, Output = O>,
    ) -> O {
        collector.collect_then_finish(items)
    }
}

#[cfg(test)]
mod proptests {
    use crate::test_utils::prelude::*;

    proptest! {
        /// Pre-requisite:
        /// - [`crate::vec::IntoParCollector`]
        /// - [`ParallelCollectorBase::take()`]
        #[test]
        fn indexed(
            (split_decision, nums) in propvec(any::<i32>(), ..=3)
                .prop_flat_map(|nums| {
                    (IndexedSplitStrategy::new(nums.len(), DEFAULT_MAX_DEPTH), Just(nums))
                }),
            first_count in ..=3_usize,
            second_count in ..=3_usize,
            pool in CoroutinePool::prop(),
        ) {
            indexed_impl(pool, split_decision, nums, first_count, second_count)?;
        }
    }

    proptest! {
        /// Pre-requisite:
        /// - [`crate::vec::IntoParCollector`]
        /// - [`ParallelCollectorBase::take()`]
        #[test]
        fn unindexed(
            nums in propvec(any::<i32>(), ..=3),
            split_decision in UnindexedSplitStrategy::new(DEFAULT_MAX_DEPTH),
            first_count in ..=3_usize,
            second_count in ..=3_usize,
            pool in CoroutinePool::prop(),
        ) {
            unindexed_impl(pool, split_decision, nums, first_count, second_count)?;
        }
    }

    fn indexed_impl(
        mut pool: CoroutinePool,
        split_decision: IndexedSplitDecision,
        nums: Vec<i32>,
        first_count: usize,
        second_count: usize,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || {
                vec![]
                    .into_par_collector()
                    .take(first_count)
                    .tee(vec![].into_par_collector().take(second_count))
            },
            should_break_pred: |iter| iter.count() >= first_count.max(second_count),
            pred: |mut iter, output| {
                let expected1: Vec<_> = iter.clone().take_iter().take(first_count).collect();
                let expected2: Vec<_> = iter.take_iter().take(second_count).collect();
                PredError::assert_eq(output, (expected1, expected2))
            },
        }
        .test_par_collector(&mut pool, &split_decision)
    }

    fn unindexed_impl(
        mut pool: CoroutinePool,
        split_decision: UnindexedSplitDecision,
        nums: Vec<i32>,
        first_count: usize,
        second_count: usize,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || {
                vec![]
                    .into_par_collector()
                    .take(first_count)
                    .tee(vec![].into_par_collector().take(second_count))
            },
            should_break_pred: |iter| iter.count() >= first_count.max(second_count),
            pred: |mut iter, (output1, output2)| {
                PredError::assert_fn(
                    &output1[..],
                    first_count.min(nums.len()),
                    |output, &len| output.len() == len,
                    "incorrect length",
                )?;

                PredError::assert_fn(
                    output1,
                    iter.clone().take_iter().collect::<Vec<_>>(),
                    |actual, expected| is_subsequence(actual, expected),
                    "not a subsequence",
                )?;

                PredError::assert_fn(
                    &output2[..],
                    second_count.min(nums.len()),
                    |output, &len| output.len() == len,
                    "incorrect length",
                )?;

                PredError::assert_fn(
                    output2,
                    iter.take_iter().collect::<Vec<_>>(),
                    |actual, expected| is_subsequence(actual, expected),
                    "not a subsequence",
                )
            },
        }
        .test_unindexed_par_collector(&mut pool, &split_decision)
    }
}
