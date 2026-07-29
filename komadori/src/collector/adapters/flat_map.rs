use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

/// A collector that collects elements in each iterator item provided by a closure.
///
/// This `struct` is created by [`CollectorBase::flat_map()`]. See its documentation for more.
#[derive(Clone)]
pub struct FlatMap<C, F> {
    collector: C,
    f: F,
}

impl<C, F> FlatMap<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self { collector, f }
    }
}

impl<C, F> CollectorBase for FlatMap<C, F>
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
            // empty iterators!
            request
        }
    }
}

impl<C, T, I, F> Collector<T> for FlatMap<C, F>
where
    C: Collector<I::Item>,
    F: FnMut(T) -> I,
    I: IntoIterator,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.collector.collect_many((self.f)(item))
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collector
            .collect_many(items.into_iter().flat_map(&mut self.f))
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().flat_map(self.f))
    }
}

impl<C: Debug, F> Debug for FlatMap<C, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlatMap")
            .field("collector", &self.collector)
            .field("f", &std::any::type_name::<F>())
            .finish()
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
        collector: vec![].into_collector().take(n).flat_map(f),
        expected_f: |iter, _| {
            let res: Vec<_> = iter.flat_map(f).take(n).collect();
            (res, nums.iter().flatten().count() >= n)
        },
        output_pred: PartialEq::eq,
        model: CollectorModel {
            state: n,
            advance_f: |n: &mut usize, item: &Vec<_>| *n = n.saturating_sub(item.len()),
            max_afford_f: |&n: &_, request| if n == 0 { n } else { request },
        },
    });

    #[allow(clippy::ptr_arg)]
    fn f(nums: &Vec<i32>) -> impl Iterator<Item = i32> {
        nums.iter().copied()
    }
}
