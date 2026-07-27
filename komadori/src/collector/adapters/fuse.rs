use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that can "safely" collect items even after
/// the underlying collector has stopped accumulating,
/// without triggering undesired behaviors.
///
/// This `struct` is created by [`CollectorBase::fuse()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Fuse<C> {
    collector: C,
    stopped: bool,
}

impl<C> Fuse<C>
where
    C: CollectorBase,
{
    #[inline]
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self {
            stopped: collector.max_afford(1) == 0,
            collector,
        }
    }
}

impl<C> Fuse<C>
where
    C: CollectorBase,
{
    #[inline]
    fn collect_impl(&mut self, f: impl FnOnce(&mut C) -> ControlFlow<()>) -> ControlFlow<()> {
        if self.stopped {
            ControlFlow::Break(())
        } else if f(&mut self.collector).is_continue() {
            ControlFlow::Continue(())
        } else {
            self.stopped = true;
            ControlFlow::Break(())
        }
    }
}

impl<C> CollectorBase for Fuse<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        if !self.stopped {
            self.collector.reserve(additional);
        }
    }

    #[inline]
    fn max_afford(&self, amount: usize) -> usize {
        if self.stopped {
            0
        } else {
            self.collector.max_afford(amount)
        }
    }
}

impl<C, T> Collector<T> for Fuse<C>
where
    C: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.collect_impl(|collector| collector.collect(item))
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collect_impl(|collector| collector.collect_many(items))
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        if self.stopped {
            self.finish()
        } else {
            self.collector.collect_then_finish(items)
        }
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        self.collect_impl(|collector| unsafe {
            // SAFETY: We've reserved for at least one item.
            collector.assume_reserved_collect(item)
        })
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
        collector: vec![].into_collector().take(n).fuse(),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
