use komadori::prelude::*;

use super::{DefineInner, NestLocalBase, SplittableInner};

/// A parallel collector that collects all the outputs
/// from local collectors created from a function to each serial reduction.
///
/// This `struct` is created by
/// [`UnindexedParallelCollectorBase::nest_local_with()`](super::UnindexedParallelCollectorBase::nest_local_with).
/// See its documentation for more.
#[allow(private_interfaces)]
pub type NestLocalWith<C, L, F> = NestLocalBase<C, NestLocalWithSplittableInner<L, F>>;

impl<C, L, F> NestLocalWith<C, L, F> {
    pub(in crate::collector) fn new(collector: C, local: L, inner_f: F) -> Self {
        Self {
            collector,
            splittable_inner: NestLocalWithSplittableInner {
                local: Some(local),
                inner_f,
            },
        }
    }
}

mod private {
    #[derive(Clone, Debug)]
    pub struct NestLocalWithSplittableInner<L, F> {
        pub(super) local: Option<L>,
        pub(super) inner_f: F,
    }
}
use private::NestLocalWithSplittableInner;

impl<'a, L, F, C> DefineInner<'a> for NestLocalWithSplittableInner<L, F>
where
    L: Clone + Send,
    F: Fn(L) -> C,
    C: IntoCollectorBase,
{
    type Inner = C::IntoCollector;
}

impl<L, F, C> SplittableInner for NestLocalWithSplittableInner<L, F>
where
    L: Clone + Send,
    F: Fn(L) -> C + Sync,
    C: IntoCollectorBase,
{
    #[inline]
    fn anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineInner<'a>>::Inner> {
        let local = self.local.clone().expect(TAKEN_ERR_MSG);
        || (self.inner_f)(local)
    }

    #[inline]
    fn take_anchor<'a>(
        &'a mut self,
    ) -> impl super::Anchor<Inner = <Self as DefineInner<'a>>::Inner> {
        let local = self.local.take().expect(TAKEN_ERR_MSG);
        || (self.inner_f)(local)
    }
}

const TAKEN_ERR_MSG: &str = "the local state is already taken";
