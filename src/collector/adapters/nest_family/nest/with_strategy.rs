use std::{
    fmt::{Debug, DebugStruct},
    ops::ControlFlow,
};

use crate::collector::{Collector, CollectorBase, Fuse};

use super::super::strategy::{Strategy, StrategyBase};

#[derive(Clone)]
pub struct WithStrategy<CO, S>
where
    S: StrategyBase,
{
    // It's possible that the active inner has been accumulating,
    // but then `break_hint` is called and it signals a stop.
    outer: Fuse<CO>,
    strategy: S,
    // An `Option` is neccessary here because this case exists:
    // if we use a bare `CI`, we create this collector, but then
    // we call `finish()` right away, it is incorrect for the outer
    // to collect this.
    inner: Option<S::Collector>,
}

impl<CO, S> WithStrategy<CO, S>
where
    CO: CollectorBase,
    S: StrategyBase,
{
    pub(super) fn new(outer: CO, strategy: S) -> Self {
        Self {
            outer: outer.fuse(),
            strategy,
            inner: None,
        }
    }
}

impl<CO, S> CollectorBase for WithStrategy<CO, S>
where
    CO: Collector<S::Output>,
    S: StrategyBase,
{
    type Output = CO::Output;

    fn finish(mut self) -> Self::Output {
        if let Some(inner) = self.inner {
            // Due to this line, the outer has to be fused.
            let _ = self.outer.collect(inner.finish());
        }

        self.outer.finish()
    }

    #[inline]
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
        let inner = if let Some(inner) = &mut self.inner {
            inner
        } else {
            // We collect in a loop. Should we use `collect_many()` for the outer?
            // Nah. The best I've tried is `iter::repeat_with(...).map_while(...)`.
            // This combo guarantees a `(0, None)` size hint, which has little
            // to no chance of optimize. Not to mention "fraudulent" `collect_many()`
            // implementation. The outer may return `Continue(())` even tho
            // it hasn't exhausted the iterator, leading to more guards
            // => goodbye optimization.
            loop {
                let inner = self.strategy.next_collector();
                if inner.break_hint().is_continue() {
                    break self.inner.insert(inner);
                }

                self.outer.collect(inner.finish())?;
            }
        };

        if inner.collect(item).is_break() {
            self.outer.collect(
                self.inner
                    .take()
                    .expect("inner collector should exist")
                    .finish(),
            )
        } else {
            self.outer.break_hint()
        }
    }

    // TODO: the overrides are still buggy

    // fn collect_many(&mut self, items: impl IntoIterator<Item = Self::Item>) -> ControlFlow<()> {
    //     // Fuse is needed because in the previous cycle the inner may've exhausted the iterator.
    //     // If we peek again, we may accidentally unroll the iterator.
    //     // Could we use our own flag? Nope:
    //     // - If we implement our own iterator, we lose benefits from heavy specialization from Rust.
    //     // - If we use `inspect()`, we actually don't need to set the flag a lot and
    //     //   and the type of the iterator may not be preserved,
    //     //   hence again we lose benefits from heavy specialization.
    //     // In a nutshell, try to use what the standard library provides as much as possible.
    //     //
    //     // "B-but `peek()` pulls out one item prematurely." Not an issue!
    //     // If there is at least one item (hence that item is pulled out prematurely),
    //     // it is eventually collected by the inner anyway.
    //     // Why? In the loop that refreshing the inner, the `break` condition is that
    //     // the inner can still accumulate, which means we always have something
    //     // to collect that prematurely pulled item.
    //     let mut items = items.into_iter().fuse().peekable();

    //     self.outer.collect_many(std::iter::from_fn(|| {
    //         let inner = if let Some(inner) = &mut self.inner {
    //             inner
    //         } else {
    //             let inner = self.inner.clone();
    //             if inner.break_hint() {
    //                 return Some(inner.finish());
    //             }

    //             self.inner.insert(inner)
    //         };

    //         inner.collect_many(&mut items).is_break().then(|| {
    //             self.inner
    //                 .take()
    //                 .expect("inner collector should exist")
    //                 .finish()
    //         })
    //     }))

    //     // // Are there still more items? If no, don't put me in a loop!
    //     // while items.peek().is_some() {
    //     //     let inner = if let Some(inner) = &mut self.inner {
    //     //         inner
    //     //     } else {
    //     //         loop {
    //     //             let inner = self.inner.clone();
    //     //             if !inner.break_hint() {
    //     //                 break self.inner.insert(inner);
    //     //             }

    //     //             self.outer.collect(inner.finish())?;
    //     //         }
    //     //     };

    //     //     if inner.collect_many(&mut items).is_break() {
    //     //         self.outer.collect(
    //     //             self.inner
    //     //                 .take()
    //     //                 .expect("inner collector should exist")
    //     //                 .finish(),
    //     //         )?;
    //     //     } else {
    //     //         // the inner still returns `Continue(())`. The iterator is definitely exhausted...
    //     //         break;
    //     //     }

    //     //     // ...but if it doesn't, we can't conclude whether it's exhausted or not.
    //     //     // It is possible that the inner stops AND the iterator is exhausted or not.
    //     //     // That's why we need the `fuse().peekable()` combo. It guards both cases.
    //     // }

    //     // ControlFlow::Continue(())
    // }

    // fn collect_then_finish(mut self, items: impl IntoIterator<Item = Self::Item>) -> Self::Output {
    //     let mut items = items.into_iter().fuse().peekable();

    //     // Both blocks look `chain`able, but unfortunably we can't share the `items`
    //     // and/or avoid premature clone.
    //     if let Some(inner) = self.inner
    //         && self
    //             .outer
    //             .collect(inner.collect_then_finish(&mut items))
    //             .is_break()
    //     {
    //         return self.outer.finish();
    //     }

    //     self.outer.collect_then_finish(std::iter::from_fn(move || {
    //         let inner = self.inner.clone();
    //         // To prevent pulling one item prematurely.
    //         if inner.break_hint() {
    //             return Some(inner.finish());
    //         }

    //         items.peek()?;
    //         Some(inner.collect_then_finish(&mut items))
    //     }))
    // }
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
