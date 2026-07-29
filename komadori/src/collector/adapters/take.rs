use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

/// A collector that stops accumulating after collecting the first `n` items.
///
/// This `struct` is created by [`CollectorBase::take()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Take<C> {
    collector: C,
    // Unspecified if the underlying collector stops accumulating.
    remaining: usize,
}

impl<C> Take<C> {
    pub(in crate::collector) fn new(collector: C, n: usize) -> Self {
        Self {
            collector,
            remaining: n,
        }
    }
}

impl<C> CollectorBase for Take<C>
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
        self.collector.reserve(self.remaining.min(additional));
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request).min(self.remaining)
    }
}

impl<C, T> Collector<T> for Take<C>
where
    C: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        // Must NOT remove it. The user may construct with `take(0)` and
        // because it hasn't yielded Break, it shouldn't panic!
        if self.remaining == 0 {
            return ControlFlow::Break(());
        }

        self.remaining -= 1;
        let cf = self.collector.collect(item);

        if self.remaining == 0 {
            ControlFlow::Break(())
        } else {
            cf
        }
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        // Must NOT remove it. The user may construct with `take(0)` and
        // because it hasn't yielded Break, it shouldn't panic!
        if self.remaining == 0 {
            return ControlFlow::Break(());
        }

        self.remaining -= 1;
        let cf = unsafe {
            // SAFETY: Since `remaining` > 0, we've reserved at least 1 item.
            self.collector.assume_reserved_collect(item)
        };

        if self.remaining == 0 {
            ControlFlow::Break(())
        } else {
            cf
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // FIXED: utilize specialization after it's stabilized.

        let mut items = items.into_iter();
        let (lower_sh, _) = items.size_hint();

        // Implementation note: we trust the iterator's hint, but only for safe code.
        // No reserve calls and unsafe code for now because
        // we don't use `assume_reserved_collect()`,
        // but it's worth keeping this in mind.

        // The collector may end early. We risk tracking the state wrong?
        // Worry not. By then, the `remaining` becomes useless
        // and acts as a *soft* fuse.
        if self.remaining <= lower_sh {
            let n = std::mem::take(&mut self.remaining);
            let _ = self.collector.collect_many(items.take(n));
            return ControlFlow::Break(());
        }

        self.remaining -= lower_sh;
        self.collector.collect_many(items.by_ref().take(lower_sh))?;

        // We don't know how many left after the lower bound,
        // so we carefully track the state with `inspect`.
        let cf = self.collector.collect_many(
            items
                .take(self.remaining)
                // Since the collector may not collect all `remaining` items
                .inspect(|_| self.remaining -= 1),
        );

        if self.remaining == 0 {
            ControlFlow::Break(())
        } else {
            cf
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        // No need to track the state anymore. We'll be gone!
        self.collector
            .collect_then_finish(items.into_iter().take(self.remaining))
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::super::take_collector_model;

    collector_test!(adapter {
        iter_data: {
            let mut nums1 = propvec(any::<i32>(), ..=3);
            let mut nums2 = propvec(any::<i32>(), ..=3);
        },
        other_data: {
            let first_n = ..=3_usize;
            let second_n = ..=3_usize;
        },
        iter: nums1
            .iter()
            .chain(nums2.iter().filter(|&&num| num >= 0))
            .copied(),
        collector: vec![].into_collector().take(first_n).take(second_n),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.take(first_n.min(second_n)).collect();
            (res, count >= first_n.min(second_n))
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(first_n.min(second_n)),
    });
}
