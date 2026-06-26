use std::fmt::{Debug, Display};

use proptest::{
    prelude::*,
    test_runner::{Reason, TestCaseResult},
};

use crate::collector::{
    ParallelCollector, ParallelCollectorBase, UnindexedParallelCollector, UnindexedParallelCollectorBase,
};

use super::{
    CoroutinePool, IndexedParallelIterator, IndexedSplitDecision, ParallelIterator, UnindexedSplitDecision,
};

pub trait ParallelCollectorTester {
    type Item<'a>
    where
        Self: 'a;
    type Output<'a>
    where
        Self: 'a;

    #[allow(clippy::type_complexity)] // Can't satisfy it so I suppress it.
    fn test_parts<'a>(
        &'a mut self,
    ) -> TestParts<
        impl IndexedParallelIterator<Item = Self::Item<'a>>,
        impl ParallelCollector<Self::Item<'a>, Output = Self::Output<'a>>,
        impl FnOnce(Self::Output<'a>) -> Result<(), PredError>,
    >;

    fn test_par_collector(
        mut self,
        pool: &mut CoroutinePool,
        split_decision: &IndexedSplitDecision,
    ) -> TestCaseResult
    where
        Self: Sized,
    {
        test_collector_impl(&mut self, pool, split_decision)
    }
}

pub trait UnindexedParallelCollectorTester {
    type Item<'a>
    where
        Self: 'a;
    type Output<'a>
    where
        Self: 'a;

    #[allow(clippy::type_complexity)] // Can't satisfy it so I suppress it.
    fn test_parts_unindexed<'a>(
        &'a mut self,
    ) -> TestParts<
        impl ParallelIterator<Item = Self::Item<'a>>,
        impl UnindexedParallelCollector<Self::Item<'a>, Output = Self::Output<'a>>,
        impl FnOnce(Self::Output<'a>) -> Result<(), PredError>,
    >;

    fn test_unindexed_par_collector(
        mut self,
        pool: &mut CoroutinePool,
        split_decision: &UnindexedSplitDecision,
    ) -> TestCaseResult
    where
        Self: Sized,
    {
        test_unindexed_collector_impl(&mut self, pool, split_decision)
    }
}

/// Test parts for collector testing.
pub struct TestParts<I, C, P>
where
    I: ParallelIterator,
    C: ParallelCollector<I::Item>,
    P: FnOnce(C::Output) -> Result<(), PredError>,
{
    /// Parallel iterator provided to feed the collector.
    pub iter: I,
    /// Collector to be tested.
    pub collector: C,
    /// Determines whether the collector should have stopped accumulating
    /// after operation.
    pub should_break: bool,
    /// Predicate on the following being satisfied:
    /// - Output of the collector.
    /// - Remaining of the iterator after the operation, collected to a [`Vec`].
    pub pred: P,
    // FIXME: Visit back later
    // pub iter_for_fuse_test: Option<IF>,
}

/// An error returned when the collection operations of the collector are not satisfied.
#[derive(Debug)]
pub enum PredError {
    /// Incorrect [`Output`] produced by the collector
    ///
    /// [`Output`]: crate::collector::ParallelCollectorBase::Output
    IncorrectOutput(String),
    // FIXME: Visit back later
    // /// The [`ParallelIterator`] is not consumed as expected.
    // IncorrectIterConsumption,
}

/// Basic implementation for [`ParallelCollectorTester`] and [`UnindexedParallelCollectorTester`]
/// for most use case.
/// Opt-out if you test the `collector(_mut)` variant, or the collector and output
/// that may borrow from the tester, or the output is judged differently.
pub struct BasicParallelCollectorTester<ItFac, ClFac, SbPred, Pred, I, C>
// `where` bound is needed otherwise we get "type annotation needed" for the input iterator.
where
    I: ParallelIterator,
    C: ParallelCollector<I::Item>,
    ItFac: FnMut() -> I,
    ClFac: FnMut() -> C,
    SbPred: FnMut(I) -> bool,
    Pred: FnMut(I, C::Output) -> Result<(), PredError>,
{
    pub iter_factory: ItFac,
    pub collector_factory: ClFac,
    pub should_break_pred: SbPred,
    pub pred: Pred,
}

impl<ItFac, ClFac, SbPred, Pred, I, C> ParallelCollectorTester
    for BasicParallelCollectorTester<ItFac, ClFac, SbPred, Pred, I, C>
where
    I: IndexedParallelIterator,
    C: ParallelCollector<I::Item>,
    ItFac: FnMut() -> I,
    ClFac: FnMut() -> C,
    SbPred: FnMut(I) -> bool,
    Pred: FnMut(I, C::Output) -> Result<(), PredError>,
{
    type Item<'a>
        = I::Item
    where
        ItFac: 'a,
        ClFac: 'a,
        SbPred: 'a,
        Pred: 'a,
        I: 'a,
        C: 'a;
    type Output<'a>
        = C::Output
    where
        ItFac: 'a,
        ClFac: 'a,
        SbPred: 'a,
        Pred: 'a,
        I: 'a,
        C: 'a;

    fn test_parts<'a>(
        &'a mut self,
    ) -> TestParts<
        impl IndexedParallelIterator<Item = Self::Item<'a>>,
        impl ParallelCollector<Self::Item<'a>, Output = Self::Output<'a>>,
        impl FnOnce(Self::Output<'a>) -> Result<(), PredError>,
    > {
        TestParts {
            iter: (self.iter_factory)(),
            collector: (self.collector_factory)(),
            should_break: (self.should_break_pred)((self.iter_factory)()),
            pred: |output| (self.pred)((self.iter_factory)(), output),
        }
    }
}

