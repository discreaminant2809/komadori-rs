//! Parallel collectors for the unit type.

use std::{fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{
        IntoParallelCollectorBase, ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that always stops accumulating.
/// It can collect every item type.
/// Its [`Output`](ParallelCollectorBase::Output) is `()`.
///
/// This struct is created by `().into_par_collector()`
/// and `().par_collector()`.
#[derive(Clone, Default)]
pub struct ParCollector(());

impl Debug for ParCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParCollector").finish()
    }
}

impl IntoParallelCollectorBase for () {
    type Output = ();

    type IntoParCollector = ParCollector;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        ParCollector::default()
    }
}

impl IntoParallelCollectorBase for &() {
    type Output = ();

    type IntoParCollector = ParCollector;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        ParCollector::default()
    }
}

impl<'a> DefineSerial<'a> for ParCollector {
    type Serial = unique::Serial<'a, Self, consumer::Serial>;
}

impl<'a> DefineUnindexedSerial<'a> for ParCollector {
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial>;
}

impl ParallelCollectorBase for ParCollector {
    type Output = ();

    #[inline]
    fn finish(self) -> Self::Output {}

    #[inline]
    fn max_afford(&self, _request: usize) -> usize {
        0
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::Consumer, |_| ControlFlow::Break(())))
    }
}

impl UnindexedParallelCollectorBase for ParCollector {
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((consumer::Consumer, |_| ControlFlow::Break(())))
    }
}

mod consumer {
    use komadori::prelude::*;

    use crate::collector::plumbing;

    pub struct Consumer;

    pub struct Combiner;

    pub type Serial = <() as IntoCollectorBase>::IntoCollector;

    impl IntoCollectorBase for Consumer {
        type Output = ();

        type IntoCollector = Serial;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            ().into_collector()
        }
    }

    impl plumbing::Consumer for Consumer {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (Self, Combiner)
        }

        #[inline]
        fn max_afford(&self, _request: usize) -> usize {
            0
        }
    }

    impl plumbing::UnindexedConsumer for Consumer {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner
        }
    }

    impl plumbing::Combiner<()> for Combiner {
        fn combine(self, _: &mut (), _: ()) {}
    }
}
