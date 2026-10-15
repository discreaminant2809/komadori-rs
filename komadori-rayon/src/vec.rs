//! Parallel collectors for [`Vec`].
//!
//! This module corresponds to [`mod@std::vec`].

mod concat;
mod par_collector;

pub use par_collector::*;

use std::collections::LinkedList;

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

fn append_linked_vec<T>(v: &mut Vec<T>, mut chunks: LinkedList<Vec<T>>, mut append_len: usize) {
    let Some(mut first_chunk) = chunks.pop_front() else {
        return;
    };

    // The idea is that we optimize
    if v.is_empty() {
        // If the first chunk has more capacity, it's always reasonable,
        // and even more if the original has no allocation at all.
        //
        // If not, it's purely allocator RNG that we can't model cleanly.
        // Who knows, having a big enough buffer to accomodate `append_len`
        // additional items is better than reallocation in some cases?
        std::mem::swap(v, &mut first_chunk);

        // Shouldn't panic. The reduction of linked vec is assumed to
        // correctly report the amount to append.
        append_len -= v.len();
    }
    // If the length isn't 0,
    // we have no other choice but to obey the original
    // which already contains some items.

    v.reserve(append_len);
    v.append(&mut first_chunk);
    for mut chunk in chunks {
        v.append(&mut chunk);
    }
}
