//! Parallel collectors for [`Vec`].
//!
//! This module corresponds to [`mod@std::vec`].

mod collection;
mod concat;
mod par_collector;

pub use par_collector::*;

/// # Safety
///
/// Must have reserved for at least one element via [`Vec::reserve()`] or similar methods.
pub(crate) unsafe fn push_unchecked<T>(v: &mut Vec<T>, item: T) {
    debug_assert!(
        v.len() < v.capacity(),
        "`assume_reserved_collect()` called without reservation"
    );

    let len = v.len();

    unsafe {
        v.as_mut_ptr()
            // SAFETY: the allocated object is `sizeof(T) * len` big.
            .add(len)
            // SAFETY: We've reserved for at least one element.
            .write(item);

        // SAFETY: We've reserved for at least one element,
        // and the element at index `len` is initialized.
        v.set_len(len + 1);
    }
}
