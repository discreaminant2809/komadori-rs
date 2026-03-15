mod concat_mut;
mod into_concat;

pub use concat_mut::*;
pub use into_concat::*;

/// Converts a container into collectors that concatenate items.
///
/// This trait is currently sealed. It exists only to add methods
/// for types that can hold the concatenation result.
///
/// See its implementors for examples, and see [`ConcatItem`]
/// for supported item types.
///
/// This trait is sealed and for providing methods only.
/// If you want to support concatenation collectors for your type, create your own.
#[allow(private_bounds)]
pub trait Concat: Sized + ConcatSealed {
    /// Creates a collector that concatenates items.
    /// The [`Output`] type is the wrapped type.
    ///
    /// [`Output`]: crate::collector::CollectorBase::Output
    #[inline]
    fn into_concat(self) -> IntoConcat<Self> {
        IntoConcat::new(self)
    }

    /// Creates a collector that concatenates items into a mutable reference.
    /// The [`Output`] type is a mutable reference of the wrapped type.
    ///
    /// [`Output`]: crate::collector::CollectorBase::Output
    #[inline]
    fn concat_mut(&mut self) -> ConcatMut<'_, Self> {
        ConcatMut::new(self)
    }
}

/// Marks a type that can be used as the item type `T` for the [`Concat`]'s collectors.
///
/// This trait is currently sealed. It exists only to determine
/// which types can be concatenated into which types.
#[allow(private_bounds)]
pub trait ConcatItem<OwnedSlice>: Sized + ConcatItemSealed<OwnedSlice> {}

pub(crate) trait ConcatSealed {}

pub(crate) trait ConcatItemSealed<OwnedSlice>: Sized {
    fn push_to(&mut self, owned_slice: &mut OwnedSlice);

    #[inline]
    fn push_into(mut self, owned_slice: &mut OwnedSlice) {
        self.push_to(owned_slice);
    }

    fn bulk_push_into(items: impl IntoIterator<Item = Self>, owned_slice: &mut OwnedSlice) {
        items
            .into_iter()
            .for_each(move |item| item.push_into(owned_slice));
    }
}
