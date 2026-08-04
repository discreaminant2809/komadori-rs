use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

use core::ops::ControlFlow;

/// A collector that [`clone`](Clone::clone)s every collected item.
///
/// This `struct` is created by [`CollectorBase::cloning()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Cloning<C>(C);

impl<C> Cloning<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self(collector)
    }
}

impl<C> CollectorBase for Cloning<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0.finish()
    }

    finish_boxed_impl!();

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.0.max_afford(request)
    }
}

impl<'a, C, T> Collector<&'a T> for Cloning<C>
where
    C: Collector<T>,
    T: Clone,
{
    #[inline]
    fn collect(&mut self, item: &'a T) -> ControlFlow<()> {
        self.0.collect(item.clone())
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: &'a T) -> ControlFlow<()> {
        // SAFETY: The caller has reserved at least 1 item.
        unsafe { self.0.assume_reserved_collect(item.clone()) }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'a T>) -> ControlFlow<()> {
        self.0.collect_many(items.into_iter().cloned())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = &'a T>) -> Self::Output {
        self.0.collect_then_finish(items.into_iter().cloned())
    }
}

impl<'a, C, T> Collector<&'a mut T> for Cloning<C>
where
    C: Collector<T>,
    T: Clone,
{
    #[inline]
    fn collect(&mut self, item: &'a mut T) -> ControlFlow<()> {
        self.0.collect(item.clone())
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: &'a mut T) -> ControlFlow<()> {
        // SAFETY: The caller has reserved at least 1 item.
        unsafe { self.0.assume_reserved_collect(item.clone()) }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'a mut T>) -> ControlFlow<()> {
        self.0
            .collect_many(items.into_iter().map(|item| item.clone()))
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = &'a mut T>) -> Self::Output {
        self.0
            .collect_then_finish(items.into_iter().map(|item| item.clone()))
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::super::take_collector_model;

    collector_test!(adapter_ref {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter(),
        collector: vec![].into_collector().take(n).cloning(),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.cloned().take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });

    collector_test!(adapter_mut {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iters: {
            let mut other_nums = nums.repeat(2);
            let (for_output, for_model) = other_nums.split_at_mut(nums.len());
            (nums.iter_mut(), for_output.iter_mut(), for_model.iter_mut())
        },
        collector: vec![].into_collector().take(n).cloning(),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.map(|&mut num| num).take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
