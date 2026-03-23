//!

use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::{
    IndexedParallelCollector, IndexedParallelCollectorBase, IntoIndexedParallelCollectorBase,
    ParallelCollector, plumbing,
};

///
#[derive(Debug, Clone, Default)]
pub struct ParCollector(());

impl IntoIndexedParallelCollectorBase for () {
    type Output = ();

    type IntoParCollector = ParCollector;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        ParCollector::default()
    }
}

impl IntoIndexedParallelCollectorBase for &() {
    type Output = ();

    type IntoParCollector = ParCollector;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        ParCollector::default()
    }
}

impl IndexedParallelCollectorBase for ParCollector {
    type Output = ();

    #[inline]
    fn finish(self) -> Self::Output {}

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Break(())
    }
}

impl<T> IndexedParallelCollector<T> for ParCollector {
    #[inline]
    fn with_consumer<F>(&mut self, _: usize, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::ConsumerFnOnce<T>,
    {
        (f.call_once(Some(0), Consumer).0, ControlFlow::Break(()))
    }
}

impl<T> ParallelCollector<T> for ParCollector {
    fn with_unindexed_consumer<F>(&mut self, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::UnindexedConsumerFnOnce<T>,
    {
        (f.call_once(Consumer).0, ControlFlow::Break(()))
    }
}

struct Consumer;

struct Combiner;

impl IntoCollectorBase for Consumer {
    type Output = ();

    type IntoCollector = <() as IntoCollectorBase>::IntoCollector;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        ().into_collector()
    }
}

impl plumbing::IndexedConsumerBase for Consumer {
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

impl plumbing::ConsumerBase for Consumer {
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
