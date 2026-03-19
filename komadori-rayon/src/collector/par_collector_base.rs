use std::ops::ControlFlow;

use super::{Fuse, IntoParallelCollectorBase, Tee};

/// Parallel version of collectors
pub trait ParallelCollectorBase: Sized {
    ///
    type Output;

    ///
    fn finish(self) -> Self::Output;

    ///
    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    ///
    #[inline]
    fn tee<C>(self, other: C) -> Tee<Self, C::IntoParCollector>
    where
        C: IntoParallelCollectorBase,
    {
        Tee::new(self, other.into_par_collector())
    }

    ///
    #[inline]
    fn fuse(self) -> Fuse<Self> {
        Fuse::new(self)
    }
}
