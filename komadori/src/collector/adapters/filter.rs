use crate::collector::{Collector, CollectorBase, break_hint, finish_boxed_impl};

use core::{fmt::Debug, ops::ControlFlow};

/// A collector that uses a closure to determine whether an item should be collected.
///
/// This `struct` is created by [`CollectorBase::filter()`]. See its documentation for more.
#[derive(Clone)]
pub struct Filter<C, F> {
    collector: C,
    pred: F,
}

impl<C, F> Filter<C, F> {
    pub(in crate::collector) fn new(collector: C, pred: F) -> Self {
        Self { collector, pred }
    }
}

impl<C, F> CollectorBase for Filter<C, F>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    finish_boxed_impl!();

    // We don't know how many exactly to reserve.

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.collector.max_afford(request) == 0 {
            0
        } else {
            // There can also be the case that we're only fed
            // items that make the predicate false!
            request
        }
    }
}

impl<C, F, T> Collector<T> for Filter<C, F>
where
    C: Collector<T>,
    F: FnMut(&T) -> bool,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if (self.pred)(&item) {
            self.collector.collect(item)
        } else {
            break_hint(&self.collector)
        }
    }

    // Removed the overriden implementations cuz the items here are being consumed
    // without consulting the underlying collector's break hint during filtering.
    // Yes, the performance degrades, but it's because of `try_for_each()` and/or
    // LLVM noise (which could be fixed soon),
    // and in multiple reduction it still works well and performs similarly to fold().
}

impl<C: Debug, F> Debug for Filter<C, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Filter")
            .field("collector", &self.collector)
            .field("pred", &core::any::type_name::<F>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::super::take_collector_model_filtered;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![].into_collector().take(n).filter(pred),
        expected_f: |iter, _| {
            let res: Vec<_> = iter.filter(pred).take(n).collect();
            (res, nums.iter().copied().filter(pred).count() >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model_filtered(n, |num| pred(&num)),
    });

    fn pred(&n: &i32) -> bool {
        n >= 0
    }
}
