use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
    ops::{BasicParClosure, DefineCallMut, ParallelFnMutBase, WithLocalParClosure},
};

// So that we can hide this struct while still be able to satisfy the compiler.
mod inner {
    #[derive(Clone, Debug)]
    pub struct MapBase<C, F> {
        pub(super) collector: C,
        pub(super) f: F,
    }
}
use inner::MapBase;

/// A parallel collector that uses a closure to determine whether
/// an item should be accumulated.
///
/// This `struct` is created by [`ParallelCollectorBase::map()`].
/// See its documentation for more.
pub type Map<C, F> = MapBase<C, BasicParClosure<F>>;

/// A parallel collector that uses a closure and a cloable state
/// to determine whether an item should be accumulated.
///
/// This `struct` is created by
/// [`ParallelCollectorBase::map_with()`].
/// See its documentation for more.
pub type MapWith<C, L1, FL2, F> = MapBase<C, WithLocalParClosure<L1, FL2, F>>;

impl<C, F> Map<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self {
            collector,
            f: BasicParClosure::new(f),
        }
    }
}

impl<C, L1, FL2, F> MapWith<C, L1, FL2, F> {
    pub(in crate::collector) fn new(collector: C, local1: L1, local2_f: FL2, f: F) -> Self {
        Self {
            collector,
            f: WithLocalParClosure::new(local1, local2_f, f),
        }
    }
}

impl<'a, C, F> DefineSerial<'a> for MapBase<C, F>
where
    C: DefineSerial<'a>,
    F: ParallelFnMutBase,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<C::Serial, <F as DefineCallMut<'a>>::CallMut>>;
}

impl<'a, C, F> DefineUnindexedSerial<'a> for MapBase<C, F>
where
    C: DefineUnindexedSerial<'a>,
    F: ParallelFnMutBase,
{
    type UnindexedSerial = unique_unindexed::Serial<
        'a,
        Self,
        consumer::Serial<C::UnindexedSerial, <F as DefineCallMut<'a>>::CallMut>,
    >;
}

impl<C, F> ParallelCollectorBase for MapBase<C, F>
where
    C: ParallelCollectorBase,
    F: ParallelFnMutBase,
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
        unique::uniquify((
            len,
            consumer::Consumer::new(consumer, self.f.callable_mut()),
            commit,
        ))
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
        unique::take_uniquify((
            len,
            consumer::Consumer::new(consumer, self.f.take_callable_mut()),
            commit,
        ))
    }
}

impl<C, F> UnindexedParallelCollectorBase for MapBase<C, F>
where
    C: UnindexedParallelCollectorBase,
    F: ParallelFnMutBase,
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
        unique_unindexed::uniquify((consumer::Consumer::new(consumer, self.f.callable_mut()), commit))
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
        unique_unindexed::take_uniquify((
            consumer::Consumer::new(consumer, self.f.take_callable_mut()),
            commit,
        ))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::{collector::plumbing, ops::CallMut};

    pub struct Consumer<C, FF> {
        consumer: C,
        into_f: FF,
    }

    // Can't utilize from komadori's filter(), since it requires item type right away.
    pub struct Serial<C, F> {
        collector: C,
        f: F,
    }

    impl<C, F> Consumer<C, F> {
        #[inline]
        pub(super) fn new(consumer: C, into_f: F) -> Self {
            Self { consumer, into_f }
        }
    }

    impl<C, FF, F> IntoCollectorBase for Consumer<C, FF>
    where
        C: IntoCollectorBase,
        FF: FnOnce() -> F,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector, F>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector: self.consumer.into_collector(),
                f: (self.into_f)(),
            }
        }
    }

    impl<C, FF, F> plumbing::Consumer for Consumer<C, FF>
    where
        C: plumbing::Consumer,
        FF: FnOnce() -> F + Clone + Send,
    {
        type Combiner = C::Combiner;

        #[inline]
        fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
            let (consumer, combiner) = self.consumer.split_off_left_at(index);
            (
                Self {
                    consumer,
                    into_f: self.into_f.clone(),
                },
                combiner,
            )
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.consumer.break_hint()
        }
    }

    impl<C, PF, P> plumbing::UnindexedConsumer for Consumer<C, PF>
    where
        C: plumbing::UnindexedConsumer,
        PF: FnOnce() -> P + Clone + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                into_f: self.into_f.clone(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
    }

    impl<C, F> CollectorBase for Serial<C, F>
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

    impl<C, F, T> Collector<T> for Serial<C, F>
    where
        C: Collector<F::Output>,
        F: CallMut<(T,)>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            self.collector.collect(self.f.call_mut((item,)))
        }

        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            self.collector
                .collect_many(items.into_iter().map(|item| self.f.call_mut((item,))))
        }

        fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
            self.collector
                .collect_then_finish(items.into_iter().map(move |item| self.f.call_mut((item,))))
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
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || vec![].into_par_collector().take(take_count).map(map_f),
            should_break_pred: |_| nums.len() >= take_count,
            pred: |_, output| {
                PredError::assert_eq(output, nums.iter().copied().map(map_f).take(take_count).collect())
            },
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
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || vec![].into_par_collector().take(take_count).map(map_f),
            should_break_pred: |_| nums.len() >= take_count,
            pred: |_, output| {
                PredError::assert_eq(output.len(), nums.len().min(take_count))?;

                PredError::assert_fn(
                    output,
                    nums.iter().copied().map(map_f).collect::<Vec<_>>(),
                    |actual, expected| is_subsequence(&actual[..], &expected[..]),
                    "not a subsequence",
                )
            },
        }
        .test_unindexed_par_collector(&mut pool, &split_decision)
    }

    fn map_f(num: i32) -> i32 {
        num.wrapping_add(i32::MAX)
    }
}
