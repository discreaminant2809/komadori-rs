use std::{collections::LinkedList, ops::ControlFlow};

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

/// Converts a container into parallel collectors that concatenate items.
///
/// This trait is currently sealed. It exists only to add methods
/// for types that can hold the concatenation result.
///
/// Currently, the supported types are:
///
/// - [`Vec<T>`], with item types that implement [`Borrow<[T]>`][`Borrow`].
/// - [`String`], with item types that implement [`Borrow<str>`][`Borrow`].
///
/// This trait is sealed and for providing methods only.
/// If you want to support concatenation collectors for your type, create your own.
///
/// [`Borrow`]: std::borrow::Borrow
#[expect(private_bounds)]
pub trait ParConcat: Sized + ParConcatSealed {
    /// Creates a parallel collector that concatenates items.
    /// The [`Output`] type is the wrapped type.
    ///
    /// [`Output`]: ParallelCollectorBase::Output
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::{prelude::*, slice::ParConcat};
    ///
    /// let concat = ["ab", "c", "def"]
    ///     .into_par_iter()
    ///     .feed_into(String::new().into_par_concat());
    ///
    /// assert_eq!(concat, "abcdef");
    /// ```
    #[inline]
    fn into_par_concat(self) -> IntoParConcat<Self> {
        IntoParConcat { owned_slice: self }
    }

    /// Creates a parallel collector that concatenates items into a mutable reference.
    /// The [`Output`] type is a mutable reference of the wrapped type.
    ///
    /// [`Output`]: ParallelCollectorBase::Output
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::{prelude::*, slice::ParConcat};
    ///
    /// let mut nums = vec![1, 2];
    ///
    /// [vec![3, 4], vec![], vec![5]]
    ///     .into_par_iter()
    ///     .feed_into(nums.par_concat_mut());
    ///
    /// assert_eq!(nums, [1, 2, 3, 4, 5]);
    /// ```
    #[inline]
    fn par_concat_mut(&mut self) -> ParConcatMut<'_, Self> {
        ParConcatMut { owned_slice: self }
    }
}

/// A parallel collector that concatenates items.
///
/// This `struct` is created by [`ParConcat::into_par_concat()`].
/// See its documentation for more.
#[derive(Debug, Clone, Default)]
pub struct IntoParConcat<OS> {
    owned_slice: OS,
}

/// A parallel collector that concatenates items.
///
/// This `struct` is created by [`ParConcat::par_concat_mut()`].
/// See its documentation for more.
#[derive(Debug)]
pub struct ParConcatMut<'os, OS> {
    owned_slice: &'os mut OS,
}

impl<'a, OS> DefineSerial<'a> for IntoParConcat<OS>
where
    OS: OwnedSliceBase + Send,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<'a, OS>>;
}

impl<'a, 'os, OS> DefineSerial<'a> for ParConcatMut<'os, OS>
where
    OS: OwnedSliceBase + Send,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<'a, OS>>;
}

impl<'a, OS> DefineUnindexedSerial<'a> for IntoParConcat<OS>
where
    OS: OwnedSliceBase + Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<'a, OS>>;
}

impl<'a, 'os, OS> DefineUnindexedSerial<'a> for ParConcatMut<'os, OS>
where
    OS: OwnedSliceBase + Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<'a, OS>>;
}

impl<OS> ParallelCollectorBase for IntoParConcat<OS>
where
    OS: OwnedSliceBase + Send,
{
    type Output = OS;

    #[inline]
    fn finish(self) -> Self::Output {
        self.owned_slice
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed(&mut self.owned_slice), |output| {
            output.finalize();
            ControlFlow::Continue(())
        }))
    }
}

impl<'os, OS> ParallelCollectorBase for ParConcatMut<'os, OS>
where
    OS: OwnedSliceBase + Send,
{
    type Output = &'os mut OS;

    #[inline]
    fn finish(self) -> Self::Output {
        self.owned_slice
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed(self.owned_slice), |output| {
            output.finalize();
            ControlFlow::Continue(())
        }))
    }
}

impl<OS> UnindexedParallelCollectorBase for IntoParConcat<OS>
where
    OS: OwnedSliceBase + Send,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((consumer::unindexed(&mut self.owned_slice), |output| {
            output.finalize();
            ControlFlow::Continue(())
        }))
    }
}

impl<'os, OS> UnindexedParallelCollectorBase for ParConcatMut<'os, OS>
where
    OS: OwnedSliceBase + Send,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((consumer::unindexed(self.owned_slice), |output| {
            output.finalize();
            ControlFlow::Continue(())
        }))
    }
}

pub(crate) trait OwnedSliceBase: Default {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn capacity(&self) -> usize;
    fn append(&mut self, other: Self);
    fn append_from_linked_list(&mut self, others: LinkedList<Self>, len: usize);
}

pub(crate) trait OwnedSlice<T>: OwnedSliceBase {
    fn push(&mut self, item: T);
}

pub(crate) trait ParConcatSealed {}

#[expect(missing_debug_implementations, private_bounds)]
mod consumer {
    use std::{cell::Cell, collections::LinkedList, ops::ControlFlow};

