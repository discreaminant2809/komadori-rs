use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, break_hint};

/// A collector that skips the first `n` collected items before it begins
/// accumulating them.
///
/// This `struct` is created by [`CollectorBase::skip()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Skip<C> {
    collector: C,
    remaining: usize,
}

impl<C> Skip<C> {
    pub(in crate::collector) fn new(collector: C, n: usize) -> Self {
        Self {
            collector,
            remaining: n,
        }
    }
}

impl<C> CollectorBase for Skip<C>
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
        self.collector
            .reserve(additional.saturating_sub(self.remaining));
    }

    fn max_afford(&self, request: usize) -> usize {
        // We make sure that `self.collector.max_afford()` is only called once.
        if request > self.remaining {
            let max_afford = self.collector.max_afford(request - self.remaining);
            if max_afford == 0 {
                0
            } else {
                self.remaining + max_afford
            }
        } else if self.collector.max_afford(1) == 0 {
            0
        } else {
            request
        }
    }
}

impl<C, T> Collector<T> for Skip<C>
where
    C: Collector<T>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if self.remaining == 0 {
            return self.collector.collect(item);
        }

        self.remaining -= 1;
        break_hint(&self.collector)
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // Unlike `Collector::take()`, a guard is needed because we drop
        // items (via `drop_n_items`) before forwarding to the underlying collector.
        if self.max_afford(1) == 0 {
            return ControlFlow::Break(());
        }

        let mut items = items.into_iter();
        let (lower_sh, _) = items.size_hint();

        if self.remaining <= lower_sh {
            items
                .by_ref()
                .take(std::mem::take(&mut self.remaining))
                .try_for_each(|_| break_hint(&self.collector))?;

            return self.collector.collect_many(items);
        }

        self.remaining -= lower_sh;
        items
            .by_ref()
            .take(lower_sh)
            .try_for_each(|_| break_hint(&self.collector))?;

        match items.by_ref().try_for_each(|_| {
            break_hint(&self.collector).map_break(|_| ControlFlow::Break(()))?;
            self.remaining -= 1;
            if self.remaining == 0 {
                ControlFlow::Break(ControlFlow::Continue(()))
            } else {
                ControlFlow::Continue(())
            }
        }) {
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
            ControlFlow::Break(ControlFlow::Break(_)) => ControlFlow::Break(()),
            ControlFlow::Break(ControlFlow::Continue(_)) => self.collector.collect_many(items),
        }
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        if self.max_afford(1) == 0 {
            return self.collector.finish();
        }

        let mut items = items.into_iter();
        let (lower_sh, _) = items.size_hint();

        if self.remaining <= lower_sh {
            return if items
                .by_ref()
                .take(std::mem::take(&mut self.remaining))
                .try_for_each(|_| break_hint(&self.collector))
                .is_break()
            {
                self.collector.finish()
            } else {
                self.collector.collect_then_finish(items)
            };
        }

        self.remaining -= lower_sh;
        if items
            .by_ref()
            .take(lower_sh)
            .try_for_each(|_| break_hint(&self.collector))
            .is_break()
        {
            return self.collector.finish();
        }

        match items.by_ref().try_for_each(|_| {
            break_hint(&self.collector).map_break(|_| ControlFlow::Break(()))?;

            self.remaining -= 1;
            if self.remaining == 0 {
                ControlFlow::Break(ControlFlow::Continue(()))
            } else {
                ControlFlow::Continue(())
            }
        }) {
            ControlFlow::Continue(_) | ControlFlow::Break(ControlFlow::Break(_)) => {
                self.collector.finish()
            }
            ControlFlow::Break(ControlFlow::Continue(_)) => {
                self.collector.collect_then_finish(items)
            }
        }
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
            let take_n = ..=5_usize;
            let skip_n = ..=5_usize;
        },
        iter: nums1
            .iter()
            .chain(nums2.iter().filter(|&&num| num >= 0))
            .copied(),
        collector: vec![].into_collector().take(take_n).skip(skip_n),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.skip(skip_n).take(take_n).collect();
            (res, take_n == 0 || count >= take_n + skip_n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(if take_n == 0 { 0 } else { take_n + skip_n }),
    });
}
