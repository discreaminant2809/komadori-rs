use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

use super::{DefinePassDown, TeeBase, Teer};

/// A collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`CollectorBase::tee_funnel()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct TeeFunnel<C1, C2> {
    base: TeeBase<C1, C2, FunnelTeer>,
}

impl<C1, C2> TeeFunnel<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            base: TeeBase::new(collector1, collector2, FunnelTeer),
        }
    }
}

impl<C1, C2> CollectorBase for TeeFunnel<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    type Output = (C1::Output, C2::Output);

    #[inline]
    fn finish(self) -> Self::Output {
        self.base.finish()
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.base.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.base.max_afford(request)
    }
}

impl<T, C1, C2> Collector<T> for TeeFunnel<C1, C2>
where
    C1: for<'a> Collector<&'a mut T>,
    C2: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.base.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.base.collect_many(items)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.base.collect_then_finish(items)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        // SAFETY: `TeeBase` alerady handles the invariants.
        unsafe { self.base.assume_reserved_collect(item) }
    }
}

#[derive(Debug, Clone)]
struct FunnelTeer;

impl<'a, T> DefinePassDown<'a, T> for FunnelTeer {
    type PassDown = &'a mut T;
}

impl<T> Teer<T> for FunnelTeer {
    #[inline]
    fn pass_down<'a>(&mut self, item: &'a mut T) -> &'a mut T {
        item
    }

    // Cannot meaningfully override anything else.
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::super::take_collector_model;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let first_n = ..=5_usize;
            let second_n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![]
            .into_collector()
            .take(first_n)
            .tee_funnel(vec![].into_collector().take(second_n)),
        expected_f: |mut iter, count| {
            let max_n = first_n.max(second_n);
            let min_n = first_n.min(second_n);

            let (mut first, mut second): (Vec<_>, Vec<_>) =
                iter.by_ref().take(min_n).map(|num| (num, num)).collect();

            if first_n < second_n {
                second.extend(iter.take(max_n - min_n));
            } else {
                first.extend(iter.take(max_n - min_n));
            };

            ((first, second), count >= first_n.max(second_n))
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(first_n.max(second_n)),
    });
}
