use std::ops::ControlFlow;

use crate::{
    collector::plumbing::{Collector, CollectorBase, finish_boxed_impl},
    ops::Try,
};

use super::{DefineLocal, NestLocalBase, SplittableLocal};

/// A parallel collector that uses a closure and local states
/// to collect items in each local reduction, which an ability
/// to stop early.
///
/// This `struct` is created by
/// [`UnindexedParallelCollectorBase::try_fold_local()`](super::UnindexedParallelCollectorBase::try_fold_local).
/// See its documentation for more.
#[allow(private_interfaces)]
pub type TryFoldLocal<C, L1, FL2, F> = NestLocalBase<C, TryFoldLocalSplittableInner<L1, FL2, F>>;

impl<C, L1, FL2, F> TryFoldLocal<C, L1, FL2, F> {
    pub(in crate::collector) fn new(collector: C, seed: L1, init: FL2, f: F) -> Self {
        Self {
            collector,
            splittable_local: TryFoldLocalSplittableInner {
                seed: Some(seed),
                init,
                f,
            },
        }
    }
}

mod private {
    use std::{any::type_name, fmt::Debug};

    use crate::ops::Try;

    #[derive(Clone)]
    pub struct TryFoldLocalSplittableInner<S, FA, F> {
        pub(super) seed: Option<S>,
        pub(super) init: FA,
        pub(super) f: F,
    }

    #[allow(missing_debug_implementations)]
    pub enum Inner<'a, A, F>
    where
        A: Try,
    {
        Continue { accum: A::Output, f: &'a F },
        Break(A::Residual),
    }

    impl<S, FA, F> Debug for TryFoldLocalSplittableInner<S, FA, F>
    where
        S: Debug,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TryFoldLocalSplittableInner")
                .field("seed", &self.seed)
                .field("init", &type_name::<FA>())
                .field("f", &type_name::<F>())
                .finish()
        }
    }
}
use private::*;

struct Anchor<'a, S, FA, F> {
    seed: S,
    init: &'a FA,
    f: &'a F,
}

impl<'a, S, FA, A, F> DefineLocal<'a> for TryFoldLocalSplittableInner<S, FA, F>
where
    S: Clone + Send,
    A: Try,
    FA: Fn(S) -> A + Sync,
    F: Sync,
{
    type Local = Inner<'a, A, F>;
}

impl<S, FA, A, F> SplittableLocal for TryFoldLocalSplittableInner<S, FA, F>
where
    S: Clone + Send,
    A: Try,
    FA: Fn(S) -> A + Sync,
    F: Sync,
{
    #[inline]
    fn anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineLocal<'a>>::Local> {
        Anchor {
            seed: self.seed.clone().expect(TAKEN_ERR_MSG),
            init: &self.init,
            f: &self.f,
        }
    }

    #[inline]
    fn take_anchor<'a>(
        &'a mut self,
    ) -> impl super::Anchor<Inner = <Self as DefineLocal<'a>>::Local> {
        Anchor {
            seed: self.seed.take().expect(TAKEN_ERR_MSG),
            init: &self.init,
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
            seed: self.seed.clone(),
            init: self.init,
            f: self.f,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.seed.clone_from(&source.seed);
        self.init = source.init;
        self.f = source.f;
    }
}

impl<'a, L, FA, A, F> super::Anchor for Anchor<'a, L, FA, F>
where
    L: Clone + Send,
    FA: Fn(L) -> A + Sync,
    A: Try,
    F: Sync,
{
    type Inner = Inner<'a, A, F>;

    #[inline]
    fn into_inner(self) -> Self::Inner {
        match (self.init)(self.seed).branch() {
            ControlFlow::Continue(accum) => Inner::Continue { accum, f: self.f },
            ControlFlow::Break(residual) => Inner::Break(residual),
        }
    }
}

const TAKEN_ERR_MSG: &str = "local1 is already taken";

impl<A, F> CollectorBase for Inner<'_, A, F>
where
    A: Try,
{
    type Output = A;

    #[inline]
    fn finish(self) -> Self::Output {
        match self {
            Self::Continue { accum: local, .. } => A::from_output(local),
            Self::Break(residual) => A::from_residual(residual),
        }
    }

    finish_boxed_impl! {}

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        match self {
            Self::Continue { .. } => request,
            Self::Break(_) => 0,
        }
    }
}

impl<A, F, T, R> Collector<T> for Inner<'_, A, F>
where
    A: Try,
    F: Fn(&mut A::Output, T) -> R,
    R: Try<Output = (), Residual = A::Residual>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self {
            Self::Continue { accum, f } => match f(accum, item).branch() {
                ControlFlow::Continue(()) => ControlFlow::Continue(()),
                ControlFlow::Break(residual) => {
                    *self = Self::Break(residual);
                    ControlFlow::Break(())
                }
            },
            Self::Break(_) => ControlFlow::Break(()),
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match self {
            Self::Continue { accum, f } => match items
                .into_iter()
                .try_for_each(move |item| f(accum, item).branch())
            {
                ControlFlow::Continue(()) => ControlFlow::Continue(()),
                ControlFlow::Break(residual) => {
                    *self = Self::Break(residual);
                    ControlFlow::Break(())
                }
            },
            Self::Break(_) => ControlFlow::Break(()),
        }
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self {
            Self::Continue { mut accum, f } => match items
                .into_iter()
                .try_for_each(|item| f(&mut accum, item).branch())
            {
                ControlFlow::Continue(()) => A::from_output(accum),
                ControlFlow::Break(residual) => A::from_residual(residual),
            },
            Self::Break(residual) => A::from_residual(residual),
        }
    }
}
