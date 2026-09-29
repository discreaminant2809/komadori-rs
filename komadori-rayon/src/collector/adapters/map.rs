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
    pub struct MapBase<C, F> {
        pub(super) collector: C,
        pub(super) f: F,
    }
}
// `trying_results()` and similar collectors rely on this.
pub(crate) use inner::MapBase;

/// A parallel collector that uses a function
/// to transform each collected item.
///
/// This `struct` is created by [`ParallelCollectorBase::map()`].
/// See its documentation for more.
pub type Map<C, F> = MapBase<C, BasicParClosure<F>>;

/// A parallel collector that uses a "consumer" and a shared state
/// to transform each collected item.
///
/// This `struct` is created by
/// [`ParallelCollectorBase::map_with()`].
/// See its documentation for more.
pub type MapWith<C, S, FF> = MapBase<C, AdvancedParClosure<S, FF>>;

impl<C, F> MapBase<C, F> {
    pub(crate) fn new_base(collector: C, f: F) -> Self {
        Self { collector, f }
    }
}

impl<C, F> Map<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self {
            collector,
            f: BasicParClosure::new(f),
        }
    }
}

impl<C, S, FF> MapWith<C, S, FF> {
    pub(in crate::collector) fn new(collector: C, shared_state: S, consumer: FF) -> Self {
        Self {
            collector,
            f: AdvancedParClosure::new(shared_state, consumer),
        }
    }
}

impl<'a, C, F> DefineSerial<'a> for MapBase<C, F>
where
    C: DefineSerial<'a>,
    F: ParallelFnMutBase,
{
    type Serial =
        unique::Serial<'a, Self, consumer::Serial<C::Serial, <F as DefineCallMut<'a>>::CallMut>>;
}

impl<'a, C, F> DefineUnindexedSerial<'a> for MapBase<C, F>
where
    C: DefineUnindexedSerial<'a>,
    F: ParallelFnMutBase,
{
    type UnindexedSerial = unique_unindexed::Serial<
        'a,
        Self,
        consumer::Serial<C::UnindexedSerial, <F as DefineCallMut<'a>>::CallMut>,
    >;
}

impl<C, F> ParallelCollectorBase for MapBase<C, F>
where
    C: ParallelCollectorBase,
    F: ParallelFnMutBase,
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
        unique::uniquify((
            consumer::Consumer::new(consumer, self.f.callable_mut()),
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
        let (consumer, commit) = self.collector.take_parts(len);
        unique::take_uniquify((
            consumer::Consumer::new(consumer, self.f.take_callable_mut()),
            commit,
        ))
    }
}

impl<C, F> UnindexedParallelCollectorBase for MapBase<C, F>
where
    C: UnindexedParallelCollectorBase,
    F: ParallelFnMutBase,
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
            consumer::Consumer::new(consumer, self.f.callable_mut()),
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
            consumer::Consumer::new(consumer, self.f.take_callable_mut()),
            commit,
        ))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::{collector::plumbing, ops::CallMut};

    pub struct Consumer<C, FF> {
        consumer: C,
        into_f: FF,
    }

    // Can't utilize from komadori's filter(), since it requires item type right away.
    pub struct Serial<C, F> {
        collector: C,
        f: F,
    }

    impl<C, F> Consumer<C, F> {
        #[inline]
        pub(super) fn new(consumer: C, into_f: F) -> Self {
            Self { consumer, into_f }
        }
    }

    impl<C, FF, F> IntoCollectorBase for Consumer<C, FF>
    where
        C: IntoCollectorBase,
        FF: FnOnce() -> F,
    {
        type Output = C::Output;

        type IntoCollector = Serial<C::IntoCollector, F>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                collector: self.consumer.into_collector(),
                f: (self.into_f)(),
            }
        }
    }

    impl<C, FF, F> plumbing::Consumer for Consumer<C, FF>
    where
        C: plumbing::Consumer,
        FF: FnOnce() -> F + Clone + Send,
    {
        #[inline]
        fn split_off_left_at(
            &mut self,
            index: usize,
        ) -> (
            Self,
            impl FnOnce(&mut Self::Output, Self::Output) + use<C, FF, F>,
        ) {
            let (consumer, combiner) = self.consumer.split_off_left_at(index);
            (
                Self {
                    consumer,
                    into_f: self.into_f.clone(),
                },
                combiner,
            )
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            self.consumer.max_afford(request)
        }
    }

    impl<C, FF, F> plumbing::UnindexedConsumer for Consumer<C, FF>
    where
        C: plumbing::UnindexedConsumer,
        FF: FnOnce() -> F + Clone + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                consumer: self.consumer.split_off_left(),
                into_f: self.into_f.clone(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> impl FnOnce(&mut Self::Output, Self::Output) + use<C, FF, F> {
            self.consumer.to_combiner()
        }
    }

    impl<C, F> CollectorBase for Serial<C, F>
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
        fn reserve(&mut self, additional: usize) {
            self.collector.reserve(additional);
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            self.collector.max_afford(request)
        }
    }

    impl<C, F, T> Collector<T> for Serial<C, F>
    where
        C: Collector<F::Output>,
        F: CallMut<(T,)>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            self.collector.collect(self.f.call_mut((item,)))
        }

        #[inline]
        unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
            unsafe {
                // SAFETY: The caller reserved for at least 1 item.
                self.collector
                    .assume_reserved_collect(self.f.call_mut((item,)))
            }
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            self.collector
                .collect_many(items.into_iter().map(|item| self.f.call_mut((item,))))
        }

        #[inline]
        fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
            self.collector
                .collect_then_finish(items.into_iter().map(move |item| self.f.call_mut((item,))))
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
        iter: nums.par_iter().cloned(),
        collector: vec![].into_par_collector().take(n).map(map_f),
        starting_ma_f: |request| n.min(request),
        expected_f: |iter, count, request| {
            let res: Vec<_> = iter.map(map_f).take(n).collect();
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
        collector: vec![].into_par_collector().take(n).map(map_f),
        starting_ma_f: |request| n.min(request),
        expected_f: |iter, count, request| {
            let res: Vec<_> = iter.map(map_f).collect();
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

    fn map_f(num: i32) -> i32 {
        num.wrapping_add(i32::MAX)
    }
}
