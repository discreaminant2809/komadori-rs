use super::{IndexedParallelIterator, ParallelIterator};

#[derive(Clone)]
pub struct Cloned<I> {
    iter: I,
}

impl<I> Cloned<I> {
    pub(super) fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<'a, I, T> ParallelIterator for Cloned<I>
where
    I: ParallelIterator<Item = &'a T>,
    T: Clone + 'a,
{
    type Item = T;

    fn take_producer(&mut self) -> impl super::Producer<Item = Self::Item> {
        Producer {
            producer: self.iter.take_producer(),
        }
    }

    fn count(self) -> usize {
        self.iter.count()
    }
}

impl<'a, I, T> IndexedParallelIterator for Cloned<I>
where
    I: IndexedParallelIterator<Item = &'a T>,
    T: Clone + 'a,
{
    fn indexed_producer(&mut self) -> impl super::IndexedProducer<Item = Self::Item> {
        Producer {
            producer: self.iter.indexed_producer(),
        }
    }

    fn len(&self) -> usize {
        self.iter.len()
    }
}

struct Producer<P> {
    producer: P,
}

impl<'a, P, T> super::Producer for Producer<P>
where
    P: super::Producer<Item = &'a T>,
    T: Clone + 'a,
{
    type Item = T;

    fn into_iter(self) -> impl Iterator<Item = Self::Item> {
        self.producer.into_iter().cloned()
    }

    fn split_off_left(&mut self) -> Self {
        Self {
            producer: self.producer.split_off_left(),
        }
    }
}

impl<'a, P, T> super::IndexedProducer for Producer<P>
where
    P: super::IndexedProducer<Item = &'a T>,
    T: Clone + 'a,
{
    type Item = T;

    fn into_iter(self) -> impl Iterator<Item = Self::Item> {
        self.producer.into_iter().cloned()
    }

    fn len(&self) -> usize {
        self.producer.len()
    }

    fn split_off_left_at(&mut self, index: usize) -> Self {
        Self {
            producer: self.producer.split_off_left_at(index),
        }
    }
}
