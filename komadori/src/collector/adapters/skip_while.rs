use core::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, break_hint, finish_boxed_impl};

/// A collector that skips the first collected items that satisfy a predicate
/// before accumulating.
///
/// This `struct` is created by [`CollectorBase::skip_while()`]. See its documentation for more.
#[derive(Clone)]
pub struct SkipWhile<C, P> {
    collector: C,
    pred: Option<P>,
}

impl<C, P> SkipWhile<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self {
            collector,
            pred: Some(pred),
        }
    }
}

impl<C, P> CollectorBase for SkipWhile<C, P>
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
        if self.pred.is_none() {
            self.collector.reserve(additional);
        }
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        let max_afford = self.collector.max_afford(request);

        if max_afford == 0 {
            0
        } else if self.pred.is_some() {
            request
        } else {
            max_afford
        }
    }
}

impl<C, P, T> Collector<T> for SkipWhile<C, P>
where
    C: Collector<T>,
    P: FnMut(&T) -> bool,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if self.pred.as_mut().is_some_and(|pred| pred(&item)) {
            break_hint(&self.collector)
        } else {
            self.pred.take();
            self.collector.collect(item)
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        let Some(pred) = &mut self.pred else {
            return self.collector.collect_many(items);
        };

        // Edge case:
        break_hint(&self.collector)?;

        let mut items = items.into_iter();
        match items.by_ref().try_for_each({
            let collector = &mut self.collector;
            move |item| {
                let skipping = pred(&item);
                break_hint(collector).map_break(|_| None)?;
                if skipping {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(Some(item))
                }
            }
        }) {
            // We've already checked for the break hint in the previous iteration.
            // We may not need to check anymore
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
            ControlFlow::Break(None) => ControlFlow::Break(()),
            ControlFlow::Break(Some(first)) => {
                self.pred.take();
                self.collector.collect(first)?;
                self.collector.collect_many(items)
            }
        }
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let Some(mut pred) = self.pred else {
            return self.collector.collect_then_finish(items);
        };

        // Edge case:
        if self.collector.max_afford(1) == 0 {
            return self.collector.finish();
        }

        let mut items = items.into_iter();
        match items.by_ref().try_for_each({
            let collector = &mut self.collector;
            move |item| {
                let skipping = pred(&item);
                break_hint(collector).map_break(|_| None)?;
                if skipping {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(Some(item))
                }
            }
        }) {
            ControlFlow::Continue(_) | ControlFlow::Break(None) => self.collector.finish(),
            ControlFlow::Break(Some(first)) => {
                if self.collector.collect(first).is_break() {
                    self.collector.finish()
                } else {
                    self.collector.collect_then_finish(items)
                }
            }
        }
    }
}

impl<C, P> Debug for SkipWhile<C, P>
where
    C: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SkipWhile")
            .field("collector", &self.collector)
            .field("pred", &core::any::type_name::<P>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::{mem::Dropping, test_utils::prelude::*};

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![].into_collector().take(n).skip_while(pred),
        expected_f: |iter, _| {
            let res: Vec<_> = iter.skip_while(pred).take(n).collect();
            (
                res,
                Dropping
                    .take(n)
                    .collect_many(nums.iter().copied().skip_while(pred))
                    .is_break(),
            )
        },
        output_pred: PartialEq::eq,
        model: CollectorModel {
            state: State { n, skipping: true },
            advance_f: |state: &mut State, item| {
                if !state.skipping || !pred(&item) {
                    state.skipping = false;
                    state.n = state.n.saturating_sub(1);
                }
            },
            max_afford_f: |state: &State, request| {
                if state.n == 0 {
                    0
                } else if state.skipping {
                    request
                } else {
                    state.n.min(request)
                }
            },
        },
    });

    fn pred(&num: &i32) -> bool {
        num < 0
    }

    struct State {
        n: usize,
        skipping: bool,
    }
}
