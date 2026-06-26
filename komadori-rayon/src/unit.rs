//! Parallel collectors for the unit type.

use std::{fmt::Debug, ops::ControlFlow};

use komadori::prelude::*;

use crate::{
    collector::{
        IntoParallelCollectorBase, ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{self, DefineSerial, DefineUnindexedSerial},
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
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Break(())
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl plumbing::Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        unique::uniquify((len, consumer::Consumer, |_| ControlFlow::Break(())))
    }
}

impl UnindexedParallelCollectorBase for ParCollector {
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl plumbing::UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((consumer::Consumer, |_| ControlFlow::Break(())))
    }
}

mod consumer {
    use std::ops::ControlFlow;

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
        fn break_hint(&self) -> ControlFlow<()> {
            ControlFlow::Break(())
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
