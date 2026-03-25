//! Parallel collectors for [`Vec`].
//!
//! This module corresponds to [`mod@std::vec`].

use std::{ops::ControlFlow, ptr::NonNull};

use komadori::prelude::*;

use crate::{
    collector::{IntoParallelCollectorBase, ParallelCollectorBase, assert_par_collector, plumbing},
    slice::in_place_write,
};

/// A parallel collector that pushes collected items into a [`Vec`].
/// Its [`Output`] is [`Vec`].
///
/// This struct is created by `Vec::into_par_collector()`.
///
/// [`Output`]: ParallelCollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoParCollector<T>(Vec<T>);

/// A parallel collector that pushes collected items into a [`&mut Vec`](Vec).
/// Its [`Output`] is [`&mut Vec`](Vec).
///
/// This struct is created by `Vec::par_collector_mut()`.
///
/// [`Output`]: ParallelCollectorBase::Output
#[derive(Debug)]
pub struct ParCollectorMut<'a, T>(&'a mut Vec<T>);

impl<T> IntoParallelCollectorBase for Vec<T>
where
    T: Send,
{
    type Output = Self;

    type IntoParCollector = IntoParCollector<T>;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        assert_par_collector::<_, T>(IntoParCollector(self))
    }
}

impl<'a, T> IntoParallelCollectorBase for &'a mut Vec<T>
where
    T: Send,
{
    type Output = Self;

    type IntoParCollector = ParCollectorMut<'a, T>;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        assert_par_collector::<_, T>(ParCollectorMut(self))
    }
}

impl<'this, T> plumbing::DefineConsumer<'this> for IntoParCollector<T>
where
    T: Send,
{
    type Consumer = in_place_write::Consumer<'this, T>;
}

impl<T> ParallelCollectorBase for IntoParCollector<T>
where
    T: Send,
{
    type Output = Vec<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as plumbing::DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as plumbing::DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        self.0.reserve(len);

        // We must use `base` of the original vec (not `self`) for the committer
        // so that we don't alias with the region we're writing to.
        let being_written = self.0.as_mut_ptr_range().end;
        let mut this = NonNull::from_mut(&mut self.0);

        (
            len,
            unsafe { in_place_write::Consumer::new(being_written, len) },
            move |write_proof| {
                in_place_write::commit(write_proof, being_written, len);

                unsafe {
                    // SAFETY: `this` is `self`, which is a `Vec`.
                    // We've release the ownership of the written part,
                    // so we can reclaim the provenance over the `Vec` here.
                    let this = this.as_mut();

                    // SAFETY: we've fully written to the memory in
                    // `this.as_mut_ptr_range().end..this.as_mut_ptr_range().end + len`
                    this.set_len(this.len() + len);
                }

                ControlFlow::Continue(())
            },
        )
    }
}

impl<'this, 'v, T> plumbing::DefineConsumer<'this> for ParCollectorMut<'v, T>
where
    T: Send,
{
    type Consumer = in_place_write::Consumer<'this, T>;
}

impl<'v, T> ParallelCollectorBase for ParCollectorMut<'v, T>
where
    T: Send,
{
    type Output = &'v mut Vec<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as plumbing::DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as plumbing::DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        // NOTE: from outside, the lifetime of `parts` is still bounded to
        // the collector, but here, the mutable reference to the collector is
        // conceptually dropped after `let mut this = ...` (to avoid aliasing).
        // It ensures that the caller cannot obtain another instance of `parts`
        // without fully consuming the previous `parts`, while the implementation
        // remains flexible.
        let this = &mut *self.0;

        this.reserve(len);

        // We must use `base` of the original vec (not `self`) for the committer
        // so that we don't alias with the region we're writing to.
        let being_written = this.as_mut_ptr_range().end;
        let mut this = NonNull::from_mut(this);

        (
            len,
            unsafe { in_place_write::Consumer::new(being_written, len) },
            move |write_proof| {
                in_place_write::commit(write_proof, being_written, len);

                unsafe {
                    // SAFETY: `this` is `self`, which is a `Vec`.
                    // We've release the ownership of the written part,
                    // so we can reclaim the provenance over the `Vec` here.
                    let this = this.as_mut();

                    // SAFETY: we've fully written to the memory in
                    // `this.as_mut_ptr_range().end..this.as_mut_ptr_range().end + len`
                    this.set_len(this.len() + len);
                }

                ControlFlow::Continue(())
            },
        )
    }
}

impl<T> Default for IntoParCollector<T>
where
    T: Send,
{
    #[inline]
    fn default() -> Self {
        Vec::default().into_par_collector()
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn miri_no_alias_for_collector_mut() {
        let mut nums = vec![1, 2, 3];
        let mut collector = nums.par_collector_mut();
        let _ = std::hint::black_box(collector.parts(3));
    }
}
