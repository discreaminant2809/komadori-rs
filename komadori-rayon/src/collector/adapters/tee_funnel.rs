use crate::collector::ParallelCollectorBase;

use super::{DefinePassDown, TeeBase, Teer};

/// A parallel collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`ParallelCollectorBase::tee_mut()`].
/// See its documentation for more.
pub type TeeFunnel<C1, C2> = TeeBase<C1, C2, FunnelTeer>;

pub(in crate::collector) fn tee_funnel<C1, C2>(collector1: C1, collector2: C2) -> TeeFunnel<C1, C2>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
{
    TeeBase::new(collector1, collector2, FunnelTeer(()))
}

// `pub` to satisfy the compiler.
// Users can't reach this anyway.
#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct FunnelTeer(());

impl<'a, T> DefinePassDown<'a, T> for FunnelTeer {
    type PassDown = &'a mut T;
}

impl<T> Teer<T> for FunnelTeer {
    #[inline]
    fn pass_down<'a>(&mut self, item: &'a mut T) -> &'a mut T {
        item
    }

    // Cannot meaningfully override anything else.
}
