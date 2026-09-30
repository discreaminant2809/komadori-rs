use core::ops::ControlFlow;

use crate::collector::{
    Collector, CollectorBase, advanced_collect_many_default_impl, finish_boxed_impl,
};

/// A collector that feeds the underlying collector with
/// the mutable reference to the item, "pretending" the collector
/// accepts owned items.
///
/// This `struct` is created by [`CollectorBase::funnel()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct Funnel<C>(C);

impl<C> Funnel<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self(collector)
    }
}

impl<C> CollectorBase for Funnel<C>
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

impl<C, T> Collector<T> for Funnel<C>
where
    C: for<'a> Collector<&'a mut T>,
{
    #[inline]
    fn collect(&mut self, mut item: T) -> ControlFlow<()> {
        self.0.collect(&mut item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, mut item: T) -> ControlFlow<()> {
        // SAFETY: The caller has reserved at least 1 item.
        unsafe { self.0.assume_reserved_collect(&mut item) }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        advanced_collect_many_default_impl(self, items)
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
        collector: vec![].into_collector().take(n).funnel(),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
