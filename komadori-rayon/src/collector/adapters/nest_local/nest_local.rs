use komadori::prelude::*;

use super::{DefineInner, NestLocalBase, SplittableInner};

/// A parallel collector that collects all the outputs
/// from local collectors cloned to each serial reduction.
///
/// This `struct` is created by
/// [`UnindexedParallelCollectorBase::nest_local()`](super::UnindexedParallelCollectorBase::nest_local).
/// See its documentation for more.
#[allow(private_interfaces)]
pub type NestLocal<C, I> = NestLocalBase<C, NestLocalSplittableInner<I>>;

impl<C, I> NestLocal<C, I> {
    pub(in crate::collector) fn new(collector: C, inner: I) -> Self {
        Self {
            collector,
            splittable_inner: NestLocalSplittableInner {
                collector: Some(inner),
            },
        }
    }
}

mod private {
    #[derive(Clone, Debug)]
    pub struct NestLocalSplittableInner<C> {
        pub(super) collector: Option<C>,
    }
}
use private::NestLocalSplittableInner;

#[derive(Clone)]
struct Anchor<C> {
    collector: C,
}

impl<'a, C> DefineInner<'a> for NestLocalSplittableInner<C>
where
    C: CollectorBase + Clone + Send,
{
    type Inner = C;
}

impl<C> SplittableInner for NestLocalSplittableInner<C>
where
    C: CollectorBase + Clone + Send,
{
    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector
            .as_ref()
            .expect(TAKEN_ERR_MSG)
            .max_afford(request)
    }

    #[inline]
    fn anchor<'a>(&'a mut self) -> impl super::Anchor<Inner = <Self as DefineInner<'a>>::Inner> {
        Anchor {
            collector: self.collector.as_ref().expect(TAKEN_ERR_MSG).clone(),
        }
    }

    #[inline]
    fn take_anchor<'a>(
        &'a mut self,
    ) -> impl super::Anchor<Inner = <Self as DefineInner<'a>>::Inner> {
        Anchor {
            collector: self.collector.take().expect(TAKEN_ERR_MSG),
        }
    }
}

impl<C> super::Anchor for Anchor<C>
where
    C: CollectorBase + Clone + Send,
{
    type Inner = C;

    #[inline]
    fn into_inner(self) -> Self::Inner {
        self.collector
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request)
    }
}

const TAKEN_ERR_MSG: &str = "the collector is already taken";
