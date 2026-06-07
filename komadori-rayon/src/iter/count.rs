use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector_base,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that counts how many items it collected.
///
/// This collector corresponds to [`Iterator::count()`].
///
/// # Overflow Behavior
///
/// This collector does no guarding against overflows, so feeding it
/// more than [`usize::MAX`] items either produces the wrong result or panics.
/// If overflow checks are enabled, a panic is guaranteed.
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParCount};
///
/// let count = (1..=10000)
///     .into_par_iter()
///     .feed_into(ParCount::new());
///
/// assert_eq!(count, 10000);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ParCount {
    count: usize,
}

impl ParCount {
    /// Creates a new instance of this parallel collector with an initial count of 0.
    #[inline]
    pub const fn new() -> Self {
        assert_unindexed_par_collector_base(Self { count: 0 })
    }
}

impl<'this> DefineSerial<'this> for ParCount {
    type Serial = unique::Serial<'this, Self, consumer::Serial>;
}

impl ParallelCollectorBase for ParCount {
    type Output = usize;

    #[inline]
    fn finish(self) -> Self::Output {
        self.count
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
        unique::uniquify((len, consumer::Consumer::new(), |count| {
            self.count += count;
            ControlFlow::Continue(())
        }))
    }
}

impl<'this> DefineUnindexedSerial<'this> for ParCount {
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, komadori::iter::Count>;
}

impl UnindexedParallelCollectorBase for ParCount {
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
        unique_unindexed::uniquify((consumer::Consumer::new(), |count| {
            self.count += count;
            ControlFlow::Continue(())
        }))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer(());

    pub struct Combiner(());

    pub type Serial = komadori::iter::Count;

    impl Consumer {
        #[inline]
        pub(super) fn new() -> Self {
            Self(())
        }
    }

    impl IntoCollectorBase for Consumer {
        type Output = usize;

        type IntoCollector = Serial;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Self::IntoCollector::new()
        }
    }

    impl plumbing::Consumer for Consumer {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl plumbing::UnindexedConsumer for Consumer {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self::new()
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner(())
        }
    }

    impl plumbing::Combiner<usize> for Combiner {
        #[inline]
        fn combine(self, left: &mut usize, right: usize) {
            *left += right;
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::ParCount;

    use crate::test_utils::prelude::*;

    proptest! {
        /// Pre-requisite: None
        #[test]
        fn indexed(
            (split_decision, nums) in propvec(any::<i32>(), ..=5)
                .prop_flat_map(|nums| {
                    (IndexedSplitStrategy::new(nums.len(), DEFAULT_MAX_DEPTH), Just(nums))
                }),
            pool in CoroutinePool::prop(),
        ) {
            indexed_impl(pool, split_decision, nums)?;
        }
    }

    proptest! {
        /// Pre-requisite: None
        #[test]
        fn unindexed(
            nums in propvec(any::<i32>(), ..=5),
            split_decision in UnindexedSplitStrategy::new(DEFAULT_MAX_DEPTH),
            pool in CoroutinePool::prop(),
        ) {
            unindexed_impl(pool, split_decision, nums)?;
        }
    }

    fn indexed_impl(
        mut pool: CoroutinePool,
        split_decision: IndexedSplitDecision,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        par_collector_tester(&nums).test_par_collector(&mut pool, &split_decision)
    }

    fn unindexed_impl(
        mut pool: CoroutinePool,
        split_decision: UnindexedSplitDecision,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        par_collector_tester(&nums).test_unindexed_par_collector(&mut pool, &split_decision)
    }

    fn par_collector_tester(nums: &[i32]) -> impl ParallelCollectorTester + UnindexedParallelCollectorTester {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || ParCount::new(),
            should_break_pred: |_| false,
            pred: |iter, output| PredError::assert_eq(output, iter.count()),
        }
    }
}
