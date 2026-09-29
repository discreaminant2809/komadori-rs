mod fold_local;
#[allow(clippy::module_inception)]
mod nest_local;
mod nest_local_with;
mod traits;
mod try_fold_local;

pub use fold_local::FoldLocal;
pub use nest_local::NestLocal;
pub use nest_local_with::NestLocalWith;
pub use try_fold_local::TryFoldLocal;

use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollector, UnindexedParallelCollectorBase,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

use traits::*;

mod inner {
    #[derive(Clone, Debug)]
    pub struct NestLocalBase<C, S> {
        pub(super) collector: C,
        pub(super) splittable_inner: S,
    }
}
use inner::NestLocalBase;

impl<'a, C, S> DefineSerial<'a> for NestLocalBase<C, S>
where
    C: DefineUnindexedSerial<
            'a,
            UnindexedSerial: Collector<<<S as DefineInner<'a>>::Inner as CollectorBase>::Output>,
        >,
    S: DefineInner<'a>,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<C::UnindexedSerial, S::Inner>>;
}

impl<'a, C, S> DefineUnindexedSerial<'a> for NestLocalBase<C, S>
where
    C: DefineUnindexedSerial<
            'a,
            UnindexedSerial: Collector<<<S as DefineInner<'a>>::Inner as CollectorBase>::Output>,
        >,
    S: DefineInner<'a>,
{
    type UnindexedSerial =
        unique_unindexed::Serial<'a, Self, consumer::Serial<C::UnindexedSerial, S::Inner>>;
}

impl<C, S> ParallelCollectorBase for NestLocalBase<C, S>
where
    C: for<'a> UnindexedParallelCollector<<<S as DefineInner<'a>>::Inner as CollectorBase>::Output>,
    S: SplittableInner,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.collector.max_afford(1) == 0 || self.splittable_inner.max_afford(1) == 0 {
            0
        } else {
            request
        }
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.unindexed_parts();
        unique::uniquify((
            consumer::unindexed(consumer, self.splittable_inner.anchor()),
            commit,
        ))
    }

    fn take_parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        let (consumer, commit) = self.collector.take_unindexed_parts();
        unique::take_uniquify((
            consumer::unindexed(consumer, self.splittable_inner.take_anchor()),
            commit,
        ))
    }
}

impl<C, S> UnindexedParallelCollectorBase for NestLocalBase<C, S>
where
    C: for<'a> UnindexedParallelCollector<<<S as DefineInner<'a>>::Inner as CollectorBase>::Output>,
    S: SplittableInner,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.unindexed_parts();
        unique_unindexed::uniquify((
            consumer::unindexed(consumer, self.splittable_inner.anchor()),
            commit,
        ))
    }

    fn take_unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        let (consumer, commit) = self.collector.take_unindexed_parts();
        unique_unindexed::take_uniquify((
            consumer::unindexed(consumer, self.splittable_inner.take_anchor()),
            commit,
        ))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use crate::collector::plumbing::{
        self, BasicUnindexedConsumer, Collector, CollectorBase, OpaqueUnindexedConsumer,
        UnindexedConsumer,
    };

    use super::Anchor;

    pub fn unindexed<C, A>(
        consumer: C,
        anchor: A,
    ) -> OpaqueUnindexedConsumer!(Serial<C::IntoCollector, A::Inner>)
    where
        C: UnindexedConsumer<IntoCollector: Collector<<A::Inner as CollectorBase>::Output>>,
        A: Anchor,
    {
        BasicUnindexedConsumer {
            state: (consumer, anchor),
            split_f: |(consumer, anchor)| (consumer.split_off_left(), anchor.clone()),
            combiner_f: |(consumer, _)| consumer.to_combiner(),
            ma_f: |(consumer, anchor), request| {
                if consumer.max_afford(1) == 0 {
                    0
                } else {
                    anchor.max_afford(request)
                }
            },
            collector_f: |(consumer, anchor)| Serial {
                outer: consumer.into_collector(),
                inner: anchor.into_inner(),
            },
        }
    }

    pub struct Serial<O, I> {
        outer: O,
        inner: I,
    }

    impl<O, I> CollectorBase for Serial<O, I>
    where
        O: Collector<I::Output>,
        I: CollectorBase,
    {
        type Output = O::Output;

        #[inline]
        fn finish(mut self) -> Self::Output {
            let _ = self.outer.collect(self.inner.finish());
            self.outer.finish()
        }

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            if self.outer.max_afford(1) == 0 {
                0
            } else {
                self.inner.max_afford(request)
            }
        }

        // Practically we use `vec![]` most of the time for `nest_local`
        // and probably `nest_local_with`, so we can make leaf reduction faster
        // for most cases.
        // If the callers dislike this behavior, they can always use `fold_local`
        // instead.
        #[inline]
        fn reserve(&mut self, additional: usize) {
            self.inner.reserve(additional);
        }
    }

    impl<O, I, T> Collector<T> for Serial<O, I>
    where
        O: Collector<I::Output>,
        I: Collector<T>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            self.inner.collect(item)?;
            plumbing::break_hint(&self.outer)
        }

        #[inline]
        unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
            unsafe {
                self.inner.assume_reserved_collect(item)?;
            }

            plumbing::break_hint(&self.outer)
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            plumbing::advanced_collect_many_default_impl(self, items)
        }
    }
}
