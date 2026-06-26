use komadori::prelude::*;

use super::{DefineLocal, NestLocalBase, SplittableLocal};

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
            splittable_local: NestLocalWithSplittableInner {
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

struct Anchor<'a, L, F> {
    local: L,
    inner_f: &'a F,
}

impl<'a, L, F, C> DefineLocal<'a> for NestLocalWithSplittableInner<L, F>
where
    L: Clone + Send,
    F: Fn(L) -> C,
    C: IntoCollectorBase,
{
    type Local = C::IntoCollector;
}

impl<L, F, C> SplittableLocal for NestLocalWithSplittableInner<L, F>
where
    L: Clone + Send,
    F: Fn(L) -> C + Sync,
    C: IntoCollectorBase,
{
    #[inline]
    fn anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineLocal<'a>>::Local> {
        let local = self.local.as_ref().expect(TAKEN_ERR_MSG).clone();
        Anchor {
            local,
            inner_f: &self.inner_f,
        }
    }

    #[inline]
    fn take_anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineLocal<'a>>::Local> {
        let local = self.local.take().expect(TAKEN_ERR_MSG);
        Anchor {
            local,
            inner_f: &self.inner_f,
        }
    }
}

impl<L, F> Clone for Anchor<'_, L, F>
where
    L: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            local: self.local.clone(),
            inner_f: self.inner_f,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.local.clone_from(&source.local);
        self.inner_f = source.inner_f;
    }
}

impl<L, F, C> super::Anchor for Anchor<'_, L, F>
where
    L: Clone + Send,
    F: Fn(L) -> C + Sync,
    C: IntoCollectorBase,
{
    type Inner = C::IntoCollector;

    #[inline]
    fn into_inner(self) -> Self::Inner {
        (self.inner_f)(self.local).into_collector()
    }
}

const TAKEN_ERR_MSG: &str = "the local state is already taken";
