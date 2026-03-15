use std::fmt::{Debug, DebugStruct};

use crate::collector::{Collector, CollectorBase};

pub trait StrategyBase {
    type Output;
    type Collector: CollectorBase<Output = Self::Output>;

    fn next_collector(&mut self) -> Self::Collector;

    /// Needed because `CloneStrategy` and `FnMut` closure
    /// debug differently.
    fn debug(&self, debug_struct: &mut DebugStruct<'_, '_>)
    where
        Self::Collector: Debug,
    {
        let _ = debug_struct;
    }
}

pub trait Strategy<T>: StrategyBase<Collector: Collector<T>> {}

impl<S, T> Strategy<T> for S where S: StrategyBase<Collector: Collector<T>> {}

#[derive(Clone)]
pub struct CloneStrategy<C>(C);

impl<C> CloneStrategy<C> {
    #[inline]
    pub fn new(x: C) -> Self {
        Self(x)
    }
}

impl<C> StrategyBase for CloneStrategy<C>
where
    C: CollectorBase + Clone,
{
    type Output = C::Output;
    type Collector = C;

    #[inline]
    fn next_collector(&mut self) -> Self::Collector {
        self.0.clone()
    }

    #[inline]
    fn debug(&self, debug_struct: &mut DebugStruct<'_, '_>)
    where
        Self::Collector: Debug,
    {
        debug_struct.field("inner_cloner", &self.0);
    }
}

impl<C, F> StrategyBase for F
where
    C: CollectorBase,
    F: FnMut() -> C,
{
    type Output = C::Output;
    type Collector = C;

    #[inline]
    fn next_collector(&mut self) -> Self::Collector {
        self()
    }
}
