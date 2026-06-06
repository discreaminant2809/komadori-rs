use std::{
    fmt::Debug,
    ops::ControlFlow,
    sync::atomic::{AtomicUsize, Ordering},
};

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{DefineSerial, DefineUnindexedSerial},
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that stops accumulating after collecting `n` items,
/// or fewer if the underlying collector stops sooner.
///
/// This `struct` is created by [`ParallelCollectorBase::take()`].
/// See its documentation for more.
#[derive(Debug)]
pub struct Take<C> {
    collector: C,
    remaining: AtomicUsize,
}

impl<C> Take<C> {
    pub(in crate::collector) fn new(collector: C, n: usize) -> Self {
        Self {
            collector,
            remaining: n.into(),
        }
    }
}

impl<'this, C> DefineSerial<'this> for Take<C>
where
    C: DefineSerial<'this>,
{
    type Serial = unique::Serial<'this, Self, indexed::Serial<C::Serial>>;
}

impl<'this, C> DefineUnindexedSerial<'this> for Take<C>
where
    C: DefineUnindexedSerial<'this>,
{
    type UnindexedSerial =
        unique_unindexed::Serial<'this, Self, unindexed::Serial<'this, C::UnindexedSerial>>;
}

impl<C> ParallelCollectorBase for Take<C>
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
        if self.remaining.load(Ordering::Relaxed) == 0 {
            ControlFlow::Break(())
        } else {
            self.collector.break_hint()
        }
    }

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
        let remaining = self.remaining.get_mut();
        let max_len = if *remaining < len {
            std::mem::take(remaining)
        } else {
            *remaining -= len;
            len
        };
        let break_hint = if *remaining == 0 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        };

        // We "lie" to the underlying parallel collector that
        // we only have this amount left.
        let (inner_max_len, consumer, commit) = self.collector.parts(max_len);
        // Only meaningful when we have "nested take()."
        // In this case we can choose a new len of the underlying
        // if appropriate.
        let max_len = inner_max_len.min(max_len);

        unique::uniquify((
            max_len,
            indexed::Consumer::new(consumer, max_len),
            move |output| {
                commit(output)?;
                break_hint
            },
        ))
    }

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
        let remaining = self.remaining.get_mut();
        let max_len = if *remaining < len {
            std::mem::take(remaining)
        } else {
            *remaining -= len;
            len
        };

        // We "lie" to the underlying parallel collector that
        // we only have this amount left.
        let (inner_max_len, consumer, commit) = self.collector.take_parts(max_len);
        // Only meaningful when we have "nested take()."
        // In this case we can choose a new len of the underlying
        // if appropriate.
        let max_len = inner_max_len.min(max_len);

        unique::take_uniquify((max_len, indexed::Consumer::new(consumer, max_len), commit))
    }
}

impl<C> UnindexedParallelCollectorBase for Take<C>
where
    C: UnindexedParallelCollectorBase,
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
        unique_unindexed::uniquify((unindexed::Consumer::new(consumer, &self.remaining), |output| {
            commit(output)?;
            if self.remaining.load(Ordering::Relaxed) == 0 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }))
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
        unique_unindexed::take_uniquify((unindexed::Consumer::new(consumer, &self.remaining), commit))
    }
}

impl<C> Clone for Take<C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            collector: self.collector.clone(),
            remaining: self.remaining.load(Ordering::Relaxed).into(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.collector.clone_from(&source.collector);
        self.remaining
            .store(source.remaining.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

#[allow(missing_debug_implementations)]
mod indexed {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing;

    pub struct Consumer<C> {
        consumer: C,
        n: usize,
    }

    impl<C> Consumer<C> {
        #[inline]
        pub(super) fn new(consumer: C, n: usize) -> Self {
            Self { consumer, n }
        }
    }

    pub type Serial<C> = komadori::collector::Take<C>;

    impl<C> IntoCollectorBase for Consumer<C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            // We have to limit by ourselves.
            // Some collectors may be fed more items than neccessary,
            // since we lied to the underlying collector.
            self.consumer.into_collector().take(self.n)
        }
    }

    impl<C> plumbing::Consumer for Consumer<C>
    where
        C: plumbing::Consumer,
    {
        type Combiner = C::Combiner;

        fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
            let index = index.clamp(0, self.n);
            let (consumer, combiner) = self.consumer.split_off_left_at(index);
            self.n -= index;

            (Self { consumer, n: index }, combiner)
        }

        fn break_hint(&self) -> ControlFlow<()> {
            if self.n == 0 {
                ControlFlow::Break(())
            } else {
                self.consumer.break_hint()
            }
        }
    }
}

