use std::{fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{Collector, CollectorBase, assert_collector, finish_boxed_impl},
    ops::Try,
};

/// A collector that accumulates items using a closure
/// as long as the closure returns a "success."
///
/// There are two constructors you can choose from:
///
/// - [`new(init, f)`](TryFold::new): Starts with either a "success" or a "failure"
///   and continues accumulating as long as the closure returns a "success."
///   The [`Output`] is a "success" if the initial value is a "success" and
///   the closure **never** returns a "failure."
///   Otherwise, accumulation stops and the [`Output`] becomes a "failure"
///   (either from the initial value or from the closure).
///
///   With this constructor, you can control whether accumulation
///   stops from the start by providing a "failure" as `init`
///   to avoid consuming one item prematurely.
///
/// - [`with_output(output, f)`](TryFold::with_output): Starts with a "success"
///   and continues accumulating as long as the closure returns a "success."
///   The [`Output`] is a "success" if the closure **never** returns
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
/// assert_eq!(collector.max_afford(1), 0);
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
    /// Creates a new instance of this collector with either a "success" or a "failure"
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

    finish_boxed_impl!();

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        match self.state {
            State::Continue { .. } => request,
            State::Break(_) => 0,
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
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

impl<A, F> Debug for State<A, F>
where
    A: Try<Output: Debug, Residual: Debug>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Continue { accum, .. } => {
                f.debug_struct("Continue").field("accum", accum).finish()
            }
            Self::Break(residual) => f.debug_tuple("Break").field(residual).finish(),
        }
    }
}

impl<A, F> Clone for TryFold<A, F>
where
    A: Try<Output: Clone, Residual: Clone>,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.state.clone_from(&source.state);
    }
}

impl<A, F> Clone for State<A, F>
where
    A: Try<Output: Clone, Residual: Clone>,
    F: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Continue { accum, f } => Self::Continue {
                accum: accum.clone(),
                f: f.clone(),
            },
            Self::Break(residual) => Self::Break(residual.clone()),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        match (self, source) {
            (
                State::Continue { accum, f },
                State::Continue {
                    accum: source_accum,
                    f: source_f,
                },
            ) => {
                accum.clone_from(source_accum);
                f.clone_from(source_f);
            }

            (State::Break(residual), State::Break(source_residual)) => {
                residual.clone_from(source_residual)
            }

            (this, source) => *this = source.clone(),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(via_new {
        iter_data: {
            let mut nums = propvec(any::<u8>(), ..=5);
        },
        other_data: {
            let init = prop_opt5050(any::<u8>());
        },
        iter: nums.iter().copied(),
        collector: TryFold::new(init, try_fold_f),
        expected_f: |mut iter, _| {
            let Some(init) = init else {
                return (None, true);
            };

            let res = iter.try_fold(init, |mut sum, num| {
                try_fold_f(&mut sum, num)?;
                Some(sum)
            });

            (res, res.is_none())
        },
        output_pred: PartialEq::eq,
        model: CollectorModel {
            state: init,
            advance_f: |_: &mut _, _| {},
            max_afford_f: |state: &Option<_>, request| if state.is_none() { 0 } else { request },
        },
    });

    collector_test!(via_with_output {
        iter_data: {
            let mut nums = propvec(any::<u8>(), ..=5);
        },
        other_data: {
            let init = any::<u8>();
        },
        iter: nums.iter().copied(),
        collector: TryFold::with_output(init, try_fold_f),
        expected_f: |mut iter, _| {
            let res = iter.try_fold(init, |mut sum, num| {
                try_fold_f(&mut sum, num)?;
                Some(sum)
            });

            (res, res.is_none())
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    fn try_fold_f(sum: &mut u8, num: u8) -> Option<()> {
        *sum = sum.checked_add(num)?;
        Some(())
    }
}

// This is to prove that `with_output` (traditional try_fold()) shouldn't be the default.
// fn _akjdas() {
//     use crate::prelude::*;

//     let sum: Option<i8> = [10, 20, 30, 100, 40, 50]
//         .into_iter()
//         .feed_into(TryFold::with_output(0_i8, |sum: &mut i8, num| {
//             *sum = sum.checked_add(num)?;
//             Some(())
//         }));

//     assert_eq!(sum, None);
// }
