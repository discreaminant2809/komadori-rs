use std::ops::ControlFlow;

use crate::{
    collector::plumbing::{Collector, CollectorBase, finish_boxed_impl},
    ops::{ChangeOutputType, Try},
};

use super::{DefineLocal, NestLocalBase, SplittableLocal};

/// A parallel collector that produces each item from each local reduction
/// to feed into the underlying parallel collector, which an ability
/// to stop early.
///
/// This `struct` is created by
/// [`UnindexedParallelCollectorBase::try_fold_local()`](super::UnindexedParallelCollectorBase::try_fold_local).
/// See its documentation for more.
#[allow(private_interfaces)]
pub type TryFoldLocal<C, S, FF> = NestLocalBase<C, TryFoldLocalSplittableInner<S, FF>>;

impl<C, S, FF> TryFoldLocal<C, S, FF> {
    pub(in crate::collector) fn new(collector: C, shared_state: S, consumer: FF) -> Self {
        Self {
            collector,
            splittable_local: TryFoldLocalSplittableInner {
                shared_state,
                consumer: Some(consumer),
            },
        }
    }
}

mod private {
    use std::{any::type_name, fmt::Debug};

    use crate::ops::Try;

    #[derive(Clone)]
    pub struct TryFoldLocalSplittableInner<S, FF> {
        pub(super) shared_state: S,
        pub(super) consumer: Option<FF>,
    }

    #[allow(missing_debug_implementations)]
    pub enum Inner<'a, S, A, F>
    where
        A: Try,
    {
        Continue {
            shared_state: &'a S,
            accum: A::Output,
            f: F,
        },
        Break(A::Residual),
    }

    impl<S, FF> Debug for TryFoldLocalSplittableInner<S, FF>
    where
        S: Debug,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TryFoldLocalSplittableInner")
                .field("shared_state", &self.shared_state)
                .field("consumer", &type_name::<FF>())
                .finish()
        }
    }
}
use private::*;

struct Anchor<'a, S, FF> {
    shared_state: &'a S,
    consumer: FF,
}

impl<'a, S, A, Acc, FF, F> DefineLocal<'a> for TryFoldLocalSplittableInner<S, FF>
where
    S: Sync,
    A: Try<Output = (Acc, F)>,
    FF: FnOnce(&S) -> A + Clone + Send,
{
    type Local = Inner<'a, S, ChangeOutputType<A, Acc>, F>;
}

impl<S, A, Acc, FF, F> SplittableLocal for TryFoldLocalSplittableInner<S, FF>
where
    S: Sync,
    A: Try<Output = (Acc, F)>,
    FF: FnOnce(&S) -> A + Clone + Send,
{
    #[inline]
    fn anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineLocal<'a>>::Local> {
        Anchor {
            shared_state: &self.shared_state,
            consumer: self.consumer.clone().expect(CONSUMER_TAKEN_MSG),
        }
    }

    #[inline]
    fn take_anchor<'a>(
        &'a mut self,
    ) -> impl super::Anchor<Inner = <Self as DefineLocal<'a>>::Local> {
        Anchor {
            shared_state: &self.shared_state,
            consumer: self.consumer.take().expect(CONSUMER_TAKEN_MSG),
        }
    }
}

impl<S, FF> Clone for Anchor<'_, S, FF>
where
    FF: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            shared_state: self.shared_state,
            consumer: self.consumer.clone(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.shared_state = source.shared_state;
        self.consumer.clone_from(&source.consumer);
    }
}

impl<'a, S, A, Acc, FF, F> super::Anchor for Anchor<'a, S, FF>
where
    S: Sync,
    A: Try<Output = (Acc, F)>,
    FF: FnOnce(&S) -> A + Clone + Send,
{
    type Inner = Inner<'a, S, ChangeOutputType<A, Acc>, F>;

    #[inline]
    fn into_inner(self) -> Self::Inner {
        match (self.consumer)(self.shared_state).branch() {
            ControlFlow::Continue((accum, f)) => Inner::Continue {
                shared_state: self.shared_state,
                accum,
                f,
            },
            ControlFlow::Break(residual) => Inner::Break(residual),
        }
    }
}

const CONSUMER_TAKEN_MSG: &str = "`consumer` is already taken";

impl<S, A, F> CollectorBase for Inner<'_, S, A, F>
where
    A: Try,
{
    type Output = A;

    #[inline]
    fn finish(self) -> Self::Output {
        match self {
            Self::Continue { accum, .. } => A::from_output(accum),
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

impl<S, A, F, T> Collector<T> for Inner<'_, S, A, F>
where
    A: Try,
    F: FnMut(&S, &mut A::Output, T) -> ChangeOutputType<A, ()>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self {
            Self::Continue {
                shared_state,
                accum,
                f,
            } => match f(shared_state, accum, item).branch() {
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
            Self::Continue {
                shared_state,
                accum,
                f,
            } => match items
                .into_iter()
                .try_for_each(move |item| f(shared_state, accum, item).branch())
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
            Self::Continue {
                shared_state,
                mut accum,
                mut f,
            } => match items
                .into_iter()
                .try_for_each(|item| f(shared_state, &mut accum, item).branch())
            {
                ControlFlow::Continue(()) => A::from_output(accum),
                ControlFlow::Break(residual) => A::from_residual(residual),
            },
            Self::Break(residual) => A::from_residual(residual),
        }
    }
}
