use std::{fmt::Debug, ops::ControlFlow};

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

use super::Fuse;

#[derive(Clone)]
pub struct TeeBase<C1, C2, TF> {
    collector1: Fuse<C1>,
    collector2: Fuse<C2>,
    teer: TF,
}

impl<C1, C2, TF> TeeBase<C1, C2, TF>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
{
    pub(super) fn new(collector1: C1, collector2: C2, teer: TF) -> Self {
        Self {
            collector1: collector1.fuse(),
            collector2: collector2.fuse(),
            teer,
        }
    }
}

impl<C1, C2, TF> Debug for TeeBase<C1, C2, TF>
where
    C1: Debug,
    C2: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeeBase")
            .field("collector1", &self.collector1)
            .field("collector2", &self.collector2)
            .finish()
    }
}

pub(super) trait DefinePassDown<'this, T: ?Sized, Binder: t_binder::Sealed = t_binder::Binder<'this, T>> {
    type PassDown;
}

/// Used for the hack. Should not be able to be referred outside.
mod t_binder {
    use std::marker::PhantomData;

    pub trait Sealed {}
    #[allow(missing_debug_implementations)]
    pub struct Binder<'a, T: ?Sized>(PhantomData<&'a mut T>);
    impl<'a, T: ?Sized> Sealed for Binder<'a, T> {}
}

pub(super) trait Teer<T>: Clone + Send + for<'this> DefinePassDown<'this, T> {
    const ITEM_IS_COPY: bool = false;

    fn pass_down<'a>(&mut self, item: &'a mut T) -> <Self as DefinePassDown<'a, T>>::PassDown;

    #[inline]
    fn no_tee_collect(
        &mut self,
        collector: &mut impl for<'a> Collector<<Self as DefinePassDown<'a, T>>::PassDown>,
        item: T,
    ) -> ControlFlow<()> {
        let mut item = item;
        collector.collect(self.pass_down(&mut item))
    }

    fn no_tee_collect_many(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: &mut impl for<'a> Collector<<Self as DefinePassDown<'a, T>>::PassDown>,
    ) -> ControlFlow<()> {
        items
            .into_iter()
            .try_for_each(|mut item| collector.collect(self.pass_down(&mut item)))
    }

    fn no_tee_collect_then_finish<O>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: impl for<'a> Collector<<Self as DefinePassDown<'a, T>>::PassDown, Output = O>,
    ) -> O {
        let mut collector = collector;
        let _ = items
            .into_iter()
            .try_for_each(|mut item| collector.collect(self.pass_down(&mut item)));
        collector.finish()
    }
}

impl<'this, C1, C2, TF> DefineSerial<'this> for TeeBase<C1, C2, TF>
where
    C1: DefineSerial<'this>,
    C2: DefineSerial<'this>,
    TF: Send + Clone,
{
    type Serial = unique::Serial<
        'this,
        Self,
        consumer::Serial<
            <Fuse<C1> as DefineSerial<'this>>::Serial,
            <Fuse<C2> as DefineSerial<'this>>::Serial,
            TF,
        >,
    >;
}

impl<'this, C1, C2, TF> DefineUnindexedSerial<'this> for TeeBase<C1, C2, TF>
where
    C1: DefineUnindexedSerial<'this>,
    C2: DefineUnindexedSerial<'this>,
    TF: Send + Clone,
{
    type UnindexedSerial = unique_unindexed::Serial<
        'this,
        Self,
        consumer::Serial<
            <Fuse<C1> as DefineUnindexedSerial<'this>>::UnindexedSerial,
            <Fuse<C2> as DefineUnindexedSerial<'this>>::UnindexedSerial,
            TF,
        >,
    >;
}

