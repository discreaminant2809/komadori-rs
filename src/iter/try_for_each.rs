use std::{fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{Collector, CollectorBase, assert_collector},
    ops::Try,
};

/// A collector that calls a provided fallible closure for each collected item.
///
/// There are two constructors you can choose from:
///
/// - [`new(f)`](TryForEach::new): Starts with nothing
///   and continues accumulating as long as the closure returns a "success."
///   The [`Output`] is a "success" if the closure **never** returns
///   a "failure." Otherwise, accumulation stops and
///   the [`Output`] becomes that "failure."
///
///   This is the closest to [`Iterator::try_for_each()`].
///   You can use this if premature collecting is not an issue.
///
/// - [`init(init, f)`](TryForEach::init): Starts with either a "success" or a "failure"
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
/// # Examples
///
/// ```
/// use std::io::Write;
/// use komadori::{prelude::*, iter::TryForEach};
///
/// let mut writer = &mut [0_u8; 14][..];
/// let mut collector = TryForEach::new(|data| writer.write_all(data));
///
/// assert!(collector.collect(b"noble").is_continue());
/// assert!(collector.collect(b"and").is_continue());
/// assert!(collector.collect(b"singer").is_continue());
///
/// assert!(collector.finish().is_ok());
/// ```
///
/// Short-circuiting:
///
/// ```
/// use std::io::Write;
/// use komadori::{prelude::*, iter::TryForEach};
///
/// let mut writer = &mut [0_u8; 14][..];
/// let mut collector = TryForEach::new(|data| writer.write_all(data));
///
/// assert!(collector.collect(b"noble").is_continue());
/// assert!(collector.collect(b"and").is_continue());
///
/// // Only 6 bytes left, but we write 7 bytes.
/// assert!(collector.collect(b"???????").is_break());
///
/// assert!(collector.finish().is_err());
/// ```
///
/// You can start with a "failure" too!
///
/// ```
/// use std::io::{self, Write};
/// use komadori::{prelude::*, iter::TryForEach};
///
/// let mut writer = &mut [0_u8; 14][..];
/// let mut collector = TryForEach::init(
///     Err(io::Error::other("I don't want to write")),
///     |data: &[u8]| writer.write_all(data),
/// );
///
/// assert!(collector.break_hint().is_break());
///
/// assert!(collector.finish().is_err());
/// ```
///
/// [`Output`]: CollectorBase::Output
pub struct TryForEach<A, F>
where
    A: Try<Output = ()>,
{
    state: ControlFlow<A::Residual, F>,
}

impl<A, F> TryForEach<A, F>
where
    A: Try<Output = ()>,
{
    /// Creates a new instance of this collector with a closure.
    pub const fn new<T>(f: F) -> Self
    where
        F: FnMut(T) -> A,
    {
        assert_collector::<_, T>(Self {
            state: ControlFlow::Continue(f),
        })
    }

    /// Creates a new instance of this collector with either a "success" or a "failure"
    /// and a closure.
    pub fn init<T>(init: A, f: F) -> Self
    where
        F: FnMut(T) -> A,
    {
        assert_collector::<_, T>(Self {
            state: match init.branch() {
                ControlFlow::Continue(_) => ControlFlow::Continue(f),
                ControlFlow::Break(residual) => ControlFlow::Break(residual),
            },
        })
    }
}

impl<A, F> CollectorBase for TryForEach<A, F>
where
    A: Try<Output = ()>,
{
    type Output = A;

    #[inline]
    fn finish(self) -> Self::Output {
        match self.state {
            ControlFlow::Continue(_) => A::from_output(()),
            ControlFlow::Break(residual) => A::from_residual(residual),
        }
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        match self.state {
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
            ControlFlow::Break(_) => ControlFlow::Break(()),
        }
    }
}

impl<A, F, T> Collector<T> for TryForEach<A, F>
where
    A: Try<Output = ()>,
    F: FnMut(T) -> A,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match &mut self.state {
            ControlFlow::Continue(f) => match f(item).branch() {
                ControlFlow::Continue(_) => ControlFlow::Continue(()),
                ControlFlow::Break(residual) => {
                    self.state = ControlFlow::Break(residual);
                    ControlFlow::Break(())
                }
            },
            ControlFlow::Break(_) => ControlFlow::Break(()),
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match &mut self.state {
            ControlFlow::Continue(f) => {
                match items.into_iter().try_for_each(move |item| f(item).branch()) {
                    ControlFlow::Continue(_) => ControlFlow::Continue(()),
                    ControlFlow::Break(residual) => {
                        self.state = ControlFlow::Break(residual);
                        ControlFlow::Break(())
                    }
                }
            }
            ControlFlow::Break(_) => ControlFlow::Break(()),
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.state {
            ControlFlow::Continue(mut f) => {
                match items.into_iter().try_for_each(move |item| f(item).branch()) {
                    ControlFlow::Continue(_) => A::from_output(()),
                    ControlFlow::Break(residual) => A::from_residual(residual),
                }
            }
            ControlFlow::Break(residual) => A::from_residual(residual),
        }
    }
}

impl<A, F> Debug for TryForEach<A, F>
where
    A: Try<Output = (), Residual: Debug>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct DebugState<'a, R, F> {
            state: &'a ControlFlow<R, F>,
        }

        impl<R, F> Debug for DebugState<'_, R, F>
        where
            R: Debug,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.state {
                    ControlFlow::Continue { .. } => f
                        .debug_tuple("Continue")
                        .field(&std::any::type_name::<F>())
                        .finish(),
                    ControlFlow::Break(residual) => f.debug_tuple("Break").field(residual).finish(),
                }
            }
        }

        f.debug_struct("TryForEach")
            .field("state", &DebugState { state: &self.state })
            .finish()
    }
}

impl<A, F> Clone for TryForEach<A, F>
where
    A: Try<Output = (), Residual: Clone>,
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

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::result::maybe_ok as propresult;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use std::convert::identity;

    use super::*;

    proptest! {
        /// [`TryFold::new()`]
        #[test]
        fn all_collect_methods_new(
            nums in propvec(u8_result(), ..=5),
        ) {
            all_collect_methods_impl(
                nums,
                Ok(()),
                || TryForEach::new(identity),
            )?;
        }
    }

    proptest! {
        /// [`TryFold::init()`]
        #[test]
        fn all_collect_methods_init(
            nums in propvec(u8_result(), ..=5),
            init in u8_result(),
        ) {
            all_collect_methods_impl(
                nums,
                init,
                || TryForEach::init(init, identity),
            )?;
        }
    }

    fn all_collect_methods_impl<C>(
        nums: Vec<I32Result>,
        init: I32Result,
        collector_factory: impl FnMut() -> C,
    ) -> TestCaseResult
    where
        C: Collector<I32Result, Output = I32Result>,
    {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory,
            should_break_pred: |iter| init.is_err() || iter_output(iter).is_err(),
            pred: |mut iter, output, remaining| {
                let expected = if init.is_err() {
                    init
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

    type I32Result = Result<(), i32>;

    fn u8_result() -> impl Strategy<Value = I32Result> {
        propresult(Just(()), any::<i32>())
    }

    fn iter_output(iter: impl IntoIterator<Item = I32Result>) -> I32Result {
        iter.into_iter().try_for_each(identity)
    }
}

// There seems to be no problem, unlike TryFold.
fn _adsknjsadknjads() {
    use crate::prelude::*;

    [1, 2, 3]
        .into_iter()
        .feed_into(TryForEach::init(Some(()), |_| Some(())));
}
