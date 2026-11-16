use std::ops::ControlFlow;

use crate::{
    collections::{Append, CollectionBase, linked},
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, OpaqueUnindexedConsumer, SerialOf,
            SerialOutputOf, UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
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
/// - [`Vec<T>`], with item types that implement [`Borrow<[T]>`][`Borrow`],
///   where [`T: Clone`](Clone).
///
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
    /// use komadori_rayon::prelude::*;
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
    /// use komadori_rayon::prelude::*;
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
    OS: CollectionBase + Append + Default + Send,
{
    type Serial = unique::Serial<'a, Self, internal::Serial<'a, OS>>;
}

impl<'a, 'os, OS> DefineSerial<'a> for ParConcatMut<'os, OS>
where
    OS: CollectionBase + Append + Default + Send,
{
    type Serial = unique::Serial<'a, Self, internal::Serial<'a, OS>>;
}

impl<'a, OS> DefineUnindexedSerial<'a> for IntoParConcat<OS>
where
    OS: CollectionBase + Append + Default + Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, internal::Serial<'a, OS>>;
}

impl<'a, 'os, OS> DefineUnindexedSerial<'a> for ParConcatMut<'os, OS>
where
    OS: CollectionBase + Append + Default + Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, internal::Serial<'a, OS>>;
}

impl<OS> ParallelCollectorBase for IntoParConcat<OS>
where
    OS: CollectionBase + Append + Default + Send,
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
        unique::uniquify(parts(&mut self.owned_slice))
    }
}

impl<'os, OS> ParallelCollectorBase for ParConcatMut<'os, OS>
where
    OS: CollectionBase + Append + Default + Send,
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
        unique::uniquify(parts(self.owned_slice))
    }
}

impl<OS> UnindexedParallelCollectorBase for IntoParConcat<OS>
where
    OS: CollectionBase + Append + Default + Send,
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
        unique_unindexed::uniquify(parts(&mut self.owned_slice))
    }
}

impl<'os, OS> UnindexedParallelCollectorBase for ParConcatMut<'os, OS>
where
    OS: CollectionBase + Append + Default + Send,
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
        unique_unindexed::uniquify(parts(self.owned_slice))
    }
}

#[expect(clippy::type_complexity)]
fn parts<OS>(
    owned_slice: &mut OS,
) -> (
    OpaqueUnindexedConsumer!(internal::Serial<'_, OS>),
    impl FnOnce(internal::Output<'_, OS>) -> ControlFlow<()>,
)
where
    OS: CollectionBase + Append + Default + Send,
{
    (
        linked::unindexed(internal::ConcatCollection::new(owned_slice)),
        |output| {
            output.finalize(
                |mut owned_slice, chunk| owned_slice.append(chunk),
                |owned_slice, chunks, len| {
                    owned_slice
                        .into_inner()
                        .append_many(chunks.into_iter().map(|chunk| chunk.into_inner()), len)
                },
            );
            ControlFlow::Continue(())
        },
    )
}

pub(crate) trait OwnedSlice<T>: CollectionBase {
    fn push(&mut self, item: T);
}

impl<OS, T> OwnedSlice<T> for &mut OS
where
    OS: OwnedSlice<T> + ?Sized,
{
    #[inline]
    fn push(&mut self, item: T) {
        OS::push(self, item);
    }
}

pub(crate) trait ParConcatSealed {}

#[expect(missing_debug_implementations)]
mod internal {
    use crate::collections::{Append, Collection, CollectionBase, linked};

    use super::OwnedSlice;

    pub type Serial<'a, OS> = linked::Serial<ConcatCollection<&'a mut OS>, ConcatCollection<OS>>;

    pub type Output<'a, OS> = linked::Output<ConcatCollection<&'a mut OS>, ConcatCollection<OS>>;

    #[derive(Default)]
    pub struct ConcatCollection<OS> {
        owned_slice: OS,
    }

    impl<OS> ConcatCollection<OS> {
        #[inline]
        pub(super) fn new(owned_slice: OS) -> Self {
            Self { owned_slice }
        }

        #[inline]
        pub(super) fn into_inner(self) -> OS {
            self.owned_slice
        }
    }

    impl<OS> CollectionBase for ConcatCollection<OS>
    where
        OS: CollectionBase,
    {
        #[inline]
        fn len(&self) -> usize {
            self.owned_slice.len()
        }

        #[inline]
        fn is_empty(&self) -> bool {
            self.owned_slice.is_empty()
        }

        #[inline]
        fn capacity(&self) -> usize {
            self.owned_slice.capacity()
        }

        // Unlike pushing an element at a time, each pushed slice
        // may contain no elements at all.
        // We choose NOT to reserve any amount upfront like
        // the standard library.
        #[inline]
        fn reserve(&mut self, _additional: usize) {}
    }

    impl<OS, OOS> Append<ConcatCollection<OOS>> for ConcatCollection<OS>
    where
        OS: Append<OOS>,
    {
        #[inline]
        fn append(&mut self, other: ConcatCollection<OOS>) {
            self.owned_slice.append(other.owned_slice);
        }
    }

    impl<OS, T> Collection<T> for ConcatCollection<OS>
    where
        OS: OwnedSlice<T>,
    {
        #[inline]
        fn push(&mut self, item: T) {
            self.owned_slice.push(item);
        }

        #[inline]
        fn push_many(&mut self, items: impl IntoIterator<Item = T>) {
            items.into_iter().for_each(|item| self.push(item));
        }
    }
}