    use crate::collector::plumbing::{
        BasicUnindexedConsumer, Collector, CollectorBase, OpaqueUnindexedConsumer,
        finish_boxed_impl,
    };

    use super::{OwnedSlice, OwnedSliceBase};

    pub fn unindexed<OS>(owned_slice: &mut OS) -> OpaqueUnindexedConsumer!(Serial<'_, OS>)
    where
        OS: OwnedSliceBase + Send,
    {
        BasicUnindexedConsumer {
            state: Cell::new(Some(owned_slice)),
            split_f: |owned_slice| Cell::new(owned_slice.take()),
            combiner_f: |_| Output::extend,
            ma_f: |_, request| request,
            collector_f: |owned_slice| match owned_slice.into_inner() {
                Some(owned_slice) => Serial::LeftMost(owned_slice),
                None => Serial::Right(OS::default()),
            },
        }
    }

    pub enum Serial<'a, OS> {
        LeftMost(&'a mut OS),
        Right(OS),
    }

    pub struct Output<'a, OS> {
        owned_slice: Option<&'a mut OS>,
        chunks: Chunks<OS>,
    }

    enum Chunks<OS> {
        // This state exists so that we can avoid another indirection of `LinkedList`.
        // A win if we somehow only ever have 1 chunk (rare).
        Single(OS),
        Plural { chunks: LinkedList<OS>, len: usize },
    }

    impl<'a, OS> CollectorBase for Serial<'a, OS>
    where
        OS: OwnedSliceBase,
    {
        type Output = Output<'a, OS>;

        #[inline]
        fn finish(self) -> Self::Output {
            match self {
                Self::LeftMost(owned_slice) => Output {
                    owned_slice: Some(owned_slice),
                    chunks: Chunks::Single(OS::default()),
                },
                Self::Right(owned_slice) => Output {
                    owned_slice: None,
                    chunks: Chunks::Single(owned_slice),
                },
            }
        }

        finish_boxed_impl! {}
    }

    impl<'a, OS, T> Collector<T> for Serial<'a, OS>
    where
        OS: OwnedSlice<T>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            self.owned_slice_mut().push(item);
            ControlFlow::Continue(())
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            let owned_slice = self.owned_slice_mut();
            items
                .into_iter()
                .for_each(move |item| owned_slice.push(item));

            ControlFlow::Continue(())
        }
    }

    impl<OS> Chunks<OS>
    where
        OS: OwnedSliceBase,
    {
        fn extend(&mut self, right: Self) {
            match (&mut *self, right) {
                (Self::Single(left_chunk), Self::Single(right_chunk)) if left_chunk.is_empty() => {
                    // Use the one with more capacity to be more efficient.
                    // If the right chunk has some elements, we must use it,
                    // so that we don't spend time copying elements.
                    if !right_chunk.is_empty() || left_chunk.capacity() < right_chunk.capacity() {
                        *left_chunk = right_chunk;
                    }
                }
                (Self::Single(left_chunk), Self::Single(right_chunk)) if right_chunk.is_empty() => {
                    // We won't waste a `LinkedList` just to combine with an empty chunk.
                }
                (Self::Single(left_chunk), Self::Single(right_chunk)) => {
                    let left_chunk = std::mem::take(left_chunk);
                    *self = Self::Plural {
                        len: left_chunk.len() + right_chunk.len(),
                        chunks: LinkedList::from([left_chunk, right_chunk]),
                    };
                }
                (Self::Single(left_chunk), Self::Plural { mut chunks, len }) => {
                    let left_chunk = std::mem::take(left_chunk);
                    *self = Self::Plural {
                        len: left_chunk.len() + len,
                        chunks: {
                            chunks.push_front(left_chunk);
                            chunks
                        },
                    };
                }

                (Self::Plural { chunks, len }, Self::Single(chunk)) => {
                    if !chunk.is_empty() {
                        *len += chunk.len();
                        chunks.push_back(chunk);
                    }

                    // Otherwise that chunk is useless and not worth appending.
                }
                (
                    Self::Plural {
                        chunks: left_chunks,
                        len: left_len,
                    },
                    Self::Plural {
                        chunks: mut right_chunks,
                        len: right_len,
                    },
                ) => {
                    *left_len += right_len;
                    left_chunks.append(&mut right_chunks);
                }
            }
        }
    }

    impl<OS> Serial<'_, OS> {
        #[inline]
        fn owned_slice_mut(&mut self) -> &mut OS {
            match self {
                Self::LeftMost(owned_slice) => owned_slice,
                Self::Right(owned_slice) => owned_slice,
            }
        }
    }

    impl<OS> Output<'_, OS>
    where
        OS: OwnedSliceBase,
    {
        fn extend(&mut self, other: Self) {
            self.chunks.extend(other.chunks);
        }

        pub(crate) fn finalize(self) {
            let Some(owned_slice) = self.owned_slice else {
                // Which means that the parallel collector will be disposed soon
                // (possibly under `trying_options()` or similar),
                // or just being combined incorrectly.
                return;
            };

            match self.chunks {
                Chunks::Single(items) => owned_slice.append(items),
                Chunks::Plural { chunks, len } => owned_slice.append_from_linked_list(chunks, len),
            }
        }
    }
}
