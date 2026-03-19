use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::{
    IndexedParallelCollector, ParallelCollector, ParallelCollectorBase, plumbing,
};

use super::Fuse;

///
#[derive(Debug, Clone)]
pub struct Tee<C1, C2> {
    collector1: Fuse<C1>,
    collector2: Fuse<C2>,
}

impl<C1, C2> Tee<C1, C2>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            collector1: collector1.fuse(),
            collector2: collector2.fuse(),
        }
    }
}

impl<C1, C2> ParallelCollectorBase for Tee<C1, C2>
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

impl<C1, C2, T> IndexedParallelCollector<T> for Tee<C1, C2>
where
    C1: IndexedParallelCollector<T>,
    C2: IndexedParallelCollector<T>,
    T: Copy,
{
    fn with_consumer<F>(&mut self, len: usize, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::ConsumerFnOnce<T>,
    {
        let (ret, _) = self.collector1.with_consumer(
            len,
            For1F {
                collector2: &mut self.collector2,
                len,
                f,
            },
        );

        return (ret, self.break_hint());

        struct For1F<'a, C2, F> {
            collector2: &'a mut C2,
            len: usize,
            f: F,
        }

        impl<C2, F, T> plumbing::ConsumerFnOnce<T> for For1F<'_, C2, F>
        where
            T: Copy,
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
                C: plumbing::Consumer<T>,
            {
                self.collector2
                    .with_consumer(
                        self.len,
                        For2F {
                            consumer1,
                            actual_len1,
                            f: self.f,
                        },
                    )
                    .0
            }
        }

        struct For2F<C1, F> {
            consumer1: C1,
            actual_len1: Option<usize>,
            f: F,
        }

        impl<C1, F, T> plumbing::ConsumerFnOnce<T> for For2F<C1, F>
        where
            T: Copy,
            C1: plumbing::Consumer<T>,
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
                let (ret, (output1, output2)) = self.f.call_once(
                    (move || Some(self.actual_len1?.max(actual_len2?)))(),
                    Consumer {
                        consumer1: self.consumer1,
                        consumer2,
                    },
                );
                ((ret, output1), output2)
            }
        }
    }

    fn with_consumer_then_finish<F>(self, len: usize, f: F) -> (F::Output, Self::Output)
    where
        F: plumbing::ConsumerFnOnce<T>,
    {
        let ((ret, output2), output1) = self.collector1.with_consumer_then_finish(
            len,
            For1F {
                collector2: self.collector2,
                len,
                f,
            },
        );

        return (ret, (output1, output2));

        struct For1F<C2, F> {
            collector2: C2,
            len: usize,
            f: F,
        }

        impl<C2, F, T> plumbing::ConsumerFnOnce<T> for For1F<C2, F>
        where
            T: Copy,
            C2: IndexedParallelCollector<T>,
            F: plumbing::ConsumerFnOnce<T>,
        {
            type Output = (F::Output, C2::Output);

            fn call_once<C>(
                self,
                actual_len1: Option<usize>,
                consumer1: C,
            ) -> (Self::Output, C::Output)
            where
                C: plumbing::Consumer<T>,
            {
                let ((ret, con_output1), output2) = self.collector2.with_consumer_then_finish(
                    self.len,
                    For2F {
                        consumer1,
                        actual_len1,
                        f: self.f,
                    },
                );

                ((ret, output2), con_output1)
            }
        }

        struct For2F<C1, F> {
            consumer1: C1,
            actual_len1: Option<usize>,
            f: F,
        }

        impl<C1, F, T> plumbing::ConsumerFnOnce<T> for For2F<C1, F>
        where
            T: Copy,
            C1: plumbing::Consumer<T>,
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
                let (ret, (con_output1, con_output2)) = self.f.call_once(
                    (move || Some(self.actual_len1?.max(actual_len2?)))(),
                    Consumer {
                        consumer1: self.consumer1,
                        consumer2,
                    },
                );
                ((ret, con_output1), con_output2)
            }
        }
    }
}

impl<C1, C2, T> ParallelCollector<T> for Tee<C1, C2>
where
    C1: ParallelCollector<T>,
    C2: ParallelCollector<T>,
    T: Copy,
{
    fn with_unindexed_consumer<F>(&mut self, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::UnindexedConsumerFnOnce<T>,
    {
        todo!()
    }
}

struct Consumer<C1, C2> {
    consumer1: C1,
    consumer2: C2,
}

struct Combiner<C1, C2> {
    combiner1: C1,
    combiner2: C2,
}

impl<C1, C2> IntoCollectorBase for Consumer<C1, C2>
where
    C1: IntoCollectorBase,
    C2: IntoCollectorBase,
{
    type Output = (C1::Output, C2::Output);

    type IntoCollector = komadori::collector::Tee<C1::IntoCollector, C2::IntoCollector>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        self.consumer1.into_collector().tee(self.consumer2)
    }
}

impl<C1, C2> plumbing::ConsumerBase for Consumer<C1, C2>
where
    C1: plumbing::ConsumerBase,
    C2: plumbing::ConsumerBase,
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

impl<C1, C2> plumbing::UnindexedConsumerBase for Consumer<C1, C2>
where
    C1: plumbing::UnindexedConsumerBase,
    C2: plumbing::UnindexedConsumerBase,
{
    #[inline]
    fn split_off_left(&self) -> Self {
        Self {
            consumer1: self.consumer1.split_off_left(),
            consumer2: self.consumer2.split_off_left(),
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
