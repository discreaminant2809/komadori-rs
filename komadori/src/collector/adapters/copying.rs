use crate::collector::{Collector, CollectorBase};

use std::ops::ControlFlow;

/// A collector that copies every collected item.
///
/// This `struct` is created by [`CollectorBase::copying()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Copying<C>(C);

impl<C> Copying<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self(collector)
    }
}

impl<C> CollectorBase for Copying<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0.finish()
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.0.max_afford(request)
    }
}

impl<'a, C, T> Collector<&'a T> for Copying<C>
where
    C: Collector<T>,
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &item: &'a T) -> ControlFlow<()> {
        self.0.collect(item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, &item: &'a T) -> ControlFlow<()> {
        // SAFETY: The caller has reserved at least 1 item.
        unsafe { self.0.assume_reserved_collect(item) }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'a T>) -> ControlFlow<()> {
        self.0.collect_many(items.into_iter().cloned())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = &'a T>) -> Self::Output {
        self.0.collect_then_finish(items.into_iter().cloned())
    }
}

impl<'a, C, T> Collector<&'a mut T> for Copying<C>
where
    C: Collector<T>,
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &mut item: &'a mut T) -> ControlFlow<()> {
        self.0.collect(item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, &mut item: &'a mut T) -> ControlFlow<()> {
        // SAFETY: The caller has reserved at least 1 item.
        unsafe { self.0.assume_reserved_collect(item) }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'a mut T>) -> ControlFlow<()> {
        self.0.collect_many(items.into_iter().map(|&mut item| item))
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = &'a mut T>) -> Self::Output {
        self.0
            .collect_then_finish(items.into_iter().map(|&mut item| item))
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
        collector: vec![].into_collector().take(n).copying(),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.copied().take(n).collect();
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
        collector: vec![].into_collector().take(n).copying(),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.map(|&mut num| num).take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
