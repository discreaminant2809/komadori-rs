use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

/// A collector that accumulates items as long as a predicate returns `true`.
///
/// This `struct` is created by [`CollectorBase::take_while()`]. See its documentation for more.
#[derive(Clone)]
pub struct TakeWhile<C, F> {
    collector: C,
    pred: F,
}

impl<C, F> TakeWhile<C, F> {
    pub(in crate::collector) fn new(collector: C, pred: F) -> Self {
        Self { collector, pred }
    }
}

impl<C, F> CollectorBase for TakeWhile<C, F>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    finish_boxed_impl!();

    // We don't know how many left to reserve

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request)
    }
}

impl<C, T, F> Collector<T> for TakeWhile<C, F>
where
    C: Collector<T>,
    F: FnMut(&T) -> bool,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if (self.pred)(&item) {
            self.collector.collect(item)
        } else {
            ControlFlow::Break(())
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // Be careful! The underlying collector may stop before the predicate return false.
        let mut all_true = true;
        let cf = self
            .collector
            .collect_many(items.into_iter().take_while(|item| {
                // We trust the implementation of the standard library and the collector.
                // They should short-circuit on the first false.
                all_true = (self.pred)(item);
                all_true
            }));

        if all_true { cf } else { ControlFlow::Break(()) }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().take_while(self.pred))
    }
}

impl<C: Debug, F> Debug for TakeWhile<C, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TakeWhile")
            .field("collector", &self.collector)
            .field("pred", &std::any::type_name::<F>())
            .finish()
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
        collector: vec![].into_collector().take(n).take_while(pred),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.take_while(pred).take(n).collect();
            (res, count >= n || !nums.iter().all(pred))
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });

    fn pred(&num: &i32) -> bool {
        num >= 0
    }

    // iter.clone().count() >= take_count || !iter.clone().all(|num| pred(&num))
}
