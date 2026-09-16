use std::ops::ControlFlow;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{
            CollectorBase, Consumer, ConsumerExt, DefineSerial, DefineUnindexedSerial, SerialOf,
            SerialOutputOf, UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that feeds the underlying parallel collector with
/// the mutable reference to the item, "pretending" the parallel collector
/// accepts owned items.
///
/// This `struct` is created by [`ParallelCollectorBase::funnel()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct Funnel<C>(C);

impl<C> Funnel<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self(collector)
    }
}

impl<'a, C> DefineSerial<'a> for Funnel<C>
where
    C: DefineSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<C::Serial>>;
}

impl<'a, C> DefineUnindexedSerial<'a> for Funnel<C>
where
    C: DefineUnindexedSerial<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<C::UnindexedSerial>>;
}

impl<C> ParallelCollectorBase for Funnel<C>
where
    C: ParallelCollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0.finish()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.0.max_afford(request)
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.0.parts(len);
        unique::uniquify((
            consumer.map_collector(|collector| collector.funnel()),
            commit,
        ))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        let (consumer, commit) = self.0.take_parts(len);
        unique::take_uniquify((
            consumer.map_collector(|collector| collector.funnel()),
            commit,
        ))
    }
}

impl<C> UnindexedParallelCollectorBase for Funnel<C>
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
        let (consumer, commit) = self.0.unindexed_parts();
        unique_unindexed::uniquify((
            consumer.map_collector(|collector| collector.funnel()),
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
        let (consumer, commit) = self.0.take_unindexed_parts();
        unique_unindexed::take_uniquify((
            consumer.map_collector(|collector| collector.funnel()),
            commit,
        ))
    }
}

mod consumer {
    use komadori::collector::Funnel;

    pub type Serial<C> = Funnel<C>;
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
        iter: nums.par_iter().cloned(),
        collector: Vec::<i32>::new().into_par_collector().take(n).funnel(),
        starting_ma_f: |request| n.min(request),
        expected_f: |iter, count, request| {
            let res: Vec<_> = iter.take(n).collect();
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
        iter: nums.par_iter().cloned(),
        collector: Vec::<i32>::new().into_par_collector().take(n).funnel(),
        starting_ma_f: |request| n.min(request),
        expected_f: |iter, count, request| {
            let res: Vec<_> = iter.collect();
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
