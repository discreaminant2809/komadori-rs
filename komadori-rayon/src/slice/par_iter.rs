use crate::test_utils::{
    IndexedParallelIterator, IndexedProducer, IntoParallelIterator, ParallelIterator, Producer as IProducer,
};

pub struct ParIter<'a, T>(&'a [T]);

impl<T> Clone for ParIter<'_, T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<'a, T: 'a> IntoParallelIterator for &'a [T] {
    type Item = &'a T;

    type IntoParIter = ParIter<'a, T>;

    fn into_par_iter(self) -> Self::IntoParIter {
        ParIter(self)
    }
}

impl<'a, T> ParallelIterator for ParIter<'a, T> {
    type Item = &'a T;

    fn take_producer(&mut self) -> impl IProducer<Item = Self::Item> {
        Producer(self.0).into_unindexed()
    }

    fn count(self) -> usize {
        self.len()
    }
}

impl<'a, T> IndexedParallelIterator for ParIter<'a, T> {
    fn indexed_producer(&mut self) -> impl IndexedProducer<Item = Self::Item> {
        Producer(self.0)
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

struct Producer<'a, T>(&'a [T]);

impl<'a, T> IndexedProducer for Producer<'a, T> {
    type Item = &'a T;

    fn into_iter(self) -> impl Iterator<Item = Self::Item> {
        self.0.iter()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn split_off_left_at(&mut self, index: usize) -> Self {
        let (left, right) = self.0.split_at(index);
        self.0 = right;
        Self(left)
    }
}