impl<C1, C2, TF> ParallelCollectorBase for TeeBase<C1, C2, TF>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
    TF: Clone + Send,
{
    type Output = (C1::Output, C2::Output);

    #[inline]
    fn finish(self) -> Self::Output {
        (self.collector1.finish(), self.collector2.finish())
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        if self.collector1.break_hint().is_break() && self.collector2.break_hint().is_break() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
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
        let (actual_len1, consumer1, commit1) = self.collector1.parts(len);
        let (actual_len2, consumer2, commit2) = self.collector2.parts(len);

        unique::uniquify((
            actual_len1.max(actual_len2),
            consumer::Consumer::new(consumer1, consumer2, self.teer.clone()),
            |(o1, o2)| and_cf_breaks(commit1(o1), commit2(o2)),
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
        let (actual_len1, consumer1, commit1) = self.collector1.take_parts(len);
        let (actual_len2, consumer2, commit2) = self.collector2.take_parts(len);

        unique::take_uniquify((
            actual_len1.max(actual_len2),
            consumer::Consumer::new(consumer1, consumer2, self.teer.clone()),
            |(o1, o2)| {
                commit1(o1);
                commit2(o2);
            },
        ))
    }
}

impl<C1, C2, TF> UnindexedParallelCollectorBase for TeeBase<C1, C2, TF>
where
    C1: UnindexedParallelCollectorBase,
    C2: UnindexedParallelCollectorBase,
    TF: Clone + Send,
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
        let (consumer1, commit1) = self.collector1.parts_unindexed();
        let (consumer2, commit2) = self.collector2.parts_unindexed();

        unique_unindexed::uniquify((
            consumer::Consumer::new(consumer1, consumer2, self.teer.clone()),
            |(o1, o2)| and_cf_breaks(commit1(o1), commit2(o2)),
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
        let (consumer1, commit1) = self.collector1.take_parts_unindexed();
        let (consumer2, commit2) = self.collector2.take_parts_unindexed();

        unique_unindexed::take_uniquify((
            consumer::Consumer::new(consumer1, consumer2, self.teer.clone()),
            |(o1, o2)| {
                commit1(o1);
                commit2(o2);
            },
        ))
    }
}

fn and_cf_breaks(cf1: ControlFlow<()>, cf2: ControlFlow<()>) -> ControlFlow<()> {
    if cf1.is_break() && cf2.is_break() {
        ControlFlow::Break(())
    } else {
        ControlFlow::Continue(())
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing;

    use super::DefinePassDown;

    pub struct Consumer<C1, C2, TF> {
        consumer1: C1,
        consumer2: C2,
        teer: TF,
    }

    impl<C1, C2, TF> Consumer<C1, C2, TF> {
        /// Both collectors are assumed to have been fused
        #[inline]
        pub(super) fn new(consumer1: C1, consumer2: C2, teer: TF) -> Self {
            Self {
                consumer1,
                consumer2,
                teer,
            }
        }
    }

    pub struct Combiner<C1, C2> {
        combiner1: C1,
        combiner2: C2,
    }

    // Unlike komadori's tee variants, the collectors here are obtained
    // from fused parallel collectors, which already guarantees fuse.
    pub struct Serial<C1, C2, TF> {
        collector1: C1,
        collector2: C2,
        teer: TF,
    }

    impl<C1, C2, TF> IntoCollectorBase for Consumer<C1, C2, TF>
    where
        C1: IntoCollectorBase,
        C2: IntoCollectorBase,
    {
        type Output = (C1::Output, C2::Output);

        type IntoCollector = Serial<C1::IntoCollector, C2::IntoCollector, TF>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector1: self.consumer1.into_collector(),
                collector2: self.consumer2.into_collector(),
                teer: self.teer,
            }
        }
    }

    impl<C1, C2, TF> plumbing::Consumer for Consumer<C1, C2, TF>
    where
        C1: plumbing::Consumer,
        C2: plumbing::Consumer,
        TF: Clone + Send,
    {
        type Combiner = Combiner<C1::Combiner, C2::Combiner>;

        #[inline]
        fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
            let (consumer1, combiner1) = self.consumer1.split_off_left_at(index);
            let (consumer2, combiner2) = self.consumer2.split_off_left_at(index);

            (
                Self {
                    consumer1,
                    consumer2,
                    teer: self.teer.clone(),
                },
                Combiner { combiner1, combiner2 },
            )
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            if self.consumer1.break_hint().is_break() && self.consumer2.break_hint().is_break() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }
    }

    impl<C1, C2, TF> plumbing::UnindexedConsumer for Consumer<C1, C2, TF>
    where
        C1: plumbing::UnindexedConsumer,
        C2: plumbing::UnindexedConsumer,
        TF: Clone + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer1: self.consumer1.split_off_left(),
                consumer2: self.consumer2.split_off_left(),
                teer: self.teer.clone(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner {
                combiner1: self.consumer1.to_combiner(),
                combiner2: self.consumer2.to_combiner(),
            }
        }
    }

    impl<C1, C2, O1, O2> plumbing::Combiner<(O1, O2)> for Combiner<C1, C2>
    where
        C1: plumbing::Combiner<O1>,
        C2: plumbing::Combiner<O2>,
    {
        #[inline]
        fn combine(self, (left1, left2): &mut (O1, O2), (right1, right2): (O1, O2)) {
            self.combiner1.combine(left1, right1);
            self.combiner2.combine(left2, right2);
        }
    }

    impl<C1, C2, TF> CollectorBase for Serial<C1, C2, TF>
    where
        C1: CollectorBase,
        C2: CollectorBase,
    {
        type Output = (C1::Output, C2::Output);

        #[inline]
        fn finish(self) -> Self::Output {
            (self.collector1.finish(), self.collector2.finish())
        }

        #[inline]
        fn break_hint(&self) -> ControlFlow<()> {
            if self.collector1.break_hint().is_break() && self.collector2.break_hint().is_break() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }
    }

    impl<C1, C2, TF, T> Collector<T> for Serial<C1, C2, TF>
    where
        C1: for<'a> Collector<<TF as DefinePassDown<'a, T>>::PassDown>,
        C2: Collector<T>,
        TF: super::Teer<T>,
    {
        #[inline]
        fn collect(&mut self, mut item: T) -> ControlFlow<()> {
            if TF::ITEM_IS_COPY {
                let _ = self.collector1.collect(self.teer.pass_down(&mut item));
                let _ = self.collector2.collect(item);
                self.break_hint()
            } else if self.collector2.break_hint().is_break() {
                self.teer.no_tee_collect(&mut self.collector1, item)
            } else if self.collector1.break_hint().is_break() {
                self.collector2.collect(item)
            } else {
                let _ = self.collector1.collect(self.teer.pass_down(&mut item));
                let _ = self.collector2.collect(item);
                self.break_hint()
            }
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            match (
                self.collector1.break_hint().is_break(),
                self.collector2.break_hint().is_break(),
            ) {
                (true, true) => return ControlFlow::Break(()),
                (false, true) => return self.teer.no_tee_collect_many(items, &mut self.collector1),
                (true, false) => return self.collector2.collect_many(items),
                (false, false) => {}
            }

            let mut items = items.into_iter();

            match items.try_for_each(|mut item| {
                if self.collector1.collect(self.teer.pass_down(&mut item)).is_break() {
                    ControlFlow::Break(Which::First(item))
                } else if self.collector2.collect(item).is_break() {
                    ControlFlow::Break(Which::Second)
                } else {
                    ControlFlow::Continue(())
                }
            }) {
                ControlFlow::Continue(_) => ControlFlow::Continue(()),
                ControlFlow::Break(Which::First(item)) => {
                    self.collector2.collect(item)?;
                    self.collector2.collect_many(items)
                }
                ControlFlow::Break(Which::Second) => {
                    self.teer.no_tee_collect_many(items, &mut self.collector1)
                }
            }
        }

        #[inline]
        fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
            match (
                self.collector1.break_hint().is_break(),
                self.collector2.break_hint().is_break(),
            ) {
                (true, true) => return self.finish(),
                (false, true) => {
                    return (
                        self.teer.no_tee_collect_then_finish(items, self.collector1),
                        self.collector2.finish(),
                    );
                }
                (true, false) => {
                    return (
                        self.collector1.finish(),
                        self.collector2.collect_then_finish(items),
                    );
                }
                (false, false) => {}
            }

            let mut items = items.into_iter();

            match items.try_for_each(|mut item| {
                if self.collector1.collect(self.teer.pass_down(&mut item)).is_break() {
                    ControlFlow::Break(Which::First(item))
                } else if self.collector2.collect(item).is_break() {
                    ControlFlow::Break(Which::Second)
                } else {
                    ControlFlow::Continue(())
                }
            }) {
                ControlFlow::Continue(_) => self.finish(),
                ControlFlow::Break(Which::First(item)) => {
                    // It's fused. We don't care.
                    let _ = self.collector2.collect(item);
                    (
                        self.collector1.finish(),
                        self.collector2.collect_then_finish(items),
                    )
                }
                ControlFlow::Break(Which::Second) => (
                    self.teer.no_tee_collect_then_finish(items, self.collector1),
                    self.collector2.finish(),
                ),
            }
        }
    }

    enum Which<T> {
        First(T),
        Second,
    }
}
