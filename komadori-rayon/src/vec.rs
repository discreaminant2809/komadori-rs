//! Parallel collectors for [`Vec`].
//!
//! This module corresponds to [`mod@std::vec`].

#[cfg(test)]
use std::mem::MaybeUninit;
use std::{collections::LinkedList, ops::ControlFlow, ptr::NonNull};

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
#[cfg(test)]
use crate::{
    slice::drainer::Drainer,
    test_utils::{self, IndexedParallelIterator, IndexedProducer},
};

/// A parallel collector that pushes collected items into a [`Vec`].
/// Its [`Output`] is [`Vec`].
///
/// This can collect `T` where `T` is [`Send`],
/// and `&T` and `&mut T` where `T` is [`Send`] and [`Copy`].
///
/// This struct is created by `Vec::into_par_collector()`.
///
/// [`Output`]: ParallelCollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoParCollector<T>(Vec<T>);

/// A parallel collector that pushes collected items into a [`&mut Vec`](Vec).
/// Its [`Output`] is [`&mut Vec`](Vec).
///
/// This can collect `T` where `T` is [`Send`],
/// and `&T` and `&mut T` where `T` is [`Send`] and [`Copy`].
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
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, linked_vec::Serial<T>>;
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
            unindexed_append_to_original(&mut self.0, chunks, len);
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
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, linked_vec::Serial<T>>;
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
            unindexed_append_to_original(self.0, chunks, len);
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

fn unindexed_append_to_original<T>(og: &mut Vec<T>, mut chunks: LinkedList<Vec<T>>, mut append_len: usize) {
    let Some(mut first_chunk) = chunks.pop_front() else {
        return;
    };

    // The idea is that we optimize
    if og.is_empty() {
        // If the first chunk has more capacity, it's always reasonable,
        // and even more if the original has no allocation at all.
        //
        // If not, it's purely allocator RNG that we can't model cleanly.
        // Who knows, having a big enough buffer to accomodate `append_len`
        // additional items is better than reallocation in some cases?
        std::mem::swap(og, &mut first_chunk);

        // Can't panic. The reduction of linked vec is assumed to
        // correctly report the amount to append.
        append_len -= og.len();
    }
    // If the length isn't 0,
    // we have no other choice but to obey the original
    // which already contains some items.

    og.reserve(append_len);
    og.append(&mut first_chunk);
    for mut chunk in chunks {
        og.append(&mut chunk);
    }
}

#[cfg(test)]
impl<T> test_utils::IntoParallelIterator for Vec<T> {
    type Item = T;

    type IntoParIter = test_types::IntoParIter<T>;

    fn into_par_iter(self) -> Self::IntoParIter {
        test_types::IntoParIter(self)
    }
}

#[cfg(test)]
mod test_types {
    // Do this because
    // `type IntoParIter = test_types::IntoParIter<T>;`
    // comaplains "leaking crate-private type."
    pub struct IntoParIter<T>(pub(super) Vec<T>);
}

#[cfg(test)]
impl<T> test_utils::ParallelIterator for test_types::IntoParIter<T> {
    type Item = T;

    fn take_producer(&mut self) -> impl test_utils::Producer<Item = Self::Item> {
        self.indexed_producer().into_unindexed()
    }

    fn count(self) -> usize {
        self.len()
    }
}

#[cfg(test)]
impl<T> test_utils::IndexedParallelIterator for test_types::IntoParIter<T> {
    fn indexed_producer(&mut self) -> impl IndexedProducer<Item = Self::Item> {
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

#[cfg(test)]
mod proptests {
    use crate::test_utils::prelude::*;

    proptest! {
        #[test]
        fn indexed(
            starting_nums in propvec(any::<i32>(), ..5),
            (split_decision, nums) in propvec(any::<i32>(), ..5)
                .prop_flat_map(|nums| {
                    (IndexedSplitStrategy::new(nums.len(), DEFAULT_MAX_DEPTH), Just(nums))
                }),
            pool in CoroutinePool::prop(),
        ) {
            indexed_impl(pool, split_decision, starting_nums, nums)?;
        }
    }

    proptest! {
        #[test]
        fn unindexed(
            starting_nums in propvec(any::<i32>(), ..5),
            nums in propvec(any::<i32>(), ..5),
            split_decision in UnindexedSplitStrategy::new(DEFAULT_MAX_DEPTH),
            pool in CoroutinePool::prop(),
        ) {
            unindexed_impl(pool, split_decision, starting_nums, nums)?;
        }
    }

    fn indexed_impl(
        mut pool: CoroutinePool,
        split_decision: IndexedSplitDecision,
        starting_nums: Vec<i32>,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || starting_nums.clone().into_par_collector(),
            should_break_pred: |_| false,
            pred: |_, output| {
                PredError::assert_eq(
                    output,
                    starting_nums
                        .iter()
                        .copied()
                        .chain(nums.iter().copied())
                        .collect(),
                )
            },
        }
        .test_par_collector(&mut pool, &split_decision)
    }

    fn unindexed_impl(
        mut pool: CoroutinePool,
        split_decision: UnindexedSplitDecision,
        starting_nums: Vec<i32>,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || starting_nums.clone().into_par_collector(),
            should_break_pred: |_| false,
            pred: |_, output| {
                PredError::assert_eq(
                    output,
                    starting_nums
                        .iter()
                        .copied()
                        .chain(nums.iter().copied())
                        .collect(),
                )
            },
        }
        .test_unindexed_par_collector(&mut pool, &split_decision)
    }
}