impl<ItFac, ClFac, SbPred, Pred, I, C> UnindexedParallelCollectorTester
    for BasicParallelCollectorTester<ItFac, ClFac, SbPred, Pred, I, C>
where
    I: ParallelIterator,
    C: UnindexedParallelCollector<I::Item>,
    ItFac: FnMut() -> I,
    ClFac: FnMut() -> C,
    SbPred: FnMut(I) -> bool,
    Pred: FnMut(I, C::Output) -> Result<(), PredError>,
{
    type Item<'a>
        = I::Item
    where
        ItFac: 'a,
        ClFac: 'a,
        SbPred: 'a,
        Pred: 'a,
        I: 'a,
        C: 'a;
    type Output<'a>
        = C::Output
    where
        ItFac: 'a,
        ClFac: 'a,
        SbPred: 'a,
        Pred: 'a,
        I: 'a,
        C: 'a;

    fn test_parts_unindexed<'a>(
        &'a mut self,
    ) -> TestParts<
        impl ParallelIterator<Item = Self::Item<'a>>,
        impl UnindexedParallelCollector<Self::Item<'a>, Output = Self::Output<'a>>,
        impl FnOnce(Self::Output<'a>) -> Result<(), PredError>,
    > {
        TestParts {
            iter: (self.iter_factory)(),
            collector: (self.collector_factory)(),
            should_break: (self.should_break_pred)((self.iter_factory)()),
            pred: |output| (self.pred)((self.iter_factory)(), output),
        }
    }
}

impl PredError {
    pub fn assert_eq<T>(actual: T, expected: T) -> Result<(), Self>
    where
        T: PartialEq + Debug,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(Self::IncorrectOutput(format!(
                "expected {expected:?}, got {actual:?}"
            )))
        }
    }

    pub fn assert_fn<T, U>(
        actual: T,
        expected: U,
        f: impl FnOnce(&T, &U) -> bool,
        err_msg: impl Display,
    ) -> Result<(), Self>
    where
        T: Debug,
        U: Debug,
    {
        if f(&actual, &expected) {
            Ok(())
        } else {
            Err(Self::IncorrectOutput(format!(
                "{actual:?} and {expected:?} didn't satisfy the predicate: {err_msg}"
            )))
        }
    }

    fn into_test_case_err(self, method_name: impl Display) -> TestCaseError {
        match self {
            PredError::IncorrectOutput(msg) => TestCaseError::Fail(Reason::from(format!(
                "in method `{method_name}()`: incorrect output: {msg}"
            ))),
        }
    }
}

fn test_collector_impl<CT>(
    tester: &mut CT,
    pool: &mut CoroutinePool,
    split_decision: &IndexedSplitDecision,
) -> TestCaseResult
where
    CT: ParallelCollectorTester + ?Sized,
{
    // `parts()`
    {
        let mut parts = tester.test_parts();
        let (_, consumer, commit) = parts.collector.parts(parts.iter.len());

        let output = pool.bridge(parts.iter.indexed_producer(), consumer, split_decision);
        prop_assert_eq!(
            commit(output).is_break(),
            parts.should_break,
            "in `parts()`: (unindexed) parallel collector didn't break correctly",
        );

        (parts.pred)(parts.collector.finish()).map_err(|e| e.into_test_case_err("parts"))?;
    }

    // `take_parts()`
    {
        let mut parts = tester.test_parts();
        let (_, consumer, commit) = parts.collector.take_parts(parts.iter.len());

        let output = pool.bridge(parts.iter.indexed_producer(), consumer, split_decision);
        commit(output);

        (parts.pred)(parts.collector.finish()).map_err(|e| e.into_test_case_err("take_parts"))?;
    }

    Ok(())
}

fn test_unindexed_collector_impl<CT>(
    tester: &mut CT,
    pool: &mut CoroutinePool,
    split_decision: &UnindexedSplitDecision,
) -> TestCaseResult
where
    CT: UnindexedParallelCollectorTester + ?Sized,
{
    // `parts_unindexed`
    {
        let mut parts = tester.test_parts_unindexed();
        let (consumer, commit) = parts.collector.parts_unindexed();

        let output = pool.bridge_unindexed(parts.iter.take_producer(), consumer, split_decision);
        prop_assert_eq!(
            commit(output).is_break(),
            parts.should_break,
            "in `parts_unindexed()`: (unindexed) parallel collector didn't break correctly",
        );

        (parts.pred)(parts.collector.finish()).map_err(|e| e.into_test_case_err("parts_unindexed"))?;
    }

    // `take_parts_unindexed`
    {
        let mut parts = tester.test_parts_unindexed();
        let (consumer, commit) = parts.collector.take_parts_unindexed();

        let output = pool.bridge_unindexed(parts.iter.take_producer(), consumer, split_decision);
        commit(output);

        (parts.pred)(parts.collector.finish()).map_err(|e| e.into_test_case_err("take_parts_unindexed"))?;
    }

    Ok(())
}
