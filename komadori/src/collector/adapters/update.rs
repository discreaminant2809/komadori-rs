use std::{fmt::Debug, ops::ControlFlow};

use itertools::Itertools;

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

/// A collector that calls a closure on each item before collecting.
///
/// This `struct` is created by [`CollectorBase::update()`]. See its documentation for more.
#[derive(Clone)]
pub struct Update<C, F> {
    collector: C,
    f: F,
}

impl<C, F> Update<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self { collector, f }
    }
}

impl<C, F> CollectorBase for Update<C, F>
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

impl<C, T, F> Collector<T> for Update<C, F>
where
    C: Collector<T>,
    F: FnMut(&mut T),
{
    #[inline]
    fn collect(&mut self, mut item: T) -> ControlFlow<()> {
        (self.f)(&mut item);
        self.collector.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collector
            .collect_many(items.into_iter().update(&mut self.f))
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().update(self.f))
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, mut item: T) -> ControlFlow<()> {
        (self.f)(&mut item);
        // SAFETY: The caller has reserved at least one item for us.
        unsafe { self.collector.assume_reserved_collect(item) }
    }
}

impl<C: Debug, F> Debug for Update<C, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Update")
            .field("collector", &self.collector)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use itertools::Itertools;

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
        collector: vec![].into_collector().take(n).update(f),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.update(f).take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });

    fn f(num: &mut i32) {
        *num = num.wrapping_add(i32::MAX);
    }
}
