//! Parallel collectors for [`Vec`].
//!
//! This module corresponds to [`mod@std::vec`].

use std::{ops::ControlFlow, ptr::NonNull};

use komadori::prelude::*;

use crate::{
    collections::linked_vec,
    collector::{
        IntoParallelCollectorBase, ParallelCollectorBase, assert_unindexed_par_collector,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
    prelude::UnindexedParallelCollectorBase,
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
        assert_unindexed_par_collector::<_, T>(IntoParCollector(self))
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
        assert_unindexed_par_collector::<_, T>(ParCollectorMut(self))
    }
}

impl<'this, T> DefineSerial<'this> for IntoParCollector<T>
where
    T: Send,
{
    type Serial = unique::Serial<'this, Self, in_place_write::WriteProof<'this, T>>;
}

impl<'this, T> DefineUnindexedSerial<'this> for IntoParCollector<T>
where
    T: Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, linked_vec::Serial<T, usize>>;
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
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        self.0.reserve(len);

        // We must use `base` of the original vec (not `self`) for the committer
        // so that we don't alias with the region we're writing to.
        // Note: DO NOT rewrite it as `as_mut_ptr_range().end`,
        // since it's UB when being used later on MIRI.
        let being_written = unsafe {
            // SAFETY: the vec is `len()` long.
            self.0.as_mut_ptr().add(self.0.len())
        };
        let mut this = NonNull::from_mut(&mut self.0);

        unique::uniquify((
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
        ))
    }
}

impl<T> UnindexedParallelCollectorBase for IntoParCollector<T>
where
    T: Send,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((linked_vec::Consumer::new(), |(chunks, len)| {
            self.0.reserve(len);

            for mut chunk in chunks {
                self.0.append(&mut chunk);
            }

            ControlFlow::Continue(())
        }))
    }
}

impl<'this, 'v, T> DefineSerial<'this> for ParCollectorMut<'v, T>
where
    T: Send,
{
    type Serial = unique::Serial<'this, Self, in_place_write::WriteProof<'this, T>>;
}

impl<'v, 'this, T> DefineUnindexedSerial<'this> for ParCollectorMut<'v, T>
where
    T: Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, linked_vec::Serial<T, usize>>;
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
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
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
        // Note: DO NOT rewrite it as `as_mut_ptr_range().end`,
        // since it's UB when being used later on MIRI.
        let being_written = unsafe {
            // SAFETY: the vec is `len()` long.
            this.as_mut_ptr().add(this.len())
        };
        let mut this = NonNull::from_mut(this);

        unique::uniquify((
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
        ))
    }
}

impl<'v, T> UnindexedParallelCollectorBase for ParCollectorMut<'v, T>
where
    T: Send,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((linked_vec::Consumer::new(), |(chunks, len)| {
            self.0.reserve(len);

            for mut chunk in chunks {
                self.0.append(&mut chunk);
            }

            ControlFlow::Continue(())
        }))
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
mod miri_tests {
    use komadori::prelude::{Collector, IntoCollectorBase};

    use crate::prelude::*;

    #[test]
    fn no_alias_for_collector_mut() {
        let mut nums = vec![1, 2, 3];
        let mut collector = nums.par_collector_mut();

        let (_, consumer, commit) = collector.take_parts(3);
        let output = consumer.into_collector().collect_then_finish([4, 5, 6]);
        commit(output);

        collector.finish();
        assert_eq!(nums, [1, 2, 3, 4, 5, 6]);
    }
}
