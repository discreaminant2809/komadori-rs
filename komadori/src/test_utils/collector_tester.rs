use std::fmt::Debug;

use proptest::{prelude::*, test_runner::TestCaseResult};

use crate::collector::{Collector, CollectorBase};

/// Test helper that returns parts needed for collector proptest.
///
/// # Notes
///
/// The [`Output`] should be reset for every call. May not needed
/// if you can make the output consistent without resetting.
///
/// [`Output`]: CollectorTester::Output
pub trait CollectorTester {
    type Item<'a>
    where
        Self: 'a;
    type Output<'a>
    where
        Self: 'a;

    #[allow(clippy::type_complexity)] // Can't satisfy it so I suppress it.
    fn collector_test_parts<'a>(
        &'a mut self,
    ) -> CollectorTestParts<
        impl Iterator<Item = Self::Item<'a>>,
        impl Collector<Self::Item<'a>, Output = Self::Output<'a>>,
        impl FnMut(Self::Output<'a>, &mut dyn Iterator<Item = Self::Item<'a>>) -> Result<(), PredError>,
        impl Iterator<Item = Self::Item<'a>>,
    >;
}

/// Test parts for collector testing.
pub struct CollectorTestParts<I, C, P, IF>
where
    I: Iterator,
    C: Collector<I::Item>,
    P: FnMut(C::Output, &mut dyn Iterator<Item = I::Item>) -> Result<(), PredError>,
    IF: Iterator<Item = I::Item>,
{
    /// Iterator provided to feed the collector.
    pub iter: I,
    /// Collector to be tested.
    pub collector: C,
    /// Determines whether the collector should have stopped accumulating
    /// after operation.
    pub should_break: bool,
    /// Predicate on the following being satisfied:
    /// - Output of the collector.
    /// - Remaining of the iterator after the operation.
    pub pred: P,
    pub iter_for_fuse_test: Option<IF>,
}

/// An error returned when the collection operations of the collector are not satisfied.
#[derive(Debug)]
pub enum PredError {
    /// Incorrect [`Output`] produced by the collector
    ///
    /// [`Output`]: crate::collector::Collector::Output
    IncorrectOutput,
    /// The [`Iterator`] is not consumed as expected.
    IncorrectIterConsumption,
}

impl PredError {
    fn of_method(self, name: &'static str) -> OfMethod {
        OfMethod {
            name,
            pred_error: self,
        }
    }
}

/// Helper to convert [`PredError`] into [`TestCaseError`].
struct OfMethod {
    name: &'static str,
    pred_error: PredError,
}

impl From<OfMethod> for TestCaseError {
    fn from(OfMethod { name, pred_error }: OfMethod) -> Self {
        Self::Fail(format!("`{name}()` is implemented incorrectly: {pred_error:?}").into())
    }
}

/// Used because we don't want the user to override any methods here.
pub trait CollectorTesterExt: CollectorTester {
    fn test_collector(&mut self) -> TestCaseResult {
        test_collector_part(self)
    }
}

impl<CT> CollectorTesterExt for CT where CT: CollectorTester {}

/// Basic implementation for [`CollectorTester`] for most use case.
/// Opt-out if you test the `collector(_mut)` variant, or the collector and output
/// that may borrow from the tester, or the output is judged differently.
pub struct BasicCollectorTester<ItFac, ClFac, SbPred, Pred, I, C>
// `where` bound is needed otherwise we get "type annotation needed" for the input iterator.
where
    I: Iterator,
    C: Collector<I::Item>,
    ItFac: FnMut() -> I,
    ClFac: FnMut() -> C,
    SbPred: FnMut(I) -> bool,
    Pred: FnMut(I, C::Output, &mut dyn Iterator<Item = I::Item>) -> Result<(), PredError>,
{
    pub iter_factory: ItFac,
    pub collector_factory: ClFac,
    pub should_break_pred: SbPred,
    pub pred: Pred,
}

