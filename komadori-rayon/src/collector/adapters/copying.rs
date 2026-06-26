use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that copies every collected item.
///
/// This `struct` is created by [`ParallelCollectorBase::copying()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct Copying<C> {
    collector: C,
}

impl<C> Copying<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self { collector }
    }
}

impl<'a, C> DefineSerial<'a> for Copying<C>
where
    C: DefineSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<C::Serial>>;
}

impl<'a, C> DefineUnindexedSerial<'a> for Copying<C>
where
    C: DefineUnindexedSerial<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<C::UnindexedSerial>>;
}

impl<C> ParallelCollectorBase for Copying<C>
where
    C: ParallelCollectorBase,
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

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        let (len, consumer, commit) = self.collector.parts(len);
        unique::uniquify((len, consumer::Consumer::new(consumer), commit))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        let (len, consumer, commit) = self.collector.take_parts(len);
        unique::take_uniquify((len, consumer::Consumer::new(consumer), commit))
    }
}

impl<C> UnindexedParallelCollectorBase for Copying<C>
where
    C: UnindexedParallelCollectorBase,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        unique_unindexed::uniquify((consumer::Consumer::new(consumer), commit))
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique_unindexed::take_uniquify((consumer::Consumer::new(consumer), commit))
    }
}

mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing;

    pub struct Consumer<C> {
        consumer: C,
    }

    pub type Serial<C> = komadori::collector::Copying<C>;

    impl<C> Consumer<C> {
        #[inline]
        pub fn new(consumer: C) -> Self {
            Self { consumer }
        }
    }

    impl<C> IntoCollectorBase for Consumer<C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            self.consumer.into_collector().copying()
        }
    }

    impl<C> plumbing::Consumer for Consumer<C>
    where
        C: plumbing::Consumer,
    {
        type Combiner = C::Combiner;

        #[inline]
        fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
            let (consumer, combiner) = self.consumer.split_off_left_at(index);
            (Self { consumer }, combiner)
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.consumer.break_hint()
        }
    }

    impl<C> plumbing::UnindexedConsumer for Consumer<C>
    where
        C: plumbing::UnindexedConsumer,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
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
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter(),
            collector_factory: || vec![].into_par_collector().take(take_count).copying(),
            should_break_pred: |_| nums.len() >= take_count,
            pred: |_, output| PredError::assert_eq(output, nums.iter().copied().take(take_count).collect()),
        }
        .test_par_collector(&mut pool, &split_decision)
    }

    fn unindexed_impl(
        mut pool: CoroutinePool,
        split_decision: UnindexedSplitDecision,
        nums: Vec<i32>,
        take_count: usize,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter(),
            collector_factory: || vec![].into_par_collector().take(take_count).copying(),
            should_break_pred: |_| nums.len() >= take_count,
            pred: |_, output| {
                PredError::assert_eq(output.len(), nums.len().min(take_count))?;

                PredError::assert_fn(
                    output,
                    &nums,
                    |actual, expected| is_subsequence(&actual[..], &expected[..]),
                    "not a subsequence",
                )
            },
        }
        .test_unindexed_par_collector(&mut pool, &split_decision)
    }
}
