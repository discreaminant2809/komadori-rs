#![allow(deprecated)]

use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector, finish_boxed_impl};

/// A collector that searches for the first item satisfying a predicate.
///
/// If no matching item has been found, its [`Output`] is `None`.
/// When the collector encounters an item that makes the predicate return `true`,
/// it returns [`Break(())`], and the [`Output`] becomes `Some` containing that item.
///
/// This collector corresponds to [`Iterator::find()`].
///
/// [`Break(())`]: std::ops::ControlFlow::Break
/// [`Output`]: CollectorBase::Output
#[deprecated(since = "0.8.0", note = "use `First::new().filter(f)` instead")]
#[derive(Clone)]
pub struct Find<T, F> {
    state: State<T, F>,
}

#[derive(Clone)]
enum State<T, F> {
    Searching(F),
    Found(T),
}

impl<T, F> Find<T, F>
where
    F: FnMut(&T) -> bool,
{
    /// Creates an intance of this collector with a given predicate.
    #[inline]
    pub const fn new(pred: F) -> Self {
        assert_collector::<_, T>(Self {
            state: State::Searching(pred),
        })
    }
}

impl<T, F> CollectorBase for Find<T, F> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        if let State::Found(item) = self.state {
            Some(item)
        } else {
            None
        }
    }

    finish_boxed_impl!();

    // Technically, we don't need to override it
    // since when this collector stops the method is useless anyway.
    // But we will have a support of `FUSED` const variable later,
    // making this neccessary.
    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if matches!(self.state, State::Found(_)) {
            0
        } else {
            request
        }
    }
}

impl<T, F> Collector<T> for Find<T, F>
where
    F: FnMut(&T) -> bool,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if let State::Searching(ref mut pred) = self.state {
            if pred(&item) {
                self.state = State::Found(item);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        } else {
            ControlFlow::Break(())
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        if let State::Searching(ref mut pred) = self.state {
            if let Some(item) = items.into_iter().find(pred) {
                self.state = State::Found(item);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        } else {
            ControlFlow::Break(())
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.state {
            State::Searching(pred) => items.into_iter().find(pred),
            State::Found(item) => Some(item),
        }
    }
}

impl<T: Debug, F> Debug for Find<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let item = if let State::Found(ref item) = self.state {
            Some(item)
        } else {
            None
        };

        f.debug_struct("Find")
            .field("found", &item)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(collector {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {},
        iter: nums.iter().copied(),
        collector: Find::new(pred),
        expected_f: |mut iter, _| {
            let res = iter.find(pred);
            (res, res.is_some())
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    fn pred(&num: &i32) -> bool {
        num >= 0
    }
}
