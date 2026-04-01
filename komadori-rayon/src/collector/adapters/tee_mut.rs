use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::ParallelCollectorBase;

use super::{DefinePassDown, TeeBase, Teer};

/// A parallel collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`ParallelCollectorBase::tee_mut()`].
/// See its documentation for more.
pub type TeeMut<C1, C2> = TeeBase<C1, C2, MutTeer>;

pub(in crate::collector) fn tee_mut<C1, C2>(collector1: C1, collector2: C2) -> TeeMut<C1, C2>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
{
    TeeBase::new(collector1, collector2, MutTeer(()))
}

// `pub` to satisfy the compiler.
// Users can't reach this anyway.
#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct MutTeer(());

impl<'a, T> DefinePassDown<'a, &mut T> for MutTeer {
    type PassDown = &'a mut T;
}

impl<'i, T> Teer<&'i mut T> for MutTeer {
    #[inline]
    fn pass_down<'a>(&mut self, item: &'a mut &mut T) -> &'a mut T {
        item
    }

    #[inline]
    fn no_tee_collect(
        &mut self,
        collector: &mut impl for<'a> Collector<&'a mut T>,
        item: &'i mut T,
    ) -> ControlFlow<()> {
        collector.collect(item)
    }

    #[inline]
    fn no_tee_collect_many(
        &mut self,
        items: impl IntoIterator<Item = &'i mut T>,
        collector: &mut impl for<'a> Collector<&'a mut T>,
    ) -> ControlFlow<()> {
        collector.collect_many(items)
    }

    #[inline]
    fn no_tee_collect_then_finish<O>(
        &mut self,
        items: impl IntoIterator<Item = &'i mut T>,
        collector: impl for<'a> Collector<&'a mut T, Output = O>,
    ) -> O {
        collector.collect_then_finish(items)
    }
}
