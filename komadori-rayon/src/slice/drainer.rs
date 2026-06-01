use std::{marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

use crate::test_utils::IndexedProducer;

pub struct Drainer<'a, T> {
    slice: &'a mut [MaybeUninit<T>],
}

impl<'a, T> Drainer<'a, T> {
    /// # SAFETY
    ///
    /// The slice must be fully initialized.
    pub unsafe fn new(slice: &'a mut [MaybeUninit<T>]) -> Self {
        Self { slice }
    }
}

impl<T> Drop for Drainer<'_, T> {
    fn drop(&mut self) {
        unsafe { self.slice.assume_init_drop() };
    }
}

impl<T> IndexedProducer for Drainer<'_, T> {
    type Item = T;

    fn into_iter(self) -> impl Iterator<Item = Self::Item> {
        let range = self.slice.as_mut_ptr_range();

        unsafe {
            IntoIter {
                start: NonNull::new_unchecked(range.start).cast(),
                end: NonNull::new_unchecked(range.end).cast(),
                _marker: PhantomData,
            }
        }
    }

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn split_off_left_at(&mut self, index: usize) -> Self {
        // Added `take()` cuz the borrow checker was complaining.
        let (left, right) = std::mem::take(&mut self.slice).split_at_mut(index);
        self.slice = right;
        Drainer { slice: left }
    }
}

struct IntoIter<'a, T> {
    start: NonNull<T>,
    end: NonNull<T>,
    _marker: PhantomData<&'a mut T>,
}

impl<T> Drop for IntoIter<'_, T> {
    fn drop(&mut self) {
        unsafe {
            drop(Drainer::<'_, T>::new(std::slice::from_raw_parts_mut(
                self.start.as_ptr() as _,
                self.len(),
            )));
        };
    }
}

impl<T> Iterator for IntoIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        (self.start < self.end).then(|| unsafe {
            let item = self.start.read();
            self.start = self.start.add(1);
            item
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<T> ExactSizeIterator for IntoIter<'_, T> {
    fn len(&self) -> usize {
        unsafe { self.end.offset_from_unsigned(self.start) }
    }
}
