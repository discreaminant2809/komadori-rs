use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::plumbing::finish_boxed_impl;

use super::{DefineInner, NestLocalBase, SplittableInner};

/// A parallel collector that uses a closure and local states
/// to collect items in each serial reduction.
///
/// This `struct` is created by
/// [`UnindexedParallelCollectorBase::fold_local()`](super::UnindexedParallelCollectorBase::fold_local).
/// See its documentation for more.
#[allow(private_interfaces)]
pub type FoldLocal<C, S, FF> = NestLocalBase<C, FoldLocalSplittableInner<S, FF>>;

impl<C, S, FF> FoldLocal<C, S, FF> {
    pub(in crate::collector) fn new(collector: C, shared_state: S, consumer: FF) -> Self {
        Self {
            collector,
            splittable_inner: FoldLocalSplittableInner {
                shared_state,
                consumer: Some(consumer),
            },
        }
    }
}

mod private {
    #[derive(Clone, Debug)]
    pub struct FoldLocalSplittableInner<S, FF> {
        pub(super) shared_state: S,
        pub(super) consumer: Option<FF>,
    }

    #[expect(missing_debug_implementations)]
    pub struct Inner<'a, S, A, F> {
        pub(super) shared_state: &'a S,
        pub(super) accum: A,
        pub(super) f: F,
    }
}
use private::*;

impl<'a, S, A, FF, F> DefineInner<'a> for FoldLocalSplittableInner<S, FF>
where
    S: Sync,
    FF: FnOnce(&S) -> (A, F) + Clone + Send,
    F: Sync,
{
    type Inner = Inner<'a, S, A, F>;
}

impl<S, A, FF, F> SplittableInner for FoldLocalSplittableInner<S, FF>
where
    S: Sync,
    FF: FnOnce(&S) -> (A, F) + Clone + Send,
    F: Sync,
{
    #[inline]
    fn anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineInner<'a>>::Inner> {
        let consumer = self.consumer.clone().expect(TAKEN_ERR_MSG);

        || {
            let (accum, f) = consumer(&self.shared_state);
            Inner {
                shared_state: &self.shared_state,
                accum,
                f,
            }
        }
    }

    #[inline]
    fn take_anchor<'a>(
        &'a mut self,
    ) -> impl super::Anchor<Inner = <Self as DefineInner<'a>>::Inner> {
        let consumer = self.consumer.take().expect(TAKEN_ERR_MSG);

        || {
            let (accum, f) = consumer(&self.shared_state);
            Inner {
                shared_state: &self.shared_state,
                accum,
                f,
            }
        }
    }
}

const TAKEN_ERR_MSG: &str = "`consumer` is already taken";

impl<S, A, F> CollectorBase for Inner<'_, S, A, F> {
    type Output = A;

    #[inline]
    fn finish(self) -> Self::Output {
        self.accum
    }

    finish_boxed_impl! {}
}

impl<S, A, F, T> Collector<T> for Inner<'_, S, A, F>
where
    F: FnMut(&S, &mut A, T),
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        (self.f)(self.shared_state, &mut self.accum, item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        items
            .into_iter()
            .for_each(|item| (self.f)(self.shared_state, &mut self.accum, item));
        ControlFlow::Continue(())
    }
}
