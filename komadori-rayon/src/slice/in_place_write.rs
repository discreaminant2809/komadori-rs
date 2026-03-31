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
        "outputs were not fully reduced: expected (addr: {:?}, len: {}), got (addr: {:?}, len: {})",
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
    /// Must ensure that `start` is properly aligned, and the memory region
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

pub struct Combiner(());

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

impl<'a, T> plumbing::ConsumerBase for Consumer<'a, T>
where
    T: Send,
{
    type Combiner = Combiner;

    fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
        // Based on the trait's spec, the caller may provide an index exceeding
        // the reported len.
        let index = index.clamp(0, self.len);

        let left_start = self.start;
        unsafe {
            // SAFETY: the index was clamped to 0..=len.
            self.start = self.start.add(index);
        }
        self.len -= index;

        (
            Self {
                start: left_start,
                len: index,
                _marker: PhantomData,
            },
            Combiner(()),
        )
    }
}

impl<'a, T> plumbing::Combiner<WriteProof<'a, T>> for Combiner {
    fn combine(self, left: &mut WriteProof<'a, T>, right: WriteProof<'a, T>) {
        left.debug_assert_fully_written();
        right.debug_assert_fully_written();

        let expected_addr = unsafe { left.start.add(left.init_len) };
        if expected_addr != right.start {
            #[cfg(debug_assertions)]
            panic!(
                "failed to combine write proofs: left address = {:?}, expected right address {:?}, got right address {:?}",
                left.start, expected_addr, right.start,
            );

            // If we're not in debug assertion, drop everything written in `right`.
        } else {
            left.init_len += right.init_len;
            left.len += right.len;
            forget(right);
        }
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

    /// # Safety
    ///
    /// Must ensure that there's a space left to write to.
    /// In other words, init_len < len. Otherwise, may UB.
    unsafe fn collect_unchecked(&mut self, item: T) -> ControlFlow<()> {
        debug_assert!(self.break_hint().is_continue(), "no space left to write");

        unsafe {
            // SAFETY: We write at the index before the len.
            self.start.add(self.init_len).write(item);
        }

        self.init_len += 1;
        self.break_hint()
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

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        if self.init_len < self.len {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}
impl<'a, T> Collector<T> for WriteProof<'a, T> {
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.break_hint()?;
        unsafe { self.collect_unchecked(item) }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.break_hint()?;
        items
            .into_iter()
            .try_for_each(|item| unsafe { self.collect_unchecked(item) })
    }
}
