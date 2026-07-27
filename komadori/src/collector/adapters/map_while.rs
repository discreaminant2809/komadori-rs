use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// A collector that accumulates items as long as a predicate returns [`Some`].
///
/// This `struct` is created by [`CollectorBase::map_while()`]. See its documentation for more.
#[derive(Clone)]
pub struct MapWhile<C, P> {
    collector: C,
    pred: P,
}

impl<C, P> MapWhile<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self { collector, pred }
    }
}

impl<C, P> CollectorBase for MapWhile<C, P>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    // We don't know how many left to reserve

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request)
    }
}

impl<C, P, T, R> Collector<T> for MapWhile<C, P>
where
    C: Collector<R>,
    P: FnMut(T) -> Option<R>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if let Some(item) = (self.pred)(item) {
            self.collector.collect(item)
        } else {
            ControlFlow::Break(())
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // Be careful! The underlying collector may stop before the predicate return false.
        let mut all_some = true;
        let cf = self
            .collector
            .collect_many(items.into_iter().map_while(|item| {
                // We trust the implementation of the standard library and the collector.
                // They should short-circuit on the first false.
                let ret = (self.pred)(item);
                all_some = ret.is_some();
                ret
            }));

        if all_some { cf } else { ControlFlow::Break(()) }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.collector
            .collect_then_finish(items.into_iter().map_while(self.pred))
    }
}

impl<C, P> Debug for MapWhile<C, P>
where
    C: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapWhile")
            .field("collector", &self.collector)
            .field("pred", &std::any::type_name::<P>())
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
        collector: vec![].into_collector().take(n).map_while(f),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.map_while(f).take(n).collect();
            (res, count >= n || nums.iter().any(|&num| f(num).is_none()))
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });

    fn f(num: i32) -> Option<i32> {
        num.checked_add(i32::MAX)
    }

    // iter.clone().count() >= take_count
    // || iter.clone().any(|num| map_pred(num).is_none())
}
