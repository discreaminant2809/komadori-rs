use core::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, assert_collector, finish_boxed_impl};

/// A collector that `&&`s all collected `bool`, and stops when it encounters a `false`.
///
/// Its [`Output`] is `true` as long as it only collects `true`, and `false` if
/// it collects `false` once. If no items were collected, the [`Output`] is `true`.
///
/// `And::new().map(f)` corresponds to [`Iterator::all(f)`](Iterator::all).
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, ops::And};
///
/// let mut collector = And::new();
///
/// assert!(collector.collect(true).is_continue());
/// assert!(collector.collect(true).is_continue());
/// assert!(collector.collect(true).is_continue());
///
/// assert!(collector.finish());
/// ```
///
/// ```
/// use komadori::{prelude::*, ops::And};
///
/// let mut collector = And::new();
///
/// assert!(collector.collect(true).is_continue());
/// assert!(collector.collect(true).is_continue());
/// assert!(collector.collect(false).is_break());
///
/// assert!(!collector.finish());
/// ```
///
/// Most of the time, this collector is paired with [`map()`](CollectorBase::map):
///
/// ```
/// use komadori::{prelude::*, ops::And};
///
/// // All positives.
/// let mut collector = And::new().map(|num| num > 0);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert!(collector.finish());
/// ```
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Default, Clone)]
pub struct And(bool);

impl And {
    /// Creates a new instance of this collector with an initial
    /// [`Output`](CollectorBase::Output) of `true`.
    #[inline]
    pub const fn new() -> Self {
        assert_collector::<_, bool>(Self(true))
    }
}

impl CollectorBase for And {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }

    finish_boxed_impl! {}

    #[cfg(debug_assertions)]
    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.0 { request } else { 0 }
    }
}

impl Collector<bool> for And {
    // Same as `First`.

    #[inline]
    fn collect(&mut self, item: bool) -> ControlFlow<()> {
        debug_assert!(
            self.0,
            "`collect`-related methods called after `Break` was returned"
        );

        self.0 = item;
        bool_to_cf(self.0)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = bool>) -> ControlFlow<()> {
        debug_assert!(
            self.0,
            "`collect`-related methods called after `Break` was returned"
        );

        self.0 = items.into_iter().try_for_each(bool_to_cf).is_continue();
        bool_to_cf(self.0)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = bool>) -> Self::Output {
        debug_assert!(
            self.0,
            "`collect`-related methods called after `Break` was returned"
        );

        items.into_iter().try_for_each(bool_to_cf).is_continue()
    }
}

#[inline]
fn bool_to_cf(b: bool) -> ControlFlow<()> {
    if b {
        ControlFlow::Continue(())
    } else {
        ControlFlow::Break(())
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(collector {
        iter_data: {
            let mut bools = propvec(any::<bool>(), ..=10);
        },
        other_data: {},
        iter: bools.iter().copied(),
        collector: And::new(),
        expected_f: |mut iter, _| {
            let res = iter.all(|b| b);
            (res, !res)
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });
}
