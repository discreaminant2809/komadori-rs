use std::mem::MaybeUninit;

use crate::{
    slice::drainer::Drainer,
    test_utils::{
        IndexedParallelIterator, IndexedProducer, IntoParallelIterator, ParallelIterator, Producer,
    },
};

impl<T> IntoParallelIterator for Vec<T> {
    type Item = T;

    type IntoParIter = test_types::IntoParIter<T>;

    fn into_par_iter(self) -> Self::IntoParIter {
        test_types::IntoParIter(self)
    }
}

mod test_types {
    // Do this because
    // `type IntoParIter = test_types::IntoParIter<T>;`
    // comaplains "leaking crate-private type."
    pub struct IntoParIter<T>(pub(super) Vec<T>);
}

impl<T> ParallelIterator for test_types::IntoParIter<T> {
    type Item = T;

    fn take_producer(&mut self) -> impl Producer<Item = Self::Item> {
        self.take_indexed_producer().into_unindexed()
    }

    fn count(self) -> usize {
        self.len()
    }
}

impl<T> IndexedParallelIterator for test_types::IntoParIter<T> {
    fn take_indexed_producer(&mut self) -> impl IndexedProducer<Item = Self::Item> {
        unsafe {
            let len = self.0.len();
            self.0.set_len(0);
            let slice = std::ptr::slice_from_raw_parts_mut(self.0.as_mut_ptr(), len);
            let slice = slice as *mut [MaybeUninit<T>];
            Drainer::new(slice.as_mut().unwrap())
        }
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}
