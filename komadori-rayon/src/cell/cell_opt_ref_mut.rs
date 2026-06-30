use std::{cell::Cell, marker::PhantomData, ptr::NonNull};

/// [`Cell`] for `Option<&mut T>`.
pub struct CellOptRefMut<'a, T> {
    ptr: Cell<Option<NonNull<T>>>,
    // We're no different from a mutable reference,
    // (more precisely, an `Option` to a mutable reference)
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> From<Option<&'a mut T>> for CellOptRefMut<'a, T> {
    #[inline]
    fn from(reference: Option<&'a mut T>) -> Self {
        Self {
            ptr: Cell::new(reference.map(NonNull::from_mut)),
            _marker: PhantomData,
        }
    }
}

impl<T> Default for CellOptRefMut<'_, T> {
    #[inline]
    fn default() -> Self {
        Self::from(None)
    }
}

// SAFETY: We're basically a mutable reference.
unsafe impl<T> Send for CellOptRefMut<'_, T> where T: Send {}
// Maybe implementing Sync but this is not needed for now.

impl<'a, T> CellOptRefMut<'a, T> {
    /// Takes the mutable reference out of this cell if it still exists,
    /// leaving the cell effectively `None`.
    pub fn take(&self) -> Option<&'a mut T> {
        let mut ptr = self.ptr.take()?;
        Some(unsafe {
            // SAFETY:
            // - It's created from a mutable reference with lifetime 'a.
            // - This returned reference is unique and the pointer in this cell
            //   is null now.
            ptr.as_mut()
        })
    }

    /// Destroys this cell and returns the mutable reference if it still exists
    pub fn into_inner(self) -> Option<&'a mut T> {
        self.ptr.into_inner().map(|mut ptr| unsafe {
            // SAFETY:
            // - It's created from a mutable reference with lifetime 'a.
            // - This returned reference is unique because this cell is gonna be dropped.
            ptr.as_mut()
        })
    }
}
