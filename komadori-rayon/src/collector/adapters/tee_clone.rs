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
    // Teeing with `tee_clone` isn't cheap.

    #[inline]
    fn pass_down(&mut self, item: &mut T) -> T {
        item.clone()
    }

    #[inline]
    fn no_tee_collect(&mut self, collector: &mut impl Collector<T>, item: T) -> ControlFlow<()> {
        collector.collect(item)
    }

    #[inline]
    unsafe fn no_tee_assume_reserved_collect(
        &mut self,
        collector: &mut impl for<'a> Collector<<Self as DefinePassDown<'a, T>>::PassDown>,
        item: T,
    ) -> ControlFlow<()> {
        unsafe { collector.assume_reserved_collect(item) }
    }
}
