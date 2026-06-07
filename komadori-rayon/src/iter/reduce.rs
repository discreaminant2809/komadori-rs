use std::{fmt::Debug, ops::ControlFlow};

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that reduces all collected items into a single value
/// by repeatedly applying a reduction function.
///
/// If no items have been collected, its [`Output`](ParallelCollectorBase::Output) is `None`;
/// otherwise, it returns `Some` containing the result of the reduction.
///
/// This collector corresponds to [`Iterator::reduce()`], except the closure is
/// the "left" value mutated by the "right" value instead of the two values
/// producing another value. Also, the application order is unspecified rather
/// than strictly from left to right, but it is still guaranteed that when
/// two items are fed into the closure, the first one is left compared to
/// the second one (the "right" value).
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParReduce};
///
/// let res = [3, 2, 5, 1, 4]
///     .into_par_iter()
///     .feed_into(ParReduce::new(|accum, num| *accum += num));
///
/// assert_eq!(res, Some(15));
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParReduce};
///
/// let res = ([] as [i32; _])
///     .into_par_iter()
///     .feed_into(ParReduce::new(|accum, num| *accum += num));
///
/// assert_eq!(res, None);
/// ```
#[derive(Clone)]
pub struct ParReduce<T, F> {
    accum: Option<T>,
    f: F,
}

impl<T: Debug, F> Debug for ParReduce<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reduce")
            .field("accum", &self.accum)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

impl<T, F> ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    /// Creates a new instance of this parallel collector with a given accumulator.
    #[inline]
    pub const fn new(f: F) -> Self {
        assert_unindexed_par_collector::<_, T>(Self { accum: None, f })
    }
}

impl<'this, T, F> DefineSerial<'this> for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<T, &'this F>>;
}

impl<T, F> ParallelCollectorBase for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.accum
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
        unique::uniquify((len, consumer::Consumer::new(&self.f), |output| {
            crate::iter::combine_opt(&mut self.accum, output, &self.f);
            ControlFlow::Continue(())
        }))
    }
}

impl<'this, T, F> DefineUnindexedSerial<'this> for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, consumer::Serial<T, &'this F>>;
}

impl<T, F> UnindexedParallelCollectorBase for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
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
        unique_unindexed::uniquify((consumer::Consumer::new(&self.f), |output| {
            crate::iter::combine_opt(&mut self.accum, output, &self.f);
            ControlFlow::Continue(())
        }))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::marker::PhantomData;

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer<T, F> {
        f: F,
        _marker: PhantomData<T>,
    }

    pub struct Combiner<F> {
        f: F,
    }

    pub type Serial<T, F> = komadori::iter::Reduce<T, F>;

    impl<T, F> Consumer<T, F> {
        pub(super) fn new(f: F) -> Self {
            Self {
                f,
                _marker: PhantomData,
            }
        }
    }

    impl<T, F> IntoCollectorBase for Consumer<T, F>
    where
        F: FnMut(&mut T, T),
    {
        type Output = Option<T>;

        type IntoCollector = komadori::iter::Reduce<T, F>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Self::IntoCollector::new(self.f)
        }
    }

    impl<T, F> plumbing::Consumer for Consumer<T, F>
    where
        T: Send,
        F: FnMut(&mut T, T) + Clone + Send,
    {
        type Combiner = Combiner<F>;

        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl<T, F> plumbing::UnindexedConsumer for Consumer<T, F>
    where
        T: Send,
        F: FnMut(&mut T, T) + Clone + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                f: self.f.clone(),
                _marker: PhantomData,
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner { f: self.f.clone() }
        }
    }

    impl<F, T> plumbing::Combiner<Option<T>> for Combiner<F>
    where
        F: FnMut(&mut T, T),
    {
        #[inline]
        fn combine(self, left: &mut Option<T>, right: Option<T>) {
            crate::iter::combine_opt(left, right, self.f);
        }
    }
}

#[cfg(test)]
mod proptests {
    use std::ops::RangeInclusive;

    use super::ParReduce;

    use crate::test_utils::prelude::*;

    // Won't overflow since we only add up to ±300,000,000 * 6 = ±1,800,000,000.
    const NUM_RANGE: RangeInclusive<i32> = -300_000_000..=300_000_000;

    proptest! {
        /// Pre-requisite: None
        #[test]
        fn indexed(
            (split_decision, nums) in propvec(NUM_RANGE, ..=5)
                .prop_flat_map(|nums| {
                    (IndexedSplitStrategy::new(nums.len(), DEFAULT_MAX_DEPTH), Just(nums))
                }),
            starting_num in prop_opt(NUM_RANGE),
            pool in CoroutinePool::prop(),
        ) {
            indexed_impl(pool, split_decision, starting_num, nums)?;
        }
    }

    proptest! {
        /// Pre-requisite: None
        #[test]
        fn unindexed(
            nums in propvec(NUM_RANGE, ..=5),
            split_decision in UnindexedSplitStrategy::new(DEFAULT_MAX_DEPTH),
            starting_num in prop_opt(NUM_RANGE),
            pool in CoroutinePool::prop(),
        ) {
            unindexed_impl(pool, split_decision, starting_num, nums)?;
        }
    }

    fn indexed_impl(
        mut pool: CoroutinePool,
        split_decision: IndexedSplitDecision,
        starting_num: Option<i32>,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        par_collector_tester(starting_num, &nums).test_par_collector(&mut pool, &split_decision)
    }

    fn unindexed_impl(
        mut pool: CoroutinePool,
        split_decision: UnindexedSplitDecision,
        starting_num: Option<i32>,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        par_collector_tester(starting_num, &nums).test_unindexed_par_collector(&mut pool, &split_decision)
    }

    fn par_collector_tester(
        starting_num: Option<i32>,
        nums: &[i32],
    ) -> impl ParallelCollectorTester + UnindexedParallelCollectorTester {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: move || {
                let mut collector = ParReduce::new(|a, b| *a += b);
                collector.accum = starting_num;
                collector
            },
            should_break_pred: |_| false,
            pred: move |mut iter, output| {
                PredError::assert_eq(
                    output,
                    starting_num
                        .into_iter()
                        .chain(iter.take_iter())
                        .reduce(|a, b| a + b),
                )
            },
        }
    }
}
