use core::ops::ControlFlow;

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
        // Put this here because if the index is `usize::MAX`
        // and it's the last item the underlying can afford,
        // we should still be able to collect it and exit early
        // instead of panicking (in debug build).
        self.collector.collect((self.idx, item))?;
        self.idx += 1;
        ControlFlow::Continue(())
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: The caller reserved for at least 1 item.
            self.collector.assume_reserved_collect((self.idx, item))?;
        }

        self.idx += 1;
        ControlFlow::Continue(())
    }

    // We can't meaningfully override the other two methods,
    // because we need to uphold the "the index is `usize::MAX` and the last item"
    // case, which would lead us to a manual `try_fold()`,
    // which is the default implementation.
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
            let (n, start) = prop_oneof![..=2_usize, usize::MAX - 2..]
                .prop_flat_map(|start| (..=(usize::MAX - start).min(5), Just(start)));
        },
        iter: nums.iter().copied(),
        collector: {
            let mut collector = vec![].into_collector().take(n).enumerate();
            collector.idx = start;
            collector
        },
        expected_f: |iter, count| {
            let mut idx = start;

            let res: Vec<_> = iter
                .zip(core::iter::repeat_with(|| {
                    let old_idx = idx;
                    idx += 1;
                    old_idx
                }))
                .map(|(num, i)| (i, num))
                .take(n)
                .collect();

            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
