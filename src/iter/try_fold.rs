use std::{fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{Collector, CollectorBase, assert_collector},
    ops::Try,
};

/// A collector that accumulates items using a closure
/// as long as the closure returns a "success."
///
/// There are two constructors you can choose from:
///
/// - [`new(init, f)`](TryFold::new): Starts with either a "success" or a "failure"
///   and continues accumulating as long as the closure returns a "success."
///   The [`Output`] is a "success" if the closure never returns
///   a "failure." Otherwise, accumulation stops and
///   the [`Output`] becomes that "failure."
///
///   With this constructor, you can control whether accumulation
///   stops from the start by providing a "failure" as `init`
///   to avoid consuming one item prematurely.
///
/// - [`with_output(output, f)`](TryFold::with_output): Starts with a "success"
///   and continues accumulating as long as the closure returns a "success."
///   The [`Output`] is a "success" if the closure never returns
///   a "failure." Otherwise, accumulation stops and
///   the [`Output`] becomes that "failure."
///
///   This is the closest to [`Iterator::try_fold()`]. However,
///   this constructor may cause type inference issues
///   and the resulting collector may consume one item prematurely.
///   Prefer the constructor above whenever possible.
///
/// This collector corresponds to [`Iterator::try_fold()`], except that
/// the accumulated value is mutated in place.
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::TryFold};
///
/// let mut collector = TryFold::new(Some(0_i8), |sum, num| {
///     *sum = sum.checked_add(num)?;
///     Some(())
/// });
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert_eq!(collector.finish(), Some(6));
/// ```
///
/// Short-circuiting:
///
/// ```
/// use komadori::{prelude::*, iter::TryFold};
///
/// let mut collector = TryFold::new(Some(0_i8), |sum, num| {
///     *sum = sum.checked_add(num)?;
///     Some(())
/// });
///
/// assert!(collector.collect(60).is_continue());
/// assert!(collector.collect(60).is_continue());
///
/// // The addition operation overflows.
/// assert!(collector.collect(60).is_break());
///
/// assert_eq!(collector.finish(), None);
/// ```
///
/// You can start with a "failure" too!
///
/// ```
/// use komadori::{prelude::*, iter::TryFold};
///
/// let mut collector = TryFold::new(None, |sum: &mut i8, num| {
///     *sum = sum.checked_add(num)?;
///     Some(())
/// });
///
/// assert!(collector.break_hint().is_break());
///
/// assert_eq!(collector.finish(), None);
/// ```
///
/// [`Output`]: CollectorBase::Output
pub struct TryFold<A, F>
where
    A: Try,
{
    state: State<A, F>,
}

enum State<A, F>
where
    A: Try,
{
    Continue { accum: A::Output, f: F },
    Break(A::Residual),
}

impl<A, F> TryFold<A, F>
where
    A: Try,
{
    /// Creates a new instance of this collector with either a "sucess" or a "failure"
    /// and an accumulator.
    #[inline]
    pub fn new<T, R>(init: A, f: F) -> Self
    where
        F: FnMut(&mut A::Output, T) -> R,
        R: Try<Output = (), Residual = A::Residual>,
    {
        assert_collector::<_, T>(TryFold {
            state: match init.branch() {
                ControlFlow::Continue(accum) => State::Continue { accum, f },
                ControlFlow::Break(residual) => State::Break(residual),
            },
        })
    }

    /// Creates a new instance of this collector with a "success" and an accumulator.
    #[inline]
    pub const fn with_output<T, R>(output: A::Output, f: F) -> Self
    where
        F: FnMut(&mut A::Output, T) -> R,
        R: Try<Output = (), Residual = A::Residual>,
    {
        assert_collector::<_, T>(TryFold {
            state: State::Continue { accum: output, f },
        })
    }
}

impl<A, F> CollectorBase for TryFold<A, F>
where
    A: Try,
{
    type Output = A;

    #[inline]
    fn finish(self) -> Self::Output {
        match self.state {
            State::Continue { accum, .. } => A::from_output(accum),
            State::Break(residual) => A::from_residual(residual),
        }
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        match self.state {
            State::Continue { .. } => ControlFlow::Continue(()),
            State::Break(_) => ControlFlow::Break(()),
        }
    }
}

