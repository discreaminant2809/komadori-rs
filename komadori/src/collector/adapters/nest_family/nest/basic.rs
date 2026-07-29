use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

use super::{super::strategy::CloneStrategy, with_strategy::WithStrategy};

/// A collector that collects all outputs produced by an inner collector.
///
/// This `struct` is created by [`CollectorBase::nest()`]. See its documentation for more.
// Needed because the "Available on crate feature" does not show up on doc.rs
#[cfg_attr(docsrs, doc(cfg(feature = "unstable")))]
#[derive(Clone)]
pub struct Nest<CO, CI>(WithStrategy<CO, CloneStrategy<CI>>)
where
    CI: CollectorBase + Clone;

impl<CO, CI> Nest<CO, CI>
where
    CO: CollectorBase,
    CI: CollectorBase + Clone,
{
    pub(in crate::collector) fn new(outer: CO, inner: CI) -> Self {
        Self(WithStrategy::new(outer, CloneStrategy::new(inner)))
    }
}

impl<CO, CI> CollectorBase for Nest<CO, CI>
where
    CO: Collector<CI::Output>,
    CI: CollectorBase + Clone,
{
    type Output = CO::Output;

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

impl<CO, CI, T> Collector<T> for Nest<CO, CI>
where
    CO: Collector<CI::Output>,
    CI: Collector<T> + Clone,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.0.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.0.collect_many(items)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.0.collect_then_finish(items)
    }
}

impl<CO, CI> Debug for Nest<CO, CI>
where
    CO: Debug,
    CI: CollectorBase + Clone + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("Nest");
        self.0.debug_struct(&mut debug_struct);
        debug_struct.finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use std::ops::Range;

    use crate::test_utils::prelude::*;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=10);
        },
        other_data: {
            let row = ..=3_usize;
            // FIXME: For 0 columns, for now the collector will consume one item prematurely.
            // This will get fixed in the future, but for now we don't know
            // if the API surface is even good enough.
            let column = 1..=3_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![]
            .into_collector()
            .take(row)
            .nest(vec![].into_collector().take(column)),
        expected_f: |mut iter, _| {
            let res: Vec<_> = std::iter::from_fn(move || {
                let column = iter.by_ref().take(column).collect::<Vec<_>>();
                (!column.is_empty()).then_some(column)
            })
            .take(row)
            .collect();

            let should_break = res.len() == row && res.last().is_none_or(|row| row.len() == column);

            (res, should_break)
        },
        output_pred: PartialEq::eq,
        model: CollectorModel {
            state: (0..row, 0_usize),
            advance_f: |(rows, j): &mut (Range<usize>, _), _| {
                while *j >= column && rows.next().is_some() {
                    *j = 0;
                }

                *j += 1
            },
            max_afford_f: |(rows, _): &(Range<usize>, _), request| {
                if rows.is_empty() { 0 } else { request }
            }
        },
    });
}
