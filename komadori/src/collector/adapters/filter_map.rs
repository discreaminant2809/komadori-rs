use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, break_hint, finish_boxed_impl};

/// A collector that both filters and maps each item before collecting.
///
/// This `struct` is created by [`CollectorBase::filter_map()`].
/// See its documentation for more.
#[derive(Clone)]
pub struct FilterMap<C, P> {
    collector: C,
    pred: P,
}

impl<C, P> FilterMap<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self { collector, pred }
    }
}

impl<C, P> CollectorBase for FilterMap<C, P>
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

impl<C, T, P, R> Collector<T> for FilterMap<C, P>
where
    C: Collector<R>,
    P: FnMut(T) -> Option<R>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if let Some(item) = (self.pred)(item) {
            self.collector.collect(item)
        } else {
            break_hint(&self.collector)
        }
    }

    // Removed the overriden implementations cuz the items here are being consumed
    // without consulting the underlying collector's break hint during filtering.
}

impl<C, P> Debug for FilterMap<C, P>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterMap")
            .field("collector", &self.collector)
            .field("pred", &std::any::type_name::<P>())
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
        collector: vec![].into_collector().take(n).filter_map(f),
        expected_f: |iter, _| {
            let res: Vec<_> = iter.filter_map(f).take(n).collect();
            (res, nums.iter().copied().filter_map(f).count() >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model_filtered(n, |num| f(num).is_some()),
    });

    fn f(n: i32) -> Option<i32> {
        n.checked_add(i32::MAX)
    }
}
