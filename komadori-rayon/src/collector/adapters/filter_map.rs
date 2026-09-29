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
    pub struct FilterMapBase<C, P> {
        pub(super) collector: C,
        pub(super) pred: P,
    }
}
use inner::FilterMapBase;

/// A parallel collector that both filters and maps each item before collecting.
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::filter_map()`].
/// See its documentation for more.
pub type FilterMap<C, P> = FilterMapBase<C, BasicParClosure<P>>;

/// A parallel collector that uses a "consumer" and a shared state
/// to both filters and maps each item before collecting.
///
/// This `struct` is created by
/// [`UnindexedParallelCollectorBase::filter_map_with()`].
/// See its documentation for more.
pub type FilterMapWith<C, S, FP> = FilterMapBase<C, AdvancedParClosure<S, FP>>;

impl<C, P> FilterMap<C, P> {
    pub(in crate::collector) fn new(collector: C, pred: P) -> Self {
        Self {
            collector,
            pred: BasicParClosure::new(pred),
        }
    }
}

impl<C, S, FP> FilterMapWith<C, S, FP> {
    pub(in crate::collector) fn new(collector: C, shared_state: S, consumer: FP) -> Self {
        Self {
            collector,
            pred: AdvancedParClosure::new(shared_state, consumer),
        }
    }
}

impl<'a, C, P> DefineSerial<'a> for FilterMapBase<C, P>
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

impl<'a, C, P> DefineUnindexedSerial<'a> for FilterMapBase<C, P>
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

impl<C, P> ParallelCollectorBase for FilterMapBase<C, P>
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
        let (consumer, commit) = self.collector.unindexed_parts();
        unique::uniquify((
            consumer::unindexed(consumer, self.pred.callable_mut()),
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
        let (consumer, commit) = self.collector.take_unindexed_parts();
        unique::take_uniquify((
            consumer::unindexed(consumer, self.pred.take_callable_mut()),
            commit,
        ))
    }
}

impl<C, P> UnindexedParallelCollectorBase for FilterMapBase<C, P>
where
    C: UnindexedParallelCollectorBase,
    P: ParallelFnMutBase,
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
            consumer::unindexed(consumer, self.pred.callable_mut()),
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
            consumer::unindexed(consumer, self.pred.take_callable_mut()),
            commit,
        ))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::{
        collector::plumbing::{
            self, BasicUnindexedConsumer, OpaqueUnindexedConsumer, UnindexedConsumer,
        },
        ops::CallMut,
    };

    pub fn unindexed<C, P>(
        consumer: C,
        into_pred: impl FnOnce() -> P + Clone + Send,
    ) -> OpaqueUnindexedConsumer!(Serial<C::IntoCollector, P>)
    where
        C: UnindexedConsumer,
    {
        BasicUnindexedConsumer {
            state: (consumer, into_pred),
            split_f: |(consumer, into_pred)| (consumer.split_off_left(), into_pred.clone()),
            combiner_f: |(consumer, _)| consumer.to_combiner(),
            ma_f: |(consumer, _), request| {
                if consumer.max_afford(1) == 0 {
                    0
                } else {
                    request
                }
            },
            collector_f: |(consumer, into_pred)| Serial {
                collector: consumer.into_collector(),
                pred: into_pred(),
            },
        }
    }

    pub struct Serial<C, P> {
        collector: C,
        pred: P,
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

    impl<C, P, T, R> Collector<T> for Serial<C, P>
    where
        C: Collector<R>,
        P: CallMut<(T,), Output = Option<R>>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            if let Some(item) = self.pred.call_mut((item,)) {
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
        collector: vec![].into_par_collector().take(n).filter_map(f),
        starting_ma_f: |request| if n > 0 { request } else { 0 },
        expected_f: |iter, _, request| {
            let res: Vec<_> = iter.filter_map(f).collect();
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
        collector: vec![].into_par_collector().take(n).filter_map(f),
        starting_ma_f: |request| if n > 0 { request } else { 0 },
        expected_f: |iter, _, request| {
            let res: Vec<_> = iter.filter_map(f).collect();
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

    fn f(num: i32) -> Option<i32> {
        num.checked_add(i32::MAX)
    }
}
