use std::{
    fmt::Debug,
    ops::ControlFlow,
    sync::atomic::{AtomicBool, Ordering},
};

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that accumulates items until it encounters
/// an items that makess a given predicate `false` at *any* time.
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::take_any_while()`].
/// See its documentation for more.
pub struct TakeAnyWhile<C, P> {
    collector: C,
    take_pred: TakePred<P>,
}

struct TakePred<P> {
    pred: P,
    stopped: AtomicBool,
}

impl<C, P> TakeAnyWhile<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self {
            collector,
            take_pred: TakePred {
                pred,
                stopped: AtomicBool::new(false),
            },
        }
    }
}

impl<C, P> Debug for TakeAnyWhile<C, P>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TakeAnyWhile")
            .field("collector", &self.collector)
            .field("pred", &std::any::type_name::<P>())
            .field("stopped", &self.take_pred.stopped.load(Ordering::Relaxed))
            .finish()
    }
}

// `AtomicBool` doesn't implement `Clone`, so we can't derive.
impl<C, P> Clone for TakeAnyWhile<C, P>
where
    C: Clone,
    P: Clone,
{
    fn clone(&self) -> Self {
        Self {
            collector: self.collector.clone(),
            take_pred: TakePred {
                pred: self.take_pred.pred.clone(),
                stopped: AtomicBool::new(self.take_pred.stopped.load(Ordering::Relaxed)),
            },
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.collector.clone_from(&source.collector);
        self.take_pred.pred.clone_from(&source.take_pred.pred);
        self.take_pred.stopped.store(
            source.take_pred.stopped.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
    }
}

impl<P> TakePred<P> {
    fn should_take<T>(&self, item: &T) -> bool
    where
        P: Fn(&T) -> bool,
    {
        if self.stopped.load(Ordering::Relaxed) {
            false
        } else if (self.pred)(item) {
            true
        } else {
            self.stopped.store(true, Ordering::Relaxed);
            false
        }
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        if self.stopped.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

impl<'this, C, P> DefineSerial<'this> for TakeAnyWhile<C, P>
where
    C: DefineUnindexedSerial<'this>,
    P: Sync,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<'this, C::UnindexedSerial, P>>;
}

impl<'this, C, P> DefineUnindexedSerial<'this> for TakeAnyWhile<C, P>
where
    C: DefineUnindexedSerial<'this>,
    P: Sync,
{
    type UnindexedSerial =
        unique_unindexed::Serial<'this, Self, consumer::Serial<'this, C::UnindexedSerial, P>>;
}

impl<C, P> ParallelCollectorBase for TakeAnyWhile<C, P>
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
        self.take_pred.break_hint()?;
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
        let (consumer, commit) = self.collector.parts_unindexed();
        let take_pred = &self.take_pred;
        unique::uniquify(
            (len, consumer::Consumer::new(consumer, take_pred), move |output| {
                commit(output)?;
                take_pred.break_hint()
            }),
        )
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
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique::take_uniquify((len, consumer::Consumer::new(consumer, &self.take_pred), commit))
    }
}

impl<C, P> UnindexedParallelCollectorBase for TakeAnyWhile<C, P>
where
    C: UnindexedParallelCollectorBase,
    P: Sync,
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
        let take_pred = &self.take_pred;
        unique_unindexed::uniquify((consumer::Consumer::new(consumer, take_pred), move |output| {
            commit(output)?;
            take_pred.break_hint()
        }))
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
        unique_unindexed::take_uniquify((consumer::Consumer::new(consumer, &self.take_pred), commit))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    use super::TakePred;

    pub struct Consumer<'a, C, P> {
        consumer: C,
        take_pred: &'a TakePred<P>,
    }

    pub struct Serial<'a, C, P> {
        collector: C,
        take_pred: &'a TakePred<P>,
    }

    impl<'a, C, P> Consumer<'a, C, P> {
        #[inline]
        pub(super) fn new(consumer: C, take_pred: &'a TakePred<P>) -> Self {
            Self { consumer, take_pred }
        }
    }

    impl<'a, C, P> IntoCollectorBase for Consumer<'a, C, P>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<'a, C::IntoCollector, P>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector: self.consumer.into_collector(),
                take_pred: self.take_pred,
            }
        }
    }

    impl<C, P> plumbing::Consumer for Consumer<'_, C, P>
    where
        C: UnindexedConsumer,
        P: Sync,
    {
        type Combiner = C::Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.take_pred.break_hint()?;
            self.consumer.break_hint()
        }
    }

    impl<C, P> UnindexedConsumer for Consumer<'_, C, P>
    where
        C: UnindexedConsumer,
        P: Sync,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                take_pred: self.take_pred,
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
    }

    impl<'a, C, P> CollectorBase for Serial<'a, C, P>
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
            self.take_pred.break_hint()?;
            self.collector.break_hint()
        }
    }

    impl<C, P, T> Collector<T> for Serial<'_, C, P>
    where
        C: Collector<T>,
        P: Fn(&T) -> bool,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            if self.take_pred.should_take(&item) {
                self.collector.collect(item)
            } else {
                self.collector.break_hint()
            }
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            let cf = self.collector.collect_many(
                items
                    .into_iter()
                    .take_while(|item| self.take_pred.should_take(item)),
            );

            self.take_pred.break_hint()?;
            cf
        }

        #[inline]
        fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
            self.collector.collect_then_finish(
                items
                    .into_iter()
                    .take_while(|item| self.take_pred.should_take(item)),
            )
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
                    .take_any_while(|&num| num >= 0)
            },
            should_break_pred: move |mut iter| {
                iter.clone().count() >= take_count || !iter.take_iter().all(|num| num >= 0)
            },
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
