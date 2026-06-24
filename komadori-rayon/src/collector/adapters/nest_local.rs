mod fold_local;
#[allow(clippy::module_inception)]
mod nest_local;
mod nest_local_with;
mod traits;

pub use fold_local::FoldLocal;
pub use nest_local::NestLocal;
pub use nest_local_with::NestLocalWith;

use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollector, UnindexedParallelCollectorBase,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

use traits::*;

mod inner {
    #[derive(Clone, Debug)]
    pub struct NestLocalBase<C, S> {
        pub(super) collector: C,
        pub(super) splittable_local: S,
    }
}
use inner::NestLocalBase;

impl<'a, C, S> DefineSerial<'a> for NestLocalBase<C, S>
where
    C: DefineUnindexedSerial<
            'a,
            UnindexedSerial: Collector<<<S as DefineLocal<'a>>::Local as CollectorBase>::Output>,
        >,
    S: DefineLocal<'a>,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<C::UnindexedSerial, S::Local>>;
}

impl<'a, C, S> DefineUnindexedSerial<'a> for NestLocalBase<C, S>
where
    C: DefineUnindexedSerial<
            'a,
            UnindexedSerial: Collector<<<S as DefineLocal<'a>>::Local as CollectorBase>::Output>,
        >,
    S: DefineLocal<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<C::UnindexedSerial, S::Local>>;
}

impl<C, S> ParallelCollectorBase for NestLocalBase<C, S>
where
    C: for<'a> UnindexedParallelCollector<<<S as DefineLocal<'a>>::Local as CollectorBase>::Output>,
    S: SplittableLocal,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        unique::uniquify((
            len,
            consumer::Consumer::new(consumer, self.splittable_local.anchor()),
            commit,
        ))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique::take_uniquify((
            len,
            consumer::Consumer::new(consumer, self.splittable_local.anchor()),
            commit,
        ))
    }
}

impl<C, S> UnindexedParallelCollectorBase for NestLocalBase<C, S>
where
    C: for<'a> UnindexedParallelCollector<<<S as DefineLocal<'a>>::Local as CollectorBase>::Output>,
    S: SplittableLocal,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        unique_unindexed::uniquify((
            consumer::Consumer::new(consumer, self.splittable_local.anchor()),
            commit,
        ))
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique_unindexed::take_uniquify((
            consumer::Consumer::new(consumer, self.splittable_local.anchor()),
            commit,
        ))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use crate::collector::plumbing::{
        self, Collector, CollectorBase, IntoCollector, IntoCollectorBase, UnindexedConsumer,
    };

    use super::Anchor;

    pub struct Consumer<C, A> {
        consumer: C,
        anchor: A,
    }

    pub struct Serial<O, I> {
        outer: O,
        inner: I,
    }

    impl<C, A> Consumer<C, A> {
        pub fn new(consumer: C, anchor: A) -> Self {
            Self { consumer, anchor }
        }
    }

    impl<C, A, I> IntoCollectorBase for Consumer<C, A>
    where
        C: IntoCollector<I::Output>,
        A: Anchor<Inner = I>,
        I: CollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector, I>;

        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                outer: self.consumer.into_collector(),
                inner: self.anchor.into_inner(),
            }
        }
    }

    impl<C, A, I> plumbing::Consumer for Consumer<C, A>
    where
        C: UnindexedConsumer<IntoCollector: Collector<I::Output>>,
        A: Anchor<Inner = I>,
        I: CollectorBase,
    {
        type Combiner = C::Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _index: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.consumer.break_hint()?;
            self.anchor.break_hint()
        }
    }

    impl<C, A, I> UnindexedConsumer for Consumer<C, A>
    where
        C: UnindexedConsumer<IntoCollector: Collector<I::Output>>,
        A: Anchor<Inner = I>,
        I: CollectorBase,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                anchor: self.anchor.clone(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
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

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            self.inner.break_hint()?;
            self.outer.break_hint()
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
            self.outer.break_hint()
        }

        // No meaningful overrides for the other two methods.
    }
}