impl<A, F, T, R> Collector<T> for TryFold<A, F>
where
    A: Try,
    F: FnMut(&mut A::Output, T) -> R,
    R: Try<Output = (), Residual = A::Residual>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match &mut self.state {
            State::Continue { accum, f } => match f(accum, item).branch() {
                ControlFlow::Continue(_) => ControlFlow::Continue(()),
                ControlFlow::Break(residual) => {
                    self.state = State::Break(residual);
                    ControlFlow::Break(())
                }
            },
            State::Break(_) => ControlFlow::Break(()),
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match &mut self.state {
            State::Continue { accum, f } => match items
                .into_iter()
                .try_for_each(move |item| f(accum, item).branch())
            {
                ControlFlow::Continue(_) => ControlFlow::Continue(()),
                ControlFlow::Break(residual) => {
                    self.state = State::Break(residual);
                    ControlFlow::Break(())
                }
            },
            State::Break(_) => ControlFlow::Break(()),
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.state {
            State::Continue { mut accum, mut f } => match items.into_iter().try_for_each({
                let accum = &mut accum;
                move |item| f(accum, item).branch()
            }) {
                ControlFlow::Continue(_) => A::from_output(accum),
                ControlFlow::Break(residual) => A::from_residual(residual),
            },
            State::Break(residual) => A::from_residual(residual),
        }
    }
}

impl<A, F> Debug for TryFold<A, F>
where
    A: Try<Output: Debug, Residual: Debug>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TryFold")
            .field("state", &self.state)
            .finish()
    }
}

impl<A, F> Debug for State<A, F>
where
    A: Try<Output: Debug, Residual: Debug>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Continue { accum, .. } => f
                .debug_struct("Continue")
                .field("accum", accum)
                .field("f", &std::any::type_name::<F>())
                .finish(),
            Self::Break(residual) => f.debug_tuple("Break").field(residual).finish(),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::*;

    proptest! {
        /// [`TryFold::new()`]
        #[test]
        fn all_collect_methods_new(
            nums in propvec(any::<u8>(), ..=5),
            starting_num in proptest::option::of(Just(0_u8))
        ) {
            all_collect_methods_impl(
                nums,
                starting_num.is_none(),
                || TryFold::new(starting_num, collector_closure),
            )?;
        }
    }

    proptest! {
        /// [`TryFold::with_output()`]
        #[test]
        fn all_collect_methods_with_output(
            nums in propvec(any::<u8>(), ..=5),
        ) {
            all_collect_methods_impl(
                nums,
                false,
                || TryFold::with_output(0_u8, collector_closure),
            )?;
        }
    }

    fn all_collect_methods_impl<C>(
        nums: Vec<u8>,
        break_from_start: bool,
        collector_factory: impl FnMut() -> C,
    ) -> TestCaseResult
    where
        C: Collector<u8, Output = Option<u8>>,
    {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory,
            should_break_pred: |iter| break_from_start || iter_output(iter).is_none(),
            pred: |mut iter, output, remaining| {
                let expected = if break_from_start {
                    None
                } else {
                    iter_output(&mut iter)
                };

                if expected != output {
                    Err(PredError::IncorrectOutput)
                } else if iter.ne(remaining) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }

    fn collector_closure(sum: &mut u8, num: u8) -> Option<()> {
        *sum = sum.checked_add(num)?;
        Some(())
    }

    fn iter_output(iter: impl IntoIterator<Item = u8>) -> Option<u8> {
        iter.into_iter().try_fold(0_u8, u8::checked_add)
    }
}

// This is to prove that `with_output` (traditional try_fold()) shouldn't be the default.
fn _akjdas() {
    use crate::prelude::*;

    let sum = [10, 20, 30, 100, 40, 50]
        .into_iter()
        .feed_into(TryFold::new(Some(0_i8), |sum, num| {
            *sum = sum.checked_add(num)?;
            Some(())
        }));

    assert_eq!(sum, None);
}