#[allow(missing_debug_implementations)]
mod unindexed {
    use std::{
        ops::ControlFlow,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer<'a, C> {
        consumer: C,
        remaining: &'a AtomicUsize,
    }

    pub struct Serial<'a, C> {
        collector: C,
        remaining: &'a AtomicUsize,
    }

    impl<'a, C> Consumer<'a, C> {
        #[inline]
        pub(super) fn new(consumer: C, remaining: &'a AtomicUsize) -> Self {
            Self { consumer, remaining }
        }
    }

    impl<'a, C> IntoCollectorBase for Consumer<'a, C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<'a, C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector: self.consumer.into_collector(),
                remaining: self.remaining,
            }
        }
    }

    impl<C> plumbing::Consumer for Consumer<'_, C>
    where
        C: UnindexedConsumer,
    {
        type Combiner = C::Combiner;

        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            if self.remaining.load(Ordering::Relaxed) == 0 {
                ControlFlow::Break(())
            } else {
                self.consumer.break_hint()
            }
        }
    }

    impl<C> UnindexedConsumer for Consumer<'_, C>
    where
        C: UnindexedConsumer,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                remaining: self.remaining,
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
    }

    impl<C> CollectorBase for Serial<'_, C>
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
            if self.remaining.load(Ordering::Relaxed) == 0 {
                ControlFlow::Break(())
            } else {
                self.collector.break_hint()
            }
        }
    }

    // The implementation is based on `rayon`
    // See: https://docs.rs/rayon/latest/src/rayon/iter/take_any.rs.html
    impl<C, T> Collector<T> for Serial<'_, C>
    where
        C: Collector<T>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            match take_one(self.remaining) {
                TakeResult::NoMore => ControlFlow::Break(()),
                TakeResult::OneLeft => {
                    self.collector.collect(item)?;
                    ControlFlow::Break(())
                }
                TakeResult::MoreThanOne => {
                    self.collector.collect(item)?;
                    ControlFlow::Continue(())
                }
            }
        }

        // Cannot meaningfully override `collect_many` and `collect_then_finish`
    }

    #[inline(always)]
    fn take_one(remaining: &AtomicUsize) -> TakeResult {
        match remaining.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |remaining| {
            remaining.checked_sub(1)
        }) {
            Err(_) => TakeResult::NoMore,
            Ok(0) => unreachable!("the previous value can't be 0 if successful"),
            Ok(1) => TakeResult::OneLeft,
            Ok(_) => TakeResult::MoreThanOne,
        }
    }

    #[repr(usize)]
    enum TakeResult {
        NoMore = 0,
        OneLeft,
        MoreThanOne,
    }
}

#[cfg(test)]
mod proptests {
    use crate::test_utils::prelude::*;

    proptest! {
        /// Pre-requisite:
        /// - [`crate::vec::IntoParCollector`]
        #[test]
        fn indexed(
            take_count in ..=5_usize,
            (split_decision, nums) in propvec(any::<i32>(), ..=5)
                .prop_flat_map(|nums| {
                    (IndexedSplitStrategy::new(nums.len(), DEFAULT_MAX_DEPTH), Just(nums))
                }),
            pool in CoroutinePool::prop(),
        ) {
            indexed_impl(pool, split_decision, nums, take_count)?;
        }
    }

    proptest! {
        #[test]
        fn unindexed(
            take_count in ..=5_usize,
            nums1 in propvec(any::<i32>(), ..=3),
            nums2 in propvec(any::<i32>(), ..=3),
            split_decision in UnindexedSplitStrategy::new(DEFAULT_MAX_DEPTH),
            pool in CoroutinePool::prop(),
        ) {
            unindexed_impl(pool, split_decision, nums1, nums2, take_count)?;
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
            collector_factory: || vec![].into_par_collector().take(take_count),
            should_break_pred: |_| nums.len() >= take_count,
            pred: |_, output| PredError::assert_eq(output, nums.iter().copied().take(take_count).collect()),
        }
        .test_par_collector(&mut pool, &split_decision)
    }

    fn unindexed_impl(
        mut pool: CoroutinePool,
        split_decision: UnindexedSplitDecision,
        nums1: Vec<i32>,
        nums2: Vec<i32>,
        take_count: usize,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || {
                nums1
                    .par_iter()
                    .cloned()
                    .chain(nums2.par_iter().cloned().filter(|&num| num >= 0))
            },
            collector_factory: || vec![].into_par_collector().take(take_count),
            should_break_pred: |iter| iter.count() >= take_count,
            pred: |mut iter, output| {
                PredError::assert_fn(
                    &output[..],
                    take_count,
                    |output, _| output.len() <= take_count,
                    "excessive length",
                )?;

                PredError::assert_fn(
                    output,
                    iter.take_iter().collect::<Vec<_>>(),
                    |output, expected| is_subsequence(output, expected),
                    "not a subsequence",
                )
            },
        }
        .test_unindexed_par_collector(&mut pool, &split_decision)
    }
}

