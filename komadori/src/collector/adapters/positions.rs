use core::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, break_hint, finish_boxed_impl};

/// A collector that feeds the underlying collector
/// with indices of items that satisfy a predicate.
///
/// This `struct` is created by [`CollectorBase::positions()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Positions<C, P> {
    collector: C,
    pred: P,
    idx: usize,
}

impl<C, P> Positions<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self {
            collector,
            pred,
            idx: 0,
        }
    }
}

impl<C, P> CollectorBase for Positions<C, P>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    finish_boxed_impl! {}

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.collector.max_afford(request) > 0 {
            request
        } else {
            0
        }
    }
}

impl<C, P, T> Collector<T> for Positions<C, P>
where
    C: Collector<usize>,
    P: FnMut(T) -> bool,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        // If the index is `usize::MAX` and it's the last index the underlying can afford,
        // we should still be able to collect it and exit early
        // instead of panicking (in debug build).

        if (self.pred)(item) {
            self.collector.collect(self.idx)
        } else {
            break_hint(&self.collector)
        }?;

        self.idx += 1;
        ControlFlow::Continue(())
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use core::convert::identity;

    use crate::test_utils::prelude::*;

    use super::super::take_collector_model_filtered;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<bool>(), ..=10);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![].into_collector().take(n).positions(identity),
        expected_f: |iter, _| {
            let res: Vec<_> = iter.positions(identity).take(n).collect();
            (res, nums.iter().copied().positions(identity).count() >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model_filtered(n, identity),
    });
}
