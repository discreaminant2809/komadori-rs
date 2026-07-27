use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that flattens items by one level of nesting before collecting.
///
/// This `struct` is created by [`CollectorBase::flatten()`]. See its documentation for more.
#[derive(Clone, Debug)]
pub struct Flatten<C> {
    collector: C,
}

impl<C> Flatten<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self { collector }
    }
}

impl<C> CollectorBase for Flatten<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    // We don't know how many exactly to reserve.

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.collector.max_afford(request) == 0 {
            0
        } else {
            // There can also be the case that we're only fed
            // empty iterators!
            request
        }
    }
}

impl<C, I> Collector<I> for Flatten<C>
where
    C: Collector<I::Item>,
    I: IntoIterator,
{
    #[inline]
    fn collect(&mut self, item: I) -> ControlFlow<()> {
        self.collector.collect_many(item)
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = I>) -> ControlFlow<()> {
        self.collector.collect_many(items.into_iter().flatten())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = I>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().flatten())
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(propvec(any::<i32>(), ..=2), ..=3);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter(),
        collector: vec![].into_collector().take(n).copying().flatten(),
        expected_f: |iter, _| {
            let res: Vec<_> = iter.flatten().copied().take(n).collect();
            (res, nums.iter().flatten().count() >= n)
        },
        output_pred: PartialEq::eq,
        model: CollectorModel {
            state: n,
            advance_f: |n: &mut usize, item: &Vec<_>| *n = n.saturating_sub(item.len()),
            max_afford_f: |&n: &_, request| if n == 0 { n } else { request },
        },
    });
}