#[cfg(test)]
mod tests {
    use komadori::prelude::*;

    use crate::{
        collector::plumbing::{Combiner, UnindexedConsumer},
        test_utils::{Producer, prelude::*},
    };

    // Turn out our own `ParallelIterator::chain()` is flawed to begin with!
    //
    // Test failed: (unindexed) parallel collector yielded an incorrect output: [1, 0] and [0, 1] didn't satisfy the predicate: not a subsequence.
    // minimal failing input: take_count = 2, nums1 = [
    //     0,
    // ], nums2 = [
    //     1,
    //     -1,
    // ], split_decision = Stay, pool = CoroutinePool {
    //     rng: Xoshiro128PlusPlus {
    //         s: [
    //             12323650,
    //             792414441,
    //             3380224488,
    //             183050292,
    //         ],
    //     },
    // }
    //         successes: 0
    //         local rejects: 0
    //         global rejects: 0
    #[test]
    fn unindexed_fail1() {
        let mut iter = vec![0, 0]
            .into_par_iter()
            .chain(vec![1, -1].into_par_iter().filter(|&num| num >= 0));

        let mut collector = vec![].into_par_collector().take(3);

        let (consumer, commit) = collector.take_parts_unindexed();
        let mut iter = iter.take_iter();
        let mut serial = consumer.into_collector();
        assert!(serial.collect(iter.next().unwrap()).is_continue());
        assert!(serial.collect(iter.next().unwrap()).is_continue());
        assert!(serial.collect(iter.next().unwrap()).is_break());
        commit(serial.finish());

        assert_eq!(collector.finish(), [0, 0, 1]);
    }

    // Turn out our own `ParallelIterator::chain()` is flawed to begin with!
    //
    // Test failed: (unindexed) parallel collector yielded an incorrect output: [0, 1, 0] and [0, 0, 0, 1, 1, 1] didn't satisfy the predicate: not a subsequence.
    // minimal failing input: take_count = 3, nums1 = [
    //     0,
    //     0,
    //     0,
    // ], nums2 = [
    //     1,
    //     1,
    //     1,
    // ], split_decision = Split {
    //     left: Split {
    //         left: Stay,
    //         right: Stay,
    //     },
    //     right: Stay,
    // }, pool = CoroutinePool {
    //     rng: Xoshiro128PlusPlus {
    //         s: [
    //             770253919,
    //             1008533523,
    //             1582379573,
    //             1843798072,
    //         ],
    //     },
    // }
    //         successes: 4
    //         local rejects: 0
    //         global rejects: 0
    #[test]
    fn unindexed_fail2() {
        let mut iter = vec![0, 0, 0]
            .into_par_iter()
            .chain(vec![1, 1, 1].into_par_iter().filter(|&num| num >= 0));

        let mut collector = vec![].into_par_collector().take(3);

        let mut producer = iter.take_producer();
        let (consumer, commit) = collector.take_parts_unindexed();
        let output = {
            let (left_producer, right_producer) = (producer.split_off_left(), producer);
            let combiner = consumer.to_combiner();
            let (left_consumer, right_consumer) = (consumer.split_off_left(), consumer);

            // Contains (when flattened): [0, 1]
            let mut left_output = {
                let mut producer = left_producer;
                let consumer = left_consumer;

                let (left_producer, right_producer) = (producer.split_off_left(), producer);
                let combiner = consumer.to_combiner();
                let (left_consumer, right_consumer) = (consumer.split_off_left(), consumer);

                // Contains (when flattened): []
                let mut left_output = left_producer.into_iter().feed_into(left_consumer);
                // Contains (when flattened): [0, 1]
                let right_output = right_producer.into_iter().feed_into(right_consumer);

                combiner.combine(&mut left_output, right_output);
                left_output
            };
            // Contains (when flattened): [0]
            let right_output = right_producer.into_iter().feed_into(right_consumer);

            combiner.combine(&mut left_output, right_output);
            left_output
        };
        commit(output);

        PredError::assert_fn(
            collector.finish(),
            [0, 0, 0, 1, 1, 1],
            |actual, expected| is_subsequence(actual, expected),
            "not a subsequence",
        )
        .unwrap()
    }
}
