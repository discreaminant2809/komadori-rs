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
    ops::{AdvancedParClosure, BasicParClosure, DefineCallMut, ParallelFnMutBase},
};

// So that we can hide this struct while still be able to satisfy the compiler.
mod inner {
    #[derive(Clone, Debug)]
    pub struct FilterBase<C, P> {
        pub(super) collector: C,
        pub(super) pred: P,
    }
}
use inner::FilterBase;

/// A parallel collector that uses a function to determine whether
/// an item should be accumulated.
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::filter()`].
/// See its documentation for more.
pub type Filter<C, P> = FilterBase<C, BasicParClosure<P>>;

/// A parallel collector that uses a "consumer" and a shared state
/// to determine whether an item should be accumulated.
///
/// This `struct` is created by
/// [`UnindexedParallelCollectorBase::filter_with()`].
/// See its documentation for more.
pub type FilterWith<C, S, FP> = FilterBase<C, AdvancedParClosure<S, FP>>;

impl<C, S, FP> FilterWith<C, S, FP> {
    pub(in crate::collector) fn new(collector: C, shared_state: S, consumer: FP) -> Self {
        Self {
            collector,
            pred: AdvancedParClosure::new(shared_state, consumer),
        }
    }
}

impl<C, P> Filter<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self {
            collector,
            pred: BasicParClosure::new(pred),
        }
    }
}

impl<'a, C, P> DefineSerial<'a> for FilterBase<C, P>
where
    C: DefineUnindexedSerial<'a>,
    P: ParallelFnMutBase,
{
    type Serial = unique::Serial<
        'a,
        Self,
        consumer::Serial<C::UnindexedSerial, <P as DefineCallMut<'a>>::CallMut>,
    >;
}

impl<'a, C, P> DefineUnindexedSerial<'a> for FilterBase<C, P>
where
    C: DefineUnindexedSerial<'a>,
    P: ParallelFnMutBase,
{
    type UnindexedSerial = unique_unindexed::Serial<
        'a,
        Self,
        consumer::Serial<C::UnindexedSerial, <P as DefineCallMut<'a>>::CallMut>,
    >;
}

impl<C, P> ParallelCollectorBase for FilterBase<C, P>
where
    C: UnindexedParallelCollectorBase,
    P: ParallelFnMutBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.collector.max_afford(1) == 0 {
            0
        } else {
            request
        }
    }

    #[inline]
    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        unique::uniquify((
            consumer::Consumer::new(consumer, self.pred.callable_mut()),
            commit,
        ))
    }

    #[inline]
    fn take_parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique::take_uniquify((
            consumer::Consumer::new(consumer, self.pred.take_callable_mut()),
            commit,
        ))
    }
}

impl<C, P> UnindexedParallelCollectorBase for FilterBase<C, P>
where
    C: UnindexedParallelCollectorBase,
    P: ParallelFnMutBase,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts_unindexed();
        unique_unindexed::uniquify((
            consumer::Consumer::new(consumer, self.pred.callable_mut()),
            commit,
        ))
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        let (consumer, commit) = self.collector.take_parts_unindexed();
        unique_unindexed::take_uniquify((
            consumer::Consumer::new(consumer, self.pred.take_callable_mut()),
            commit,
        ))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::{
        collector::plumbing::{self, UnindexedConsumer},
        ops::CallMut,
    };

    pub struct Consumer<C, PF> {
        consumer: C,
        into_pred: PF,
    }

    // Can't utilize from komadori's filter(), since it requires item type right away.
    pub struct Serial<C, P> {
        collector: C,
        pred: P,
    }

    impl<C, P> Consumer<C, P> {
        #[inline]
        pub(super) fn new(consumer: C, into_pred: P) -> Self {
            Self {
                consumer,
                into_pred,
            }
        }
    }

    impl<C, PF, P> IntoCollectorBase for Consumer<C, PF>
    where
        C: IntoCollectorBase,
        PF: FnOnce() -> P,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector, P>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector: self.consumer.into_collector(),
                pred: (self.into_pred)(),
            }
        }
    }

    impl<C, PF, P> plumbing::Consumer for Consumer<C, PF>
    where
        C: plumbing::UnindexedConsumer,
        PF: FnOnce() -> P + Clone + Send,
    {
        type Combiner = C::Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            if self.consumer.max_afford(1) == 0 {
                0
            } else {
                request
            }
        }
    }

    impl<C, PF, P> plumbing::UnindexedConsumer for Consumer<C, PF>
    where
        C: plumbing::UnindexedConsumer,
        PF: FnOnce() -> P + Clone + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                into_pred: self.into_pred.clone(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            self.consumer.to_combiner()
        }
    }

    impl<C, P> CollectorBase for Serial<C, P>
    where
        C: CollectorBase,
    {
        type Output = C::Output;

        #[inline]
        fn finish(self) -> Self::Output {
            self.collector.finish()
        }

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            if self.collector.max_afford(1) == 0 {
                0
            } else {
                request
            }
        }
    }

    impl<C, P, T> Collector<T> for Serial<C, P>
    where
        C: Collector<T>,
        P: for<'a> CallMut<(&'a T,), Output = bool>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            if self.pred.call_mut((&item,)) {
                self.collector.collect(item)
            } else {
                plumbing::break_hint(&self.collector)
            }
        }

        // Removed the overriden implementations cuz the items here are being consumed
        // without consulting the underlying collector's break hint during filtering.
        // Yes, the performance degrades, but it's because of `try_for_each()` and/or
        // LLVM noise (which could be fixed soon),
        // and in multiple reduction it still works well and performs similarly to fold().
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
        iter: nums.par_iter().cloned(),
        collector: vec![].into_par_collector().take(n).filter(pred),
        starting_ma_f: |request| if n > 0 { request } else { 0 },
        expected_f: |iter, _, request| {
            let res: Vec<_> = iter.filter(pred).collect();
            let res_len = res.len();

            (
                res,
                if res_len < n {
                    Continue(((), request))
                } else {
                    Break(())
                },
            )
        },
        output_pred: |actual, expected| actual.len() <= nums.len().min(n)
            && is_subsequence(actual, expected),
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
        collector: vec![].into_par_collector().take(n).filter(pred),
        starting_ma_f: |request| if n > 0 { request } else { 0 },
        expected_f: |iter, _, request| {
            let res: Vec<_> = iter.filter(pred).collect();
            let res_len = res.len();

            (
                res,
                if res_len < n {
                    Continue(((), request))
                } else {
                    Break(())
                },
            )
        },
        output_pred: |actual, expected| actual.len() <= nums.len().min(n)
            && is_subsequence(actual, expected),
        state_pred: state_is_irrelevant(),
    });

    fn pred(&num: &i32) -> bool {
        num >= 0
    }
}
