#![allow(missing_debug_implementations)]

use std::{collections::LinkedList, marker::PhantomData, ops::ControlFlow};

use komadori::prelude::*;

use crate::collector::plumbing;

pub struct Consumer<T> {
    _marker: PhantomData<T>,
}

pub struct Serial<T> {
    chunk: Vec<T>,
}

pub struct Combiner(());

impl<T> Consumer<T> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<T> IntoCollectorBase for Consumer<T> {
    type Output = (LinkedList<Vec<T>>, usize);

    type IntoCollector = Serial<T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        Serial { chunk: vec![] }
    }
}

impl<T> plumbing::Consumer for Consumer<T>
where
    T: Send,
{
    type Combiner = Combiner;

    #[inline]
    fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
        use plumbing::UnindexedConsumer;
        (self.split_off_left(), self.to_combiner())
    }
}

impl<T> plumbing::UnindexedConsumer for Consumer<T>
where
    T: Send,
{
    #[inline]
    fn split_off_left(&self) -> Self {
        Consumer { _marker: PhantomData }
    }

    #[inline]
    fn to_combiner(&self) -> Self::Combiner {
        Combiner(())
    }
}

impl<T> plumbing::Combiner<(LinkedList<Vec<T>>, usize)> for Combiner {
    #[inline]
    fn combine(self, left: &mut (LinkedList<Vec<T>>, usize), mut right: (LinkedList<Vec<T>>, usize)) {
        left.0.append(&mut right.0);
        left.1 += right.1;
    }
}

impl<T> CollectorBase for Serial<T> {
    type Output = (LinkedList<Vec<T>>, usize);

    #[inline]
    fn finish(self) -> Self::Output {
        let len = self.chunk.len();
        (
            if len == 0 {
                LinkedList::new()
            } else {
                [self.chunk].into()
            },
            len,
        )
    }
}

impl<T> Collector<T> for Serial<T> {
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

impl<'i, T> Collector<&'i T> for Serial<T>
where
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

impl<'i, T> Collector<&'i mut T> for Serial<T>
where
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
