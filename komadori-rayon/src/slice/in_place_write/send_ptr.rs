use std::{fmt::Debug, ptr::NonNull};

pub struct SendPtr<T>(NonNull<T>);

/// SAFETY: We uphold the fact that this acts as a `&mut T`.
unsafe impl<T: Send> Send for SendPtr<T> {}

impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SendPtr<T> {}

impl<T> SendPtr<T> {
    #[inline]
    pub unsafe fn new_unchecked(ptr: *mut T) -> Self {
        unsafe { Self(NonNull::new_unchecked(ptr)) }
    }

    #[inline]
    pub fn get(self) -> *mut T {
        self.0.as_ptr()
    }

    #[inline]
    pub unsafe fn add(self, n: usize) -> Self {
        unsafe { Self(NonNull::new_unchecked(self.get().add(n))) }
    }

    #[inline]
    pub unsafe fn write(self, value: T) {
        unsafe {
            self.get().write(value);
        }
    }
}

impl<T> PartialEq for SendPtr<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for SendPtr<T> {}

impl<T> Debug for SendPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}
