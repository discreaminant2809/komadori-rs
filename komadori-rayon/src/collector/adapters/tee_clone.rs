use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::ParallelCollectorBase;

use super::{DefinePassDown, TeeBase, Teer};

/// A parallel collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`ParallelCollectorBase::tee_clone()`].
/// See its documentation for more.
pub type TeeClone<C1, C2> = TeeBase<C1, C2, CloneTeer>;

pub(in crate::collector) fn tee_clone<C1, C2>(collector1: C1, collector2: C2) -> TeeClone<C1, C2>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
{
    TeeBase::new(collector1, collector2, CloneTeer(()))
}

// `pub` to satisfy the compiler.
// Users can't reach this anyway.
#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct CloneTeer(());

impl<'this, T> DefinePassDown<'this, T> for CloneTeer
where
    T: Clone,
{
    type PassDown = T;
}

impl<T> Teer<T> for CloneTeer
where
    T: Clone,
{
    #[inline]
    fn pass_down(&mut self, item: &mut T) -> T {
        item.clone()
    }

    #[inline]
    fn no_tee_collect(&mut self, collector: &mut impl Collector<T>, item: T) -> ControlFlow<()> {
        collector.collect(item)
    }

    #[inline]
    fn no_tee_collect_many(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: &mut impl Collector<T>,
    ) -> ControlFlow<()> {
        collector.collect_many(items)
    }

    #[inline]
    fn no_tee_collect_then_finish<O>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: impl Collector<T, Output = O>,
    ) -> O {
        collector.collect_then_finish(items)
    }
}
