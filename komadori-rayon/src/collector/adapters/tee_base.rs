use std::{marker::PhantomData, ops::ControlFlow};

use komadori::{collector::Fuse as SequentialFuse, prelude::*};

use crate::collector::{
    IndexedParallelCollector, ParallelCollector, ParallelCollectorBase, plumbing,
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
    pub fn new(collector1: C1, collector2: C2, teer: TF) -> Self {
        Self {
            collector1: collector1.fuse(),
            collector2: collector2.fuse(),
            teer,
        }
    }
}

pub trait Teer<T>: Clone + Send {
    const ITEM_IS_COPY: bool = false;

    type PassDown<'a>
    where
        T: 'a;

    fn pass_down<'a>(&mut self, item: &'a mut T) -> Self::PassDown<'a>;

    #[inline]
    fn no_tee_collect(
        &mut self,
        collector: &mut SequentialFuse<impl for<'a> Collector<Self::PassDown<'a>>>,
        item: T,
    ) -> ControlFlow<()> {
        let mut item = item;
        collector.collect(self.pass_down(&mut item))
    }

    fn no_tee_collect_many(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: &mut SequentialFuse<impl for<'a> Collector<Self::PassDown<'a>>>,
    ) -> ControlFlow<()> {
        items
            .into_iter()
            .try_for_each(|mut item| collector.collect(self.pass_down(&mut item)))
    }

    fn no_tee_collect_then_finish<O>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: SequentialFuse<impl for<'a> Collector<Self::PassDown<'a>, Output = O>>,
    ) -> O {
        let mut collector = collector;
        let _ = items
            .into_iter()
            .try_for_each(|mut item| collector.collect(self.pass_down(&mut item)));
        collector.finish()
    }
}

impl<C1, C2, TF> ParallelCollectorBase for TeeBase<C1, C2, TF>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
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

impl<C1, C2, TF, T> IndexedParallelCollector<T> for TeeBase<C1, C2, TF>
where
    C1: for<'a> IndexedParallelCollector<TF::PassDown<'a>>,
    C2: IndexedParallelCollector<T>,
    TF: Teer<T>,
{
    fn with_consumer<F>(&mut self, len: usize, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::ConsumerFnOnce<T>,
    {
        let (ret, _) = self.collector1.with_consumer(
            len,
            For1F {
                collector2: &mut self.collector2,
                teer: self.teer.clone(),
                len,
                f,
                _marker: PhantomData,
            },
        );

        return (ret, self.break_hint());

        struct For1F<'a, C2, TF, F, T> {
            collector2: &'a mut C2,
            teer: TF,
            len: usize,
            f: F,
            _marker: PhantomData<fn(T)>,
        }

        impl<'a, C2, F, TF, T> plumbing::ConsumerFnOnce<TF::PassDown<'a>> for For1F<'_, C2, TF, F, T>
        where
            TF: Teer<T>,
            C2: IndexedParallelCollector<T>,
            F: plumbing::ConsumerFnOnce<T>,
        {
            type Output = F::Output;

            fn call_once<C>(
                self,
                actual_len1: Option<usize>,
                consumer1: C,
            ) -> (Self::Output, C::Output)
            where
                C: plumbing::Consumer<TF::PassDown<'a>>,
            {
                self.collector2
                    .with_consumer(
                        self.len,
                        For2F {
                            consumer1,
                            teer: self.teer,
                            actual_len1,
                            f: self.f,
                        },
                    )
                    .0
            }
        }

        struct For2F<C1, TF, F> {
            consumer1: C1,
            teer: TF,
            actual_len1: Option<usize>,
            f: F,
        }

        impl<'a, C1, TF, F, T: 'a> plumbing::ConsumerFnOnce<T> for For2F<C1, TF, F>
        where
            TF: Teer<T>,
            C1: plumbing::Consumer<TF::PassDown<'a>>,
            F: plumbing::ConsumerFnOnce<T>,
        {
            type Output = (F::Output, C1::Output);

            fn call_once<C>(
                self,
                actual_len2: Option<usize>,
                consumer2: C,
            ) -> (Self::Output, C::Output)
            where
                C: plumbing::Consumer<T>,
            {
                // let (ret, (output1, output2)) = self.f.call_once(
                //     (move || Some(self.actual_len1?.max(actual_len2?)))(),
                //     Consumer {
                //         consumer1: self.consumer1,
                //         consumer2,
                //         teer: self.teer,
                //     },
                // );
                // ((ret, output1), output2)
                todo!()
            }
        }
    }
}

struct Consumer<C1, C2, TF> {
    consumer1: C1,
    consumer2: C2,
    teer: TF,
}

struct Combiner<C1, C2> {
    combiner1: C1,
    combiner2: C2,
}

struct IntoCollector<C1, C2, TF> {
    collector1: SequentialFuse<C1>,
    collector2: SequentialFuse<C2>,
    teer: TF,
}

impl<C1, C2, TF> IntoCollectorBase for Consumer<C1, C2, TF>
where
    C1: IntoCollectorBase,
    C2: IntoCollectorBase,
{
    type Output = (C1::Output, C2::Output);

    type IntoCollector = IntoCollector<C1::IntoCollector, C2::IntoCollector, TF>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        IntoCollector {
            collector1: self.consumer1.into_collector().fuse(),
            collector2: self.consumer2.into_collector().fuse(),
            teer: self.teer,
        }
    }
}

impl<C1, C2, TF> plumbing::ConsumerBase for Consumer<C1, C2, TF>
where
    C1: plumbing::ConsumerBase,
    C2: plumbing::ConsumerBase,
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
            Combiner {
                combiner1,
                combiner2,
            },
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

impl<C1, C2, TF> plumbing::UnindexedConsumerBase for Consumer<C1, C2, TF>
where
    C1: plumbing::UnindexedConsumerBase,
    C2: plumbing::UnindexedConsumerBase,
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

impl<C1, C2, TF> CollectorBase for IntoCollector<C1, C2, TF>
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

impl<C1, C2, TF, T> Collector<T> for IntoCollector<C1, C2, TF>
where
    C1: for<'a> Collector<TF::PassDown<'a>>,
    C2: Collector<T>,
    TF: Teer<T>,
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
            if self
                .collector1
                .collect(self.teer.pass_down(&mut item))
                .is_break()
            {
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
}

enum Which<T> {
    First(T),
    Second,
}