impl<ItFac, ClFac, SbPred, Pred, I, C> CollectorTester
    for BasicCollectorTester<ItFac, ClFac, SbPred, Pred, I, C>
where
    I: Iterator + Clone,
    C: Collector<I::Item>,
    ItFac: FnMut() -> I,
    ClFac: FnMut() -> C,
    SbPred: FnMut(I) -> bool,
    Pred: FnMut(I, C::Output, &mut dyn Iterator<Item = I::Item>) -> Result<(), PredError>,
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

    fn collector_test_parts<'a>(
        &'a mut self,
    ) -> CollectorTestParts<
        impl Iterator<Item = Self::Item<'a>>,
        impl Collector<Self::Item<'a>, Output = Self::Output<'a>>,
        impl FnMut(Self::Output<'a>, &mut dyn Iterator<Item = Self::Item<'a>>) -> Result<(), PredError>,
        impl Iterator<Item = Self::Item<'a>>,
    > {
        CollectorTestParts {
            iter: (self.iter_factory)(),
            collector: (self.collector_factory)(),
            should_break: (self.should_break_pred)((self.iter_factory)()),
            pred: |output, it| (self.pred)((self.iter_factory)(), output, it),
            iter_for_fuse_test: None::<std::iter::Empty<I::Item>>,
        }
    }
}

pub fn none_iter_for_fuse_test<T>() -> Option<impl Iterator<Item = T>> {
    None::<std::iter::Empty<T>>
}

fn test_collector_part<CT>(tester: &mut CT) -> TestCaseResult
where
    CT: CollectorTester + ?Sized,
{
    // `collect()`
    // Introduce scope so that `test_parts` is dropped,
    // or else we get the "mutable more than once" error.
    {
        let mut test_parts = tester.collector_test_parts();
        // Simulate the fact that break_hint is used before looping,
        // which is the intended use case.
        let has_stopped = (|| {
            test_parts.collector.break_hint()?;
            test_parts
                .iter
                .try_for_each(|item| test_parts.collector.collect(item))
        })()
        .is_break();

        prop_assert_eq!(
            has_stopped,
            test_parts.should_break,
            "`collect()` didn't break correctly"
        );

        if has_stopped && let Some(items) = test_parts.iter_for_fuse_test {
            for item in items {
                prop_assert!(
                    test_parts.collector.collect(item).is_break(),
                    "`collect()` isn't actually fused"
                );
            }
        }
        // We may have not considered that the collector is implemeted incorrectly
        // and even if the above test passes, the output of the collector
        // may have been "tainted" by extra items fed.
        // We will catch it also in the below test

        (test_parts.pred)(test_parts.collector.finish(), &mut test_parts.iter)
            .map_err(|e| e.of_method("collect"))?;
    }

    // `collect_many()`
    {
        let mut test_parts = tester.collector_test_parts();
        // We don't call `break_hint()` because it's NOT an intended use case.
        // The user should be able to call it directly without that method.
        let has_stopped = test_parts
            .collector
            .collect_many(&mut test_parts.iter)
            .is_break();
        prop_assert_eq!(
            has_stopped,
            test_parts.should_break,
            "`collect_many()` didn't break correctly"
        );

        if has_stopped && let Some(items) = test_parts.iter_for_fuse_test {
            prop_assert!(
                test_parts.collector.collect_many(items).is_break(),
                "`collect_many()` isn't actually fused"
            );
        }

        (test_parts.pred)(test_parts.collector.finish(), &mut test_parts.iter)
            .map_err(|e| e.of_method("collect_many"))?;
    }

    // `collect_then_finish()`
    {
        let mut test_parts = tester.collector_test_parts();
        (test_parts.pred)(
            test_parts
                .collector
                .collect_then_finish(&mut test_parts.iter),
            &mut test_parts.iter,
        )
        .map_err(|e| e.of_method("collect_then_finish"))?;
    }

    Ok(())
}
