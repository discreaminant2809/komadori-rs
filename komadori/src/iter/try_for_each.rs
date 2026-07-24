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
/// assert_eq!(collector.max_afford(1), 0);
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
    fn max_afford(&self, request: usize) -> usize {
        match self.state {
            ControlFlow::Continue(_) => request,
            ControlFlow::Break(_) => 0,
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
                    ControlFlow::Continue { .. } => f.debug_struct("Continue").finish(),
                    ControlFlow::Break(residual) => f.debug_tuple("Break").field(residual).finish(),
                }
            }
        }

        f.debug_struct("TryForEach")
            .field("state", &DebugState { state: &self.state })
            .field("f", &std::any::type_name::<F>())
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
    use std::convert::identity;

    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(via_new {
        iter_data: {
            let mut nums = propvec(prop_opt5050(any::<()>()), ..=5);
        },
        other_data: {},
        iter: nums.iter().copied(),
        collector: TryForEach::new(identity),
        expected_f: |iter| {
            let res: Option<_> = iter.try_for_each(identity);
            (res, res.is_none())
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    collector_test!(via_init {
        iter_data: {
            let mut nums = propvec(prop_opt5050(any::<()>()), ..=5);
        },
        other_data: {
            let init = prop_opt5050(any::<()>());
        },
        iter: nums.iter().copied(),
        collector: TryForEach::init(init, identity),
        expected_f: |iter| {
            if init.is_none() {
                return (None, true);
            }

            let res = iter.try_for_each(identity);
            (res, res.is_none())
        },
        output_pred: PartialEq::eq,
        model: CollectorModel {
            state: init,
            advance_f: |_: &mut _, _| {},
            max_afford_f: |state: &Option<_>, request| if state.is_none() { 0 } else { request },
        },
    });
}

// There seems to be no problem, unlike TryFold.
fn _adsknjsadknjads() {
    use crate::prelude::*;

    [1, 2, 3]
        .into_iter()
        .feed_into(TryForEach::init(Some(()), |_| Some(())));
}
