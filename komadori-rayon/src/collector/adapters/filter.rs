use std::{fmt::Debug, ops::ControlFlow};

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{DefineSerial, DefineUnindexedSerial},
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that uses a closure to determine whether
/// an item should be accumulated.
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::filter()`].
/// See its documentation for more.
#[derive(Clone)]
pub struct Filter<C, P> {
    collector: C,
    pred: P,
}

impl<C, P> Filter<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self { collector, pred }
    }
}

impl<C, P> Debug for Filter<C, P>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filter")
            .field("collector", &self.collector)
            .field("pred", &std::any::type_name::<P>())
            .finish()
    }
}

impl<'this, C, P> DefineSerial<'this> for Filter<C, P>
where
    C: DefineUnindexedSerial<'this>,
    P: Sync,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<C::UnindexedSerial, &'this P>>;
}

impl<'this, C, P> DefineUnindexedSerial<'this> for Filter<C, P>
where
    C: DefineUnindexedSerial<'this>,
    P: Sync,
{
    type UnindexedSerial =
        unique_unindexed::Serial<'this, Self, consumer::Serial<C::UnindexedSerial, &'this P>>;
}

impl<C, P> ParallelCollectorBase for Filter<C, P>
where
    C: UnindexedParallelCollectorBase,
    P: Sync,
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

    #[inline]
    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl crate::collector::plumbing::Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        unique::uniquify((len, consumer::Consumer::new(consumer, &self.pred), commit))
    }

    #[inline]
    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl crate::collector::plumbing::Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique::take_uniquify((len, consumer::Consumer::new(consumer, &self.pred), commit))
    }
}

impl<C, P> UnindexedParallelCollectorBase for Filter<C, P>
where
    C: UnindexedParallelCollectorBase,
    P: Sync,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl crate::collector::plumbing::UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        unique_unindexed::uniquify((consumer::Consumer::new(consumer, &self.pred), commit))
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl crate::collector::plumbing::UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique_unindexed::take_uniquify((consumer::Consumer::new(consumer, &self.pred), commit))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer<C, P> {
        consumer: C,
        pred: P,
    }

    // Can't utilize from komadori's filter(), since it requires item type right away.
    pub struct Serial<C, P> {
        collector: C,
        pred: P,
    }

    impl<C, P> Consumer<C, P> {
        #[inline]
        pub(super) fn new(consumer: C, pred: P) -> Self {
            Self { consumer, pred }
        }
    }

    impl<C, P> IntoCollectorBase for Consumer<C, P>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector, P>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector: self.consumer.into_collector(),
                pred: self.pred,
            }
        }
    }

    impl<C, P> plumbing::Consumer for Consumer<C, P>
    where
        C: plumbing::UnindexedConsumer,
        P: Clone + Send,
    {
        type Combiner = C::Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.consumer.break_hint()
        }
    }

    impl<C, P> CollectorBase for Serial<C, P>
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

    impl<C, P> plumbing::UnindexedConsumer for Consumer<C, P>
    where
        C: plumbing::UnindexedConsumer,
        P: Clone + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                pred: self.pred.clone(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
    }

    impl<C, P, T> Collector<T> for Serial<C, P>
    where
        C: Collector<T>,
        P: FnMut(&T) -> bool,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            if (self.pred)(&item) {
                self.collector.collect(item)
            } else {
                self.collector.break_hint()
            }
        }

        // Removed the overriden implementations cuz the items here are being consumed
        // without consulting the underlying collector's break hint during filtering.
        // Yes, the performance degrades, but it's because of `try_for_each()` and/or
        // LLVM noise (which could be fixed soon),
        // and in multiple reduction it still works well and performs similarly to fold().
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
            (split_decision, nums) in propvec(any::<i32>(), ..=5)
                .prop_flat_map(|nums| {
                    (IndexedSplitStrategy::new(nums.len(), DEFAULT_MAX_DEPTH), Just(nums))
                }),
            take_count in ..=5_usize,
            pool in CoroutinePool::prop(),
        ) {
            indexed_impl(pool, split_decision, nums, take_count)?;
        }
    }

    proptest! {
        /// Pre-requisite:
        /// - [`crate::vec::IntoParCollector`]
        /// - [`ParallelCollectorBase::take()`]
        #[test]
        fn unindexed(
            nums in propvec(any::<i32>(), ..=5),
            split_decision in UnindexedSplitStrategy::new(DEFAULT_MAX_DEPTH),
            take_count in ..=5_usize,
            pool in CoroutinePool::prop(),
        ) {
            unindexed_impl(pool, split_decision, nums, take_count)?;
        }
    }

    fn indexed_impl(
        mut pool: CoroutinePool,
        split_decision: IndexedSplitDecision,
        nums: Vec<i32>,
        take_count: usize,
    ) -> TestCaseResult {
        par_collector_tester(&nums, take_count).test_par_collector(&mut pool, &split_decision)
    }

    fn unindexed_impl(
        mut pool: CoroutinePool,
        split_decision: UnindexedSplitDecision,
        nums: Vec<i32>,
        take_count: usize,
    ) -> TestCaseResult {
        par_collector_tester(&nums, take_count).test_unindexed_par_collector(&mut pool, &split_decision)
    }

    // Grouped into one method because
    // both the indexed and unindexed paths are the same anyway.
    fn par_collector_tester(
        nums: &[i32],
        take_count: usize,
    ) -> impl ParallelCollectorTester + UnindexedParallelCollectorTester {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: move || {
                vec![]
                    .into_par_collector()
                    .take(take_count)
                    .filter(|&num| num >= 0)
            },
            should_break_pred: move |mut iter| iter.take_iter().filter(|&num| num >= 0).count() >= take_count,
            pred: move |mut iter, output| {
                // Properties:
                // - At most `take_count` items.
                // - All items must satisfy the predicate.
                // - Subsequence-ness.

                PredError::assert_fn(
                    &output[..],
                    // We could also add `.min(nums.len())`,
                    // but `take()` has alr been tested this possibility.
                    take_count,
                    |output, &take_count| output.len() <= take_count,
                    "excessive amount of items",
                )?;

                if !output.iter().all(|&num| num >= 0) {
                    return Err(PredError::IncorrectOutput(format!(
                        "{output:?} contains an item dissatisfying the predicate"
                    )));
                }

                PredError::assert_fn(
                    output,
                    iter.take_iter().collect::<Vec<_>>(),
                    |actual, expected| is_subsequence(actual, expected),
                    "not a subsequence",
                )
            },
        }
    }
}
