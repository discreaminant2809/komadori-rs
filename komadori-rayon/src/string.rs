//! Parallel collectors for [`String`].
//!
//! This module corresponds to [`std::string`].

mod concat;

use std::collections::LinkedList;

fn append_linked_string(s: &mut String, mut chunks: LinkedList<String>, mut append_len: usize) {
    let Some(mut first_chunk) = chunks.pop_front() else {
        return;
    };

    // The idea is that we optimize
    if s.is_empty() {
        // If the first chunk has more capacity, it's always reasonable,
        // and even more if the original has no allocation at all.
        //
        // If not, it's purely allocator RNG that we can't model cleanly.
        // Who knows, having a big enough buffer to accomodate `append_len`
        // additional items is better than reallocation in some cases?
        std::mem::swap(s, &mut first_chunk);

        // Shouldn't panic. The reduction of linked vec is assumed to
        // correctly report the amount to append.
        append_len -= s.len();
    }
    // If the length isn't 0,
    // we have no other choice but to obey the original
    // which already contains some items.

    s.reserve(append_len);
    *s += &first_chunk;
    s.extend(chunks);
}
