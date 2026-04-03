#![allow(missing_debug_implementations)]

use std::{collections::LinkedList, marker::PhantomData, ops::ControlFlow};

use komadori::prelude::*;

use crate::collector::plumbing;

trait LenCarrier {
    fn new(len: usize) -> Self;
    fn combine(&mut self, other: Self);
}

impl LenCarrier for usize {
    #[inline]
    fn new(len: usize) -> Self {
        len
    }

    #[inline]
    fn combine(&mut self, other: Self) {
        *self += other;
    }
}

impl LenCarrier for () {
    #[inline]
    fn new(_: usize) -> Self {}

    #[inline]
    fn combine(&mut self, _: Self) {}
}

pub struct Consumer<T, L> {
    _marker: PhantomData<(T, L)>,
}

pub struct IntoCollector<T, L> {
    chunk: Vec<T>,
    _marker: PhantomData<L>,
}

pub struct Combiner(());

impl<T, L> Consumer<T, L> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T, L> IntoCollectorBase for Consumer<T, L>
where
    L: LenCarrier,
{
    type Output = (LinkedList<Vec<T>>, L);

    type IntoCollector = IntoCollector<T, L>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        IntoCollector {
            chunk: vec![],
            _marker: PhantomData,
        }
    }
}

impl<T, L> plumbing::ConsumerBase for Consumer<T, L>
where
    T: Send,
    L: LenCarrier + Send,
{
    type Combiner = Combiner;

    #[inline]
    fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
        use plumbing::UnindexedConsumerBase;
        (self.split_off_left(), self.to_combiner())
    }
}

impl<T, L> plumbing::UnindexedConsumerBase for Consumer<T, L>
where
    T: Send,
    L: LenCarrier + Send,
{
    #[inline]
    fn split_off_left(&self) -> Self {
        Consumer {
            _marker: PhantomData,
        }
    }

    #[inline]
    fn to_combiner(&self) -> Self::Combiner {
        Combiner(())
    }
}

impl<T, L> plumbing::Combiner<(LinkedList<Vec<T>>, L)> for Combiner
where
    L: LenCarrier,
{
    #[inline]
    fn combine(self, left: &mut (LinkedList<Vec<T>>, L), mut right: (LinkedList<Vec<T>>, L)) {
        left.0.append(&mut right.0);
        left.1.combine(right.1);
    }
}

impl<T, L> CollectorBase for IntoCollector<T, L>
where
    L: LenCarrier,
{
    type Output = (LinkedList<Vec<T>>, L);

    #[inline]
    fn finish(self) -> Self::Output {
        let len_carrier = L::new(self.chunk.len());
        ([self.chunk].into(), len_carrier)
    }
}

impl<T, L> Collector<T> for IntoCollector<T, L>
where
    L: LenCarrier,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.chunk.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.chunk.extend(items);
        ControlFlow::Continue(())
    }
}

impl<'i, T, L> Collector<&'i T> for IntoCollector<T, L>
where
    L: LenCarrier,
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        self.chunk.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i T>) -> ControlFlow<()> {
        self.chunk.extend(items);
        ControlFlow::Continue(())
    }
}

impl<'i, T, L> Collector<&'i mut T> for IntoCollector<T, L>
where
    L: LenCarrier,
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        self.chunk.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut T>) -> ControlFlow<()> {
        self.chunk.extend(items.into_iter().map(|&mut item| item));
        ControlFlow::Continue(())
    }
}
