use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

/// A collector that feeds the underlying collector with the current count
/// alongside with the item.
///
/// This `struct` is created by [`CollectorBase::enumerate()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Enumerate<C> {
    collector: C,
    idx: usize,
}

impl<C> Enumerate<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self { collector, idx: 0 }
    }
}

impl<C> CollectorBase for Enumerate<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    finish_boxed_impl!();

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.collector.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request)
    }
}

impl<C, T> Collector<T> for Enumerate<C>
where
    C: Collector<(usize, T)>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        let idx = self.idx;
        self.idx += 1;
        self.collector.collect((idx, item))
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        let idx = self.idx;
        self.idx += 1;
        // SAFETY: The caller has reserved at least 1 item.
        unsafe { self.collector.assume_reserved_collect((idx, item)) }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collector.collect_many(
            // Be careful! We have to `zip(items, indices)`, not `zip(indices, items)`.
            // the iterator will pull out one index prematurely even tho `items` are exhausted,
            // skipping one index for the next call of collect-related method!
            items
                .into_iter()
                .zip(std::iter::repeat_with(|| {
                    let idx = self.idx;
                    self.idx += 1;
                    idx
                }))
                .map(|(item, idx)| (idx, item)),
        )
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        // This is fine, unlike `collect_many()`.
        // We get rid of the collector anyway!
        self.collector.collect_then_finish((self.idx..).zip(items))
    }
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
            let n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![].into_collector().take(n).enumerate(),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.enumerate().take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
