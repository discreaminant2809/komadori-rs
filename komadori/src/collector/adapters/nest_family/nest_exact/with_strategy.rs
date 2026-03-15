use std::{
    fmt::{Debug, DebugStruct},
    iter, mem,
    ops::ControlFlow,
};

use crate::collector::{Collector, CollectorBase};

use super::super::strategy::{Strategy, StrategyBase};

#[derive(Clone)]
pub struct WithStrategy<CO, S>
where
    S: StrategyBase,
{
    outer: CO,
    strategy: S,
    inner: S::Collector,
}

impl<CO, S> WithStrategy<CO, S>
where
    S: StrategyBase,
{
    pub(super) fn new(outer: CO, mut strategy: S) -> Self {
        Self {
            outer,
            inner: strategy.next_collector(),
            strategy,
        }
    }
}

impl<CO, S> CollectorBase for WithStrategy<CO, S>
where
    CO: CollectorBase,
    S: StrategyBase,
{
    type Output = CO::Output;

    fn finish(self) -> Self::Output {
        self.outer.finish()
    }

    fn break_hint(&self) -> ControlFlow<()> {
        self.outer.break_hint()
    }
}

impl<CO, S, T> Collector<T> for WithStrategy<CO, S>
where
    CO: Collector<S::Output>,
    S: Strategy<T>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.outer.collect_many(iter::from_fn(|| {
            if self.inner.break_hint().is_break() {
                let inner = mem::replace(&mut self.inner, self.strategy.next_collector());
                Some(inner.finish())
            } else {
                None
            }
        }))?;

        if self.inner.collect(item).is_break() {
            let inner = mem::replace(&mut self.inner, self.strategy.next_collector());
            self.outer.collect(inner.finish())
        } else {
            self.outer.break_hint()
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // FIXME: specialization for the iterator.
        // It is good if we can know in advanced whether an iterator is exhausted or not.
        // For now, we trust the size hint.

        let mut items = items.into_iter();

        self.outer.collect_many(iter::from_fn(|| {
            let size_hint = items.size_hint();
            if let (0, Some(0)) = size_hint {
                return None;
            }

            if self.inner.break_hint().is_break() {
                let inner = mem::replace(&mut self.inner, self.strategy.next_collector());
                return Some(inner.finish());
            }

            // We try our best not to consume prematurely,
            // while not also causing an infinite loop.

            match size_hint {
                (1.., _) => {
                    if self.inner.collect_many(&mut items).is_break() {
                        let inner = mem::replace(&mut self.inner, self.strategy.next_collector());
                        Some(inner.finish())
                    } else {
                        None
                    }
                }
                _ => {
                    // We have no other way but to probe.
                    // We may pull one item prematurely, but that's fine.
                    // We have done the best effort.
                    let item = items.next()?;

                    if self
                        .inner
                        .collect_many(iter::once(item).chain(&mut items))
                        .is_break()
                    {
                        let inner = mem::replace(&mut self.inner, self.strategy.next_collector());
                        Some(inner.finish())
                    } else {
                        None
                    }
                }
            }
        }))
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let mut items = items.into_iter();
        let mut inner = self.inner;

        self.outer.collect_then_finish(iter::from_fn(move || {
            let size_hint = items.size_hint();
            if let (0, Some(0)) = size_hint {
                return None;
            }

            if inner.break_hint().is_break() {
                let inner = mem::replace(&mut inner, self.strategy.next_collector());
                return Some(inner.finish());
            }

            // We try our best not to consume prematurely,
            // while not also causing an infinite loop.

            match size_hint {
                (1.., _) => {
                    // Use `collect_many()` because `collect_then_finish()`
                    // doesn't tell us whether it has stopped after the call.
                    if inner.collect_many(&mut items).is_break() {
                        let inner = mem::replace(&mut inner, self.strategy.next_collector());
                        Some(inner.finish())
                    } else {
                        None
                    }
                }
                _ => {
                    // We have no other way but to probe.
                    // We may pull one item prematurely, but that's fine.
                    // We have done the best effort.
                    let item = items.next()?;

                    if inner
                        .collect_many(iter::once(item).chain(&mut items))
                        .is_break()
                    {
                        let inner = mem::replace(&mut inner, self.strategy.next_collector());
                        Some(inner.finish())
                    } else {
                        None
                    }
                }
            }
        }))
    }
}

impl<CO, S> WithStrategy<CO, S>
where
    CO: Debug,
    S: StrategyBase<Collector: Debug>,
{
    pub(super) fn debug_struct(&self, debug_struct: &mut DebugStruct<'_, '_>) {
        debug_struct.field("outer", &self.outer);
        self.strategy.debug(debug_struct);
        debug_struct.field("inner", &self.inner);
    }
}
