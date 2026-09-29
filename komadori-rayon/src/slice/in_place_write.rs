#![allow(missing_debug_implementations)]

mod send_ptr;

use send_ptr::*;

use std::{marker::PhantomData, mem::forget, ops::ControlFlow};

use komadori::prelude::*;

use crate::collector::plumbing;

pub(crate) fn commit<'a, T>(proof: WriteProof<'a, T>, expected_addr: *mut T, expected_len: usize) {
    assert_eq!(
        (proof.start.get(), proof.init_len),
        (expected_addr, expected_len),
        "outputs were not fully combined: expected (addr: {:?}, len: {}), got (addr: {:?}, len: {})",
        expected_addr,
        expected_len,
        proof.start,
        proof.init_len,
    );

    // Release the ownership. Now the caller can use the memory again.
    forget(proof);
}

/*
Ideas:
- From a memory region, split into halves at a given index.
- When converted into a "write proof" (a collector), gradually write until it's "full."
- Multiple write proofs are guadually combined to the grand write proof. During the combination,
  it's checked whether both are fully written and the left's end must match the right's start.
- Finally, it's checked whether the number of writes matches the expectation.
 */

pub struct Consumer<'a, T> {
    start: SendPtr<T>,
    len: usize,
    _marker: PhantomData<&'a mut [T]>,
}

impl<T> Consumer<'_, T>
where
    T: Send,
{
    /// # Safety
    ///
    /// Must ensure that `start` is non-null and properly aligned, and the memory region
    /// from `start` to `start.add(len)` is not aliased and valid to write to.
    pub(crate) unsafe fn new(start: *mut T, len: usize) -> Self {
        Consumer {
            start: unsafe { SendPtr::new_unchecked(start) },
            len,
            _marker: PhantomData,
        }
    }
}

pub struct WriteProof<'a, T> {
    start: SendPtr<T>,
    len: usize,
    init_len: usize,
    // The lifetime must be invariant so that we can't just pick a proof
    // that has a smaller/greater lifetime.
    // Ultimately, it is both an output (from consumer) and input (for combiner),
    // so... invariant.
    #[allow(clippy::type_complexity)]
    _marker: PhantomData<fn(&'a mut [T]) -> &'a mut [T]>,
}

impl<'a, T> IntoCollectorBase for Consumer<'a, T> {
    type Output = WriteProof<'a, T>;

    type IntoCollector = WriteProof<'a, T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        WriteProof {
            start: self.start,
            len: self.len,
            init_len: 0,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> plumbing::Consumer for Consumer<'a, T>
where
    T: Send,
{
    #[inline]
    fn split_off_left_at(
        &mut self,
        index: usize,
    ) -> (
        Self,
        impl FnOnce(&mut Self::Output, Self::Output) + use<'a, T>,
    ) {
        assert!(
            (0..=self.len).contains(&index),
            "splitting out of bound: len is {}, but index is {index}",
            self.len,
        );

        let consumer = Self {
            start: self.start,
            len: index,
            _marker: PhantomData,
        };

        // SAFETY: the index was checked to be in 0..=len.
        self.start = unsafe { self.start.add(index) };
        self.len -= index;

        (consumer, WriteProof::append)
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.len.min(request)
    }
}

impl<'a, T> WriteProof<'a, T> {
    #[inline]
    fn debug_assert_fully_written(&self) {
        debug_assert_eq!(
            self.init_len, self.len,
            "have not fully written: address = {:?}, expected len {}, got len {}",
            self.start, self.len, self.init_len,
        );
    }

    #[inline]
    fn append(&mut self, other: Self) {
        self.debug_assert_fully_written();
        other.debug_assert_fully_written();

        let expected_addr = unsafe { self.start.add(self.init_len) };
        if expected_addr != other.start {
            #[cfg(debug_assertions)]
            panic!(
                "failed to combine write proofs: left address = {:?}, \
                expected right address {:?}, got right address {:?}",
                self.start, expected_addr, other.start,
            );

            // If we're not in debug assertion, drop everything written in `right`.
        } else {
            self.init_len += other.init_len;
            self.len += other.len;
            forget(other);
        }
    }

    #[inline]
    unsafe fn collect_unchecked(&mut self, item: T) {
        unsafe {
            // SAFETY: We write at the index before the len.
            self.start.add(self.init_len).write(item);
        }

        self.init_len += 1;
    }
}

impl<'a, T> Drop for WriteProof<'a, T> {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: we've ensured that we've wrote from `start` to `start + init_len`,
            // and `init_len <= len`.
            std::ptr::slice_from_raw_parts_mut(self.start.get(), self.init_len).drop_in_place();
        }
    }
}

impl<'a, T> CollectorBase for WriteProof<'a, T> {
    type Output = Self;

    #[inline]
    fn finish(self) -> Self::Output {
        self
    }

    plumbing::finish_boxed_impl! {}

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        request.min(self.len - self.init_len)
    }
}

impl<'a, T> Collector<T> for WriteProof<'a, T> {
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        assert!(self.init_len < self.len, "no space left to write");
        // SAFETY: We write at the index before the len.
        unsafe { self.collect_unchecked(item) };
        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        items.into_iter().for_each(|item| {
            assert!(self.init_len < self.len, "no space left to write");
            // SAFETY: We write at the index before the len.
            unsafe { self.collect_unchecked(item) };
        });
        ControlFlow::Continue(())
    }
}

impl<'a, 'i, T> Collector<&'i T> for WriteProof<'a, T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        self.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i T>) -> ControlFlow<()> {
        self.collect_many(items.into_iter().copied())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = &'i T>) -> Self::Output {
        self.collect_then_finish(items.into_iter().copied())
    }
}

impl<'a, 'i, T> Collector<&'i mut T> for WriteProof<'a, T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        self.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut T>) -> ControlFlow<()> {
        self.collect_many(items.into_iter().map(|&mut item| item))
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = &'i mut T>) -> Self::Output {
        self.collect_then_finish(items.into_iter().map(|&mut item| item))
    }
}
