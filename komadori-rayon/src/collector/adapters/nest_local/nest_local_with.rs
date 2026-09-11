use komadori::prelude::*;

use super::{DefineInner, NestLocalBase, SplittableInner};

/// A parallel collector that collects all the outputs
/// from local collectors created from a function to each serial reduction.
///
/// This `struct` is created by
/// [`UnindexedParallelCollectorBase::nest_local_with()`](super::UnindexedParallelCollectorBase::nest_local_with).
/// See its documentation for more.
#[allow(private_interfaces)]
pub type NestLocalWith<C, S, F> = NestLocalBase<C, NestLocalWithSplittableInner<S, F>>;

impl<C, S, F> NestLocalWith<C, S, F> {
    pub(in crate::collector) fn new(collector: C, shared_state: S, consumer: F) -> Self {
        Self {
            collector,
            splittable_inner: NestLocalWithSplittableInner {
                shared_state,
                consumer: Some(consumer),
            },
        }
    }
}

mod private {
    #[derive(Clone, Debug)]
    pub struct NestLocalWithSplittableInner<S, F> {
        pub(super) shared_state: S,
        pub(super) consumer: Option<F>,
    }
}
use private::NestLocalWithSplittableInner;

impl<'a, S, F, C> DefineInner<'a> for NestLocalWithSplittableInner<S, F>
where
    S: Sync,
    F: FnOnce(&S) -> C + Clone + Send,
    C: IntoCollectorBase,
{
    type Inner = C::IntoCollector;
}

impl<S, F, C> SplittableInner for NestLocalWithSplittableInner<S, F>
where
    S: Sync,
    F: FnOnce(&S) -> C + Clone + Send,
    C: IntoCollectorBase,
{
    #[inline]
    fn anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineInner<'a>>::Inner> {
        let consumer = self.consumer.clone().expect(TAKEN_ERR_MSG);
        || consumer(&self.shared_state)
    }

    #[inline]
    fn take_anchor<'a>(
        &'a mut self,
    ) -> impl super::Anchor<Inner = <Self as DefineInner<'a>>::Inner> {
        let consumer = self.consumer.clone().expect(TAKEN_ERR_MSG);
        || consumer(&self.shared_state)
    }
}

const TAKEN_ERR_MSG: &str = "`consumer` is already taken";
