use std::ops::ControlFlow;

use komadori::prelude::*;

use super::{DefineLocal, NestLocalBase, SplittableLocal};

/// A parallel collector that uses a closure and local states
/// to collect items in each serial reduction.
///
/// This `struct` is created by
/// [`UnindexedParallelCollectorBase::fold_local()`](super::UnindexedParallelCollectorBase::fold_local).
/// See its documentation for more.
#[allow(private_interfaces)]
pub type FoldLocal<C, L1, FL2, F> = NestLocalBase<C, FoldLocalSplittableInner<L1, FL2, F>>;

impl<C, L1, FL2, F> FoldLocal<C, L1, FL2, F> {
    pub(in crate::collector) fn new(collector: C, local1: L1, local2_f: FL2, f: F) -> Self {
        Self {
            collector,
            splittable_local: FoldLocalSplittableInner {
                local1: Some(local1),
                local2_f,
                f,
            },
        }
    }
}

mod private {
    #[derive(Clone, Debug)]
    pub struct FoldLocalSplittableInner<L1, FL2, F> {
        pub(super) local1: Option<L1>,
        pub(super) local2_f: FL2,
        pub(super) f: F,
    }

    #[allow(missing_debug_implementations)]
    pub struct Inner<'a, L1, L2, F> {
        pub(super) local1: L1,
        pub(super) local2: L2,
        pub(super) f: &'a F,
    }
}
use private::*;

struct Anchor<'a, L1, FL2, F> {
    local1: L1,
    local2_f: &'a FL2,
    f: &'a F,
}

impl<'a, L1, FL2, L2, F> DefineLocal<'a> for FoldLocalSplittableInner<L1, FL2, F>
where
    L1: Clone + Send,
    FL2: Fn() -> L2 + Sync,
    F: Sync,
{
    type Local = Inner<'a, L1, L2, F>;
}

impl<L1, FL2, L2, F> SplittableLocal for FoldLocalSplittableInner<L1, FL2, F>
where
    L1: Clone + Send,
    FL2: Fn() -> L2 + Sync,
    F: Sync,
{
    #[inline]
    fn anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineLocal<'a>>::Local> {
        Anchor {
            local1: self.local1.as_ref().expect(TAKEN_ERR_MSG).clone(),
            local2_f: &self.local2_f,
            f: &self.f,
        }
    }

    #[inline]
    fn take_anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineLocal<'a>>::Local> {
        Anchor {
            local1: self.local1.take().expect(TAKEN_ERR_MSG),
            local2_f: &self.local2_f,
            f: &self.f,
        }
    }
}

impl<L1, FL2, F> Clone for Anchor<'_, L1, FL2, F>
where
    L1: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            local1: self.local1.clone(),
            local2_f: self.local2_f,
            f: self.f,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.local1.clone_from(&source.local1);
        self.local2_f = source.local2_f;
        self.f = source.f;
    }
}

impl<'a, L1, FL2, L2, F> super::Anchor for Anchor<'a, L1, FL2, F>
where
    L1: Clone + Send,
    FL2: Fn() -> L2 + Sync,
    F: Sync,
{
    type Inner = Inner<'a, L1, L2, F>;

    #[inline]
    fn into_inner(self) -> Self::Inner {
        Inner {
            local1: self.local1,
            local2: (self.local2_f)(),
            f: self.f,
        }
    }
}

const TAKEN_ERR_MSG: &str = "local1 is already taken";

impl<L1, L2, F> CollectorBase for Inner<'_, L1, L2, F> {
    type Output = (L1, L2);

    #[inline]
    fn finish(self) -> Self::Output {
        (self.local1, self.local2)
    }
}

impl<L1, L2, F, T> Collector<T> for Inner<'_, L1, L2, F>
where
    F: Fn(&mut L1, &mut L2, T),
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        (self.f)(&mut self.local1, &mut self.local2, item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        items
            .into_iter()
            .for_each(|item| (self.f)(&mut self.local1, &mut self.local2, item));
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        items.into_iter().for_each({
            let local1 = &mut self.local1;
            let local2 = &mut self.local2;
            let f = self.f;
            move |item| f(local1, local2, item)
        });

        (self.local1, self.local2)
    }
}
