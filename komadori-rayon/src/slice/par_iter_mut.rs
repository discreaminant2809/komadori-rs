use crate::test_utils::{
    IndexedParallelIterator, IndexedProducer, IntoParallelIterator, ParallelIterator, Producer as IProducer,
};

pub struct ParIterMut<'a, T>(&'a mut [T]);

impl<'a, T: 'a> IntoParallelIterator for &'a mut [T] {
    type Item = &'a mut T;

    type IntoParIter = ParIterMut<'a, T>;

    fn into_par_iter(self) -> Self::IntoParIter {
        ParIterMut(self)
    }
}

impl<'a, T> ParallelIterator for ParIterMut<'a, T> {
    type Item = &'a mut T;

    fn take_producer(&mut self) -> impl IProducer<Item = Self::Item> {
        self.take_indexed_producer().into_unindexed()
    }

    fn count(self) -> usize {
        self.len()
    }
}

impl<'a, T> IndexedParallelIterator for ParIterMut<'a, T> {
    fn take_indexed_producer(&mut self) -> impl IndexedProducer<Item = Self::Item> {
        Producer(std::mem::take(&mut self.0))
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

struct Producer<'a, T>(&'a mut [T]);

impl<'a, T> IndexedProducer for Producer<'a, T> {
    type Item = &'a mut T;

    fn into_iter(self) -> impl Iterator<Item = Self::Item> {
        self.0.iter_mut()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn split_off_left_at(&mut self, index: usize) -> Self {
        let (left, right) = std::mem::take(&mut self.0).split_at_mut(index);
        self.0 = right;
        Self(left)
    }
}
