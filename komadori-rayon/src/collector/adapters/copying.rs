use std::ops::ControlFlow;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that copies every collected item.
///
/// This `struct` is created by [`ParallelCollectorBase::copying()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct Copying<C> {
    collector: C,
}

impl<C> Copying<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self { collector }
    }
}

impl<'a, C> DefineSerial<'a> for Copying<C>
where
    C: DefineSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<C::Serial>>;
}

impl<'a, C> DefineUnindexedSerial<'a> for Copying<C>
where
    C: DefineUnindexedSerial<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<C::UnindexedSerial>>;
}

impl<C> ParallelCollectorBase for Copying<C>
where
    C: ParallelCollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request)
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts(len);
        unique::uniquify((consumer::Consumer::new(consumer), commit))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        let (consumer, commit) = self.collector.take_parts(len);
        unique::take_uniquify((consumer::Consumer::new(consumer), commit))
    }
}

impl<C> UnindexedParallelCollectorBase for Copying<C>
where
    C: UnindexedParallelCollectorBase,
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
        unique_unindexed::uniquify((consumer::Consumer::new(consumer), commit))
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
        unique_unindexed::take_uniquify((consumer::Consumer::new(consumer), commit))
    }
}

mod consumer {
    use komadori::prelude::*;

    use crate::collector::plumbing;

    pub struct Consumer<C> {
        consumer: C,
    }

    pub type Serial<C> = komadori::collector::Copying<C>;

    impl<C> Consumer<C> {
        #[inline]
        pub fn new(consumer: C) -> Self {
            Self { consumer }
        }
    }

    impl<C> IntoCollectorBase for Consumer<C>
    where
        C: IntoCollectorBase,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            self.consumer.into_collector().copying()
        }
    }

    impl<C> plumbing::Consumer for Consumer<C>
    where
        C: plumbing::Consumer,
    {
        #[inline]
        fn split_off_left_at(
            &mut self,
            index: usize,
        ) -> (Self, impl FnOnce(&mut Self::Output, Self::Output) + use<C>) {
            let (consumer, combine) = self.consumer.split_off_left_at(index);
            (Self { consumer }, combine)
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            self.consumer.max_afford(request)
        }
    }

    impl<C> plumbing::UnindexedConsumer for Consumer<C>
    where
        C: plumbing::UnindexedConsumer,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> impl FnOnce(&mut Self::Output, Self::Output) + use<C> {
            self.consumer.to_combiner()
        }
    }
}

#[cfg(test)]
mod proptests {
    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let mut n = ..=5_usize;
        },
        iter: nums.par_iter(),
        collector: vec![].into_par_collector().take(n).copying(),
        starting_ma_f: |request| n.min(request),
        expected_f: |iter, count, request| {
            let res: Vec<_> = iter.copied().take(n).collect();
            (
                res,
                if let remaining @ 1.. = n.saturating_sub(count) {
                    Continue(((), remaining.min(request)))
                } else {
                    Break(())
                },
            )
        },
        output_pred: PartialEq::eq,
        state_pred: state_is_irrelevant(),
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let mut n = ..=5_usize;
        },
        iter: nums.par_iter(),
        collector: vec![].into_par_collector().take(n).copying(),
        starting_ma_f: |request| n.min(request),
        expected_f: |iter, count, request| {
            let res: Vec<_> = iter.copied().collect();
            (
                res,
                if let remaining @ 1.. = n.saturating_sub(count) {
                    Continue(((), remaining.min(request)))
                } else {
                    Break(())
                },
            )
        },
        output_pred: |actual, expected| actual.len() == nums.len().min(n)
            && is_subsequence(actual, expected),
        state_pred: state_is_irrelevant(),
    });
}
