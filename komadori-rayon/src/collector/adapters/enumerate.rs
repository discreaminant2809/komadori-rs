use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase,
        plumbing::{Consumer, DefineSerial},
    },
    helpers::unique,
};

/// A parallel collector that feeds the underlying collector
/// with the position of an item alongside with the item.
///
/// This `struct` is created by [`ParallelCollectorBase::enumerate()`].
/// See its documentation for more.
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

impl<'a, C> DefineSerial<'a> for Enumerate<C>
where
    C: DefineSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<C::Serial>>;
}

impl<C> ParallelCollectorBase for Enumerate<C>
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
        let (actual_len, consumer, commit) = self.collector.parts(len);
        let start = self.idx;

        // We try to panic (in debug) if we can't even afford the actual length.
        std::hint::black_box(self.idx + actual_len);
        // Otherwise, we only add up to usize::MAX.
        self.idx = self.idx.saturating_add(len);
        // But why we even add to `len` and not `actual_len`?
        // Because based on the specifications, we don't know whether it stops
        // even if `actual_len` < `len`. So we must add `len`.
        // The remaining `len - actual_len` are gonna be skipped anyway!

        unique::uniquify((actual_len, consumer::Consumer::new(consumer, start), commit))
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
        let (actual_len, consumer, commit) = self.collector.take_parts(len);
        let start = self.idx;

        // We try to panic (in debug) if we can't even afford the actual length.
        std::hint::black_box(self.idx + actual_len);
        // Otherwise, we only add up to usize::MAX.
        self.idx = self.idx.saturating_add(len);
        // But why we even add to `len` and not `actual_len`?
        // Because based on the specifications, we don't know whether it stops
        // even if `actual_len` < `len`. So we must add `len`.
        // The remaining `len - actual_len` are gonna be skipped anyway!

        unique::take_uniquify((actual_len, consumer::Consumer::new(consumer, start), commit))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing;

    pub struct Consumer<C> {
        consumer: C,
        start: usize,
    }

    pub struct Serial<C> {
        collector: C,
        idx: usize,
    }

    impl<C> Consumer<C> {
        pub(super) fn new(consumer: C, start: usize) -> Self {
            Self { consumer, start }
        }
    }

    impl<C> IntoCollectorBase for Consumer<C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector>;

        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector: self.consumer.into_collector(),
                idx: self.start,
            }
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
            let start = self.start;
            // The runtime is permitted to split pass we can hold.
            // Due to the "lie" we do in `parts()` and `take_parts()`,
            // we may overflow here if (prior to creating a consumer)
            // `self.idx <= usize::MAX < self.idx + len`
            // (`self` here means the parallel collector, not this consumer)
            self.start = self.start.saturating_add(index);

            (Self { consumer, start }, combiner)
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.consumer.break_hint()
        }
    }

    impl<C> CollectorBase for Serial<C>
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

    impl<C, T> Collector<T> for Serial<C>
    where
        C: Collector<(usize, T)>,
    {
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            // Put this here because if the index is `usize::MAX`
            // and it's the last item the underlying can afford,
            // we should still be able to collect it and exit early
            // instead of panicking (in debug build).
            self.collector.collect((self.idx, item))?;
            self.idx += 1;
            ControlFlow::Continue(())
        }

        // We can't meaningfully override the other two methods,
        // because we need to uphold the "the index is `usize::MAX` and the last item"
        // case, which would lead us to a manual `try_fold()`,
        // which is the default implementation.
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
            (take_count, start) in prop_oneof![..=2_usize, usize::MAX - 2..]
                .prop_flat_map(|start| (..=(usize::MAX - start).min(5), Just(start))),
            pool in CoroutinePool::prop(),
        ) {
            indexed_impl(pool, split_decision, nums, take_count, start)?;
        }
    }

    fn indexed_impl(
        mut pool: CoroutinePool,
        split_decision: IndexedSplitDecision,
        nums: Vec<i32>,
        take_count: usize,
        start: usize,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || {
                let mut collector = vec![].into_par_collector().take(take_count).enumerate();
                collector.idx = start;
                collector
            },
            should_break_pred: |_| nums.len() >= take_count,
            pred: |_, output| {
                let expected: Vec<_> = nums
                    .iter()
                    .copied()
                    .zip(start..=usize::MAX)
                    .map(|(num, i)| (i, num))
                    .take(take_count)
                    .collect();

                PredError::assert_eq(output, expected)
            },
        }
        .test_par_collector(&mut pool, &split_decision)
    }
}
