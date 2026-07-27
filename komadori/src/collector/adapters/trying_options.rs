use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that sets the [`Output`] to [`None`] when
/// a [`None`] item is encountered for the first time,
/// else the underlying collector collects the `item` inside
/// [`Some(item)`](Some).
///
/// This `struct` is created by [`CollectorBase::trying_options()`].
/// See its documentation for more.
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Clone)]
pub struct TryingOptions<C> {
    collector: Option<C>,
}

impl<C> TryingOptions<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self {
            collector: Some(collector),
        }
    }
}

impl<C> CollectorBase for TryingOptions<C>
where
    C: CollectorBase,
{
    type Output = Option<C::Output>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.map(CollectorBase::finish)
    }

    // No reserve

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector
            .as_ref()
            .map_or(0, move |collector| collector.max_afford(request))
    }
}

impl<C, T> Collector<Option<T>> for TryingOptions<C>
where
    C: Collector<T>,
{
    fn collect(&mut self, item: Option<T>) -> ControlFlow<()> {
        match (&mut self.collector, item) {
            // If the underlying collector has stopped to begin with,
            // we must effectively ignore every item at all,
            // even if it's an `Err` or `None`.
            // It's to be consistent with `collect_many` and `collect_then_finish`
            (Some(collector), _) if collector.max_afford(1) == 0 => ControlFlow::Break(()),
            (Some(collector), Some(item)) => collector.collect(item),
            (None, _) => ControlFlow::Break(()),
            (collector, None) => {
                *collector = None;
                ControlFlow::Break(())
            }
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = Option<T>>) -> ControlFlow<()> {
        match &mut self.collector {
            None => ControlFlow::Break(()),
            Some(collector) => {
                let mut any_none = false;
                let cf = collector.collect_many(items.into_iter().map_while(|item| {
                    any_none |= item.is_none();
                    item
                }));

                if any_none {
                    self.collector = None;
                    ControlFlow::Break(())
                } else {
                    cf
                }
            }
        }
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = Option<T>>) -> Self::Output {
        let mut collector = self.collector?;

        let mut any_none = false;
        let _ = collector.collect_many(items.into_iter().map_while(|item| {
            any_none |= item.is_none();
            item
        }));

        (!any_none).then(|| collector.finish())
    }

    // No override for `assume_reserved_collect()` because we don't override `reserve()`.
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::super::take_collector_model;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(prop_opt5050(any::<i32>()), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![].into_collector().take(n).trying_options(),
        expected_f: |iter, count| {
            let res: Option<Vec<_>> = iter.take(n).collect();
            (res, count >= n || nums.iter().any(|num| num.is_none()))
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
