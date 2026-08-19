use core::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, assert_collector, finish_boxed_impl};

/// A collector that `||`s all collected `bool`, and stops when it encounters a `true`.
///
/// Its [`Output`] is `false` as long as it only collects `false`, and `true` if
/// it collects `true` once. If no items were collected, the [`Output`] is `false`.
///
/// `Or::new().map(f)` corresponds to [`Iterator::any(f)`](Iterator::any).
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, ops::Or};
///
/// let mut collector = Or::new();
///
/// assert!(collector.collect(false).is_continue());
/// assert!(collector.collect(false).is_continue());
/// assert!(collector.collect(false).is_continue());
///
/// assert!(!collector.finish());
/// ```
///
/// ```
/// use komadori::{prelude::*, ops::Or};
///
/// let mut collector = Or::new();
///
/// assert!(collector.collect(false).is_continue());
/// assert!(collector.collect(false).is_continue());
/// assert!(collector.collect(true).is_break());
///
/// assert!(collector.finish());
/// ```
///
/// Most of the time, this collector is paired with [`map()`](CollectorBase::map):
///
/// ```
/// use komadori::{prelude::*, ops::Or};
///
/// // Any positives.
/// let mut collector = Or::new().map(|num| num > 0);
///
/// assert!(collector.collect(0).is_continue());
/// assert!(collector.collect(0).is_continue());
/// assert!(collector.collect(7).is_break());
///
/// assert!(collector.finish());
/// ```
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Clone)]
pub struct Or(bool);

impl Or {
    /// Creates a new instance of this collector with an initial
    /// [`Output`](CollectorBase::Output) of `false`.
    #[inline]
    pub const fn new() -> Self {
        assert_collector::<_, bool>(Self(false))
    }
}

// Try to be explicit.
impl Default for Or {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl CollectorBase for Or {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }

    finish_boxed_impl! {}

    #[cfg(debug_assertions)]
    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if !self.0 { request } else { 0 }
    }
}

impl Collector<bool> for Or {
    // Same as `First`.

    #[inline]
    fn collect(&mut self, item: bool) -> ControlFlow<()> {
        debug_assert!(
            !self.0,
            "`collect`-related methods called after `Break` was returned"
        );

        self.0 = item;
        bool_to_cf(self.0)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = bool>) -> ControlFlow<()> {
        debug_assert!(
            !self.0,
            "`collect`-related methods called after `Break` was returned"
        );

        self.0 = items.into_iter().try_for_each(bool_to_cf).is_break();
        bool_to_cf(self.0)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = bool>) -> Self::Output {
        debug_assert!(
            !self.0,
            "`collect`-related methods called after `Break` was returned"
        );

        items.into_iter().try_for_each(bool_to_cf).is_break()
    }
}

#[inline]
fn bool_to_cf(b: bool) -> ControlFlow<()> {
    if b {
        ControlFlow::Break(())
    } else {
        ControlFlow::Continue(())
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
        collector: Or::new(),
        expected_f: |mut iter, _| {
            let res = iter.any(|b| b);
            (res, res)
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });
}
