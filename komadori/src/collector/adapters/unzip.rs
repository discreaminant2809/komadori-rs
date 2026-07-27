use std::ops::ControlFlow;

use crate::collector::{
    Collector, CollectorBase, Fuse, advanced_collect_many_default_impl, and_break,
};

/// A collector that destructures each 2-tuple `(A, B)` item and distributes its fields:
/// `A` goes to the first collector, and `B` goes to the second collector.
///
/// This `struct` is created by [`CollectorBase::unzip()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Unzip<C1, C2> {
    // `Fuse` is neccessary since either may end earlier.
    // It can ease the implementation.
    collector1: Fuse<C1>,
    collector2: Fuse<C2>,
}

impl<C1, C2> Unzip<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            collector1: Fuse::new(collector1),
            collector2: Fuse::new(collector2),
        }
    }
}

impl<C1, C2> CollectorBase for Unzip<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    type Output = (C1::Output, C2::Output);

    fn finish(self) -> Self::Output {
        (self.collector1.finish(), self.collector2.finish())
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.collector1.reserve(additional);
        self.collector2.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        // `max`, not `min`.
        // Even if one stops, the other still proceeds.
        self.collector1
            .max_afford(request)
            .max(self.collector2.max_afford(request))
    }
}

impl<C1, C2, T1, T2> Collector<(T1, T2)> for Unzip<C1, C2>
where
    C1: Collector<T1>,
    C2: Collector<T2>,
{
    #[inline]
    fn collect(&mut self, (item1, item2): (T1, T2)) -> ControlFlow<()> {
        let cf1 = self.collector1.collect(item1);
        let cf2 = self.collector2.collect(item2);
        and_break(cf1, cf2)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, (item1, item2): (T1, T2)) -> ControlFlow<()> {
        // SAFETY: The caller has reserved at least 1 item.
        let cf1 = unsafe { self.collector1.assume_reserved_collect(item1) };
        // SAFETY: The caller has reserved at least 1 item.
        let cf2 = unsafe { self.collector2.assume_reserved_collect(item2) };

        and_break(cf1, cf2)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = (T1, T2)>) -> ControlFlow<()> {
        advanced_collect_many_default_impl(self, items)
    }

    // No meaningful override for this method.
    // fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {}
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
        iter: nums.iter().map(|&num| (num, num)),
        collector: vec![]
            .into_collector()
            .take(first_n)
            .unzip(vec![].into_collector().take(second_n)),
        expected_f: |mut iter, count| {
            let max_n = first_n.max(second_n);
            let min_n = first_n.min(second_n);

            let (mut first, mut second): (Vec<_>, Vec<_>) = iter.by_ref().take(min_n).collect();

            if first_n < second_n {
                second.extend(iter.map(|(_, num)| num).take(max_n - min_n));
            } else {
                first.extend(iter.map(|(num, _)| num).take(max_n - min_n));
            };

            ((first, second), count >= first_n.max(second_n))
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(first_n.max(second_n)),
    });
}
