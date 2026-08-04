//! [![Crates.io Version](https://img.shields.io/crates/v/komadori.svg)](https://crates.io/crates/komadori)
//! [![Docs.rs](https://img.shields.io/docsrs/komadori)](https://docs.rs/komadori)
//! [![GitHub Repo](https://img.shields.io/badge/github-komadori--rs-blue?logo=github)](https://github.com/discreaminant2809/komadori-rs.git)
//! ![MSRV](https://img.shields.io/crates/msrv/komadori)
//!
//! Multi-reduction library. Provides a composable, declarative way to consume an iterator.
//!
//! If [`Iterator`] is the "source half" of data pipeline, [`Collector`] is the "sink half" of the pipeline.
//!
//! In order words, [`Iterator`] describes how to produce data, and [`Collector`] describes how to consume it.
//!
//! # Motivation
//!
//! Suppose we are given an array of `i32` and we are asked to calculate sum
//! and create a [`Vec`] of every num being doubled. What would be our approach?
//!
//! - Approach 1: Two-pass
//!
//! ```
//! let nums = [1, 3, 2];
//! let sum: i32 = nums.into_iter().sum();
//! let doubles = nums.into_iter()
//!     .map(|num| num * 2)
//!     .collect::<Vec<_>>();
//!
//! assert_eq!(sum, 6);
//! assert_eq!(doubles, [2, 6, 4]);
//! ```
//!
//! **Cons:** This performs two passes over the data, which may be worse than one-pass
//! due to increased memory traffic. It is fine for arrays,
//! but can be much worse for [`HashSet`], [`LinkedList`], or... data from an IO stream.
//!
//! - Approach 2: `for`-loop (or [`Iterator::fold()`] if you prefer)
//!
//! ```
//! let nums = [1, 3, 2];
//!
//! let mut sum = 0;
//! let mut doubles = Vec::with_capacity(nums.len());
//!
//! for num in nums {
//!     sum += num;
//!     doubles.push(num * 2);
//! }
//!
//! assert_eq!(sum, 6);
//! assert_eq!(doubles, [2, 6, 4]);
//! ```
//!
//! **Cons:** Not very declarative. Even with `fold()`,
//! the main logic is still kind of procedural because you have to ensure that
//! the logic is a one-pass.
//! Moreover, its performance is much worse due to `doubles.push(num * 2)`
//! being lowered into a scalar loop with repeaated capacity check
//! (even though we have reserved beforehand!),
//! inhibiting vectorization.
//!
//! - Approach 3: [`Iterator::inspect()`]
//!
//! ```
//! let nums = [1, 3, 2];
//! let mut sum = 0;
//! let doubles = nums
//!     .into_iter()
//!     .inspect(|num| sum += num)
//!     .map(|num| num * 2)
//!     .collect::<Vec<_>>();
//!
//! assert_eq!(sum, 6);
//! assert_eq!(doubles, [2, 6, 4]);
//! ```
//!
//! **Cons:** This approach has multiple drawbacks:
//!
//! - If the requirement changes to "calculate sum and find any negative value,"
//!   this approach may produce incorrect results.
//!   The "any" logic may short-circuit on finding the desired value,
//!   preventing the "sum" logic from summing every value.
//!   It is possible that we can rearrange so that the "any" logic goes first,
//!   but if the requirement changes to "find any negative value and even value,"
//!   we cannot escape.
//! - The state is kept outside. Now the iterator cannot go anywhere else
//!   (e.g. returning from a function).
//! - Very unintuitive and hack-y (hard to reason about).
//! - Not declarative enough.
//! - Slower than the upcoming approach.
//!   (Pro tip: Use `map()` instead of `inspect()` results in way better performance,
//!   but the above issues still exist)
//!
//! This crate proposes a one-pass, declarative approach:
//!
//! ```
//! use komadori::{prelude::*, cmp::Max};
//!
//! let nums = [1, 3, 2];
//! let (sum, doubles) = nums
//!     .into_iter()
//!     .feed_into((
//!         0.into_sum(),
//!         vec![].into_collector().map(|num| num * 2),
//!     ));
//!
//! assert_eq!(sum, 6);
//! assert_eq!(doubles, [2, 6, 4]);
//! ```
//!
//! This approach achieves:
//!
//! - One-pass.
//! - Declarative.
//! - Performance (LLVM lowers it into a very nice vectorized one-pass loop).
//!
//! See [this benchmark][sum-doubles-benchmark] to see more approaches.
//! You can run the benchmark by yourself!
//!
//! This is only with integers. How about with non-`Copy` types?
//!
//! ```
//! // Suppose we open a connection...
//! fn socket_stream() -> impl Iterator<Item = String> {
//!     ["the", "noble", "and", "the", "singer"]
//!         .into_iter()
//!         .map(String::from)
//! }
//!
//! // Task: Returns:
//! // - An array of data from the stream.
//! // - How many bytes were read.
//! // - The last-seen data.
//!
//! // Usually, we're pretty much stuck with for-loop
//! // (tradition, `(try_)fold`, `(try_)for_each`).
//! // No common existing tools can help us here:
//! let mut byte_read = 0_usize;
//! let mut received = vec![];
//! let mut last_seen = None;
//!
//! for data in socket_stream() {
//!     byte_read += data.len();
//!     received.push(data.clone());
//!     last_seen = Some(data);
//! }
//!
//! let expected = (byte_read, received, last_seen);
//!
//! // This crate's way:
//! use komadori::{prelude::*, iter::Last, clb_mut};
//!
//! let (byte_read, received, last_seen) = socket_stream()
//!     .feed_into((
//!         0_usize
//!             .into_sum()
//!             .map(clb_mut!(|s: &mut String| -> usize { s.len() })),
//!         vec![].into_collector().cloning(),
//!         Last::new(),
//!     ));
//!
//! assert_eq!((byte_read, received, last_seen), expected);
//! ```
//!
//! Very declarative! We describe what we want to collect.
//!
//! You might think this is just like [`Iterator::unzip()`]...
//!
//! Consider this example:
//!
//! ```
//! use std::collections::HashSet;
//! use komadori::{prelude::*, clb_mut};
//!
//! // Suppose we open a connection...
//! fn socket_stream() -> impl Iterator<Item = String> {
//!     ["the", "noble", "and", "the", "singer"]
//!         .into_iter()
//!         .map(String::from)
//! }
//!
//! // Task: Collect UNIQUE chunks of data and concatenate them.
//!
//! // `Iterator::unzip`
//! let unzip_way: (String, HashSet<_>) = socket_stream()
//!     // Sad. We have to clone.
//!     // We can't take a reference, since the referenced data is returned too.
//!     .map(|chunk| (chunk.clone(), chunk))
//!     .unzip();
//!
//! // Another approach is do two passes (collect to `Vec`, then iterate),
//! // which is still another allocation,
//! // or `Iterator::fold`, which's procedural.
//!
//! // `Collector`
//! let collector_way = socket_stream()
//!     // No clone. The data flows smoothly.
//!     .feed_into((
//!         String::new()
//!             .into_concat()
//!             .map(clb_mut!(|s: &mut String| -> &str { &s[..] })),
//!         HashSet::new(),
//!     ));
//!
//! assert_eq!(unzip_way, collector_way);
//! ```
//!
//! # Crate stucture
//!
//! Modules in this crate mirror those in the standard library, because this crate
//! extends many types there. There is also `collector` which
//! contains collector functionalities that work behind [`feed_into()`],
//! and `prelude` which re-exports commons items for easier use.
//!
//! It is recommended to read the documentation of `collector` next
//! if you want to delve into how collectors work.
//!
//! # Features
//!
//! - **`alloc`** — Enables collectors and implementations for types in the
//!   [`alloc`] crate (e.g., [`Vec`], [`VecDeque`], [`BTreeSet`]).
//!
//! - **`std`** *(default)* — Enables the `alloc` feature and implementations
//!   for [`std`]-only types (e.g., [`HashMap`]).
//!   When this feature is disabled, the crate builds in `no_std` mode.
//!
//! - **`itertools`** — Enables collectors and adapters that resemble those
//!   in the `itertools` crate.
//!
//! - **`unstable`** — Enables experimental and unstable features.
//!   Items gated behind this feature do **not** follow normal semver guarantees
//!   and may change or be removed at any time.
//!
//!   Although the crate as a whole is technically still experimental, the items under
//!   `unstable` are even more experimental, and it is generally
//!   discouraged to use them until their designs are finalized and not
//!   under this flag anymore.
//!
//! [`Collector`]: crate::collector::Collector
//! [`feed_into()`]: crate::iter::IteratorExt::feed_into
//! [`Vec`]: alloc::vec::Vec
//! [`HashSet`]: std::collections::HashSet
//! [`HashMap`]: std::collections::HashMap
//! [`LinkedList`]: alloc::collections::LinkedList
//! [`VecDeque`]: alloc::collections::VecDeque
//! [`BTreeSet`]: alloc::collections::BTreeSet
//! [sum-doubles-benchmark]: https://github.com/discreaminant2809/komadori-rs/blob/main/komadori/benches/sum_doubles.rs

#![forbid(
    missing_docs,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    clippy::alloc_instead_of_core
)]
#![deny(missing_debug_implementations)]
#![cfg_attr(test, deny(deprecated))]
#![cfg_attr(docsrs, feature(doc_cfg))]
// To make doc examples in sync (prevent accidental deprecated items usage in doc).
#![doc(test(attr(deny(deprecated))))]
#![no_std]

// We need doc to be able to refer to `alloc` crate also.
#[cfg(any(doc, feature = "alloc"))]
extern crate alloc;

// We need doc to be able to refer to `std` crate also.
#[cfg(any(doc, feature = "std"))]
extern crate std;

pub mod cmp;
#[cfg(feature = "alloc")]
pub mod collections;
pub mod collector;
pub mod either;
pub mod iter;
pub mod mem;
pub mod num;
pub mod ops;
pub mod prelude;
pub mod slice;
#[cfg(feature = "alloc")]
pub mod string;
pub mod sync;
pub mod tuple;
pub mod unit;
#[cfg(feature = "alloc")]
pub mod vec;

#[cfg(feature = "alloc")]
mod boxed;
mod reference;
#[cfg(all(test, feature = "std"))]
mod test_utils;

/// Introduces the [`#!\[feature = closure_lifetime_binder\]`] to help dealing with
/// poor lifetime inference issues of the compiler while using collectors.
///
/// This macro creates an [`FnOnce`] closure.
///
/// To use generics and lifetimes outside of the closure, put them in the `use`
/// item first.
///
/// # Examples
///
/// ```
/// use komadori::clb_once;
///
/// # fn foo<'b, T: 'b>() {
/// clb_once!(use<'b, T> for<'a> |x: &'a i32, _y: &'b T| -> &'a i32 { x });
/// # }
/// ```
///
/// [`#!\[feature = closure_lifetime_binder\]`]: <https://rust-lang.github.io/rfcs/3216-closure-lifetime-binder.html>
#[macro_export]
macro_rules! clb_once {
    (
        use<$($use_lts:lifetime,)* $($use_tys:ident),*>
        for<$($lts:lifetime),* $(,)?>
        $($move_kw:ident)?
        |$($params:ident: $param_tys:ty),*| -> $ret_ty:ty $block:block
    ) => {
        ({
            fn __closure__<$($use_lts,)* __F__, $($use_tys),*>(f: __F__) -> __F__
            where
                __F__: for<$($lts),*> ::core::ops::FnOnce($($param_tys),*) -> $ret_ty,
            {
                f
            }

            __closure__::<$($use_lts,)* _, $($use_tys),*>
        })($($move_kw)? |$($params),*| $block)
    };

    (
        for<$($lts:lifetime),* $(,)?>
        $($move_kw:ident)?
        |$($params:ident: $param_tys:ty),*| -> $ret_ty:ty $block:block
    ) => {
        $crate::clb_once!(
            use<>
            for<$($lts),*>
            $($move_kw)?
            |$($params: $param_tys),*| -> $ret_ty $block
        )
    };

    (
        $($move_kw:ident)?
        |$($params:ident: $param_tys:ty),*| -> $ret_ty:ty $block:block
    ) => {
        $crate::clb_once!(
            for<>
            $($move_kw)?
            |$($params: $param_tys),*| -> $ret_ty $block
        )
    };
}

/// Introduces the [`#!\[feature = closure_lifetime_binder\]`] to help dealing with
/// poor lifetime inference issues of the compiler while using collectors.
///
/// This macro creates an [`FnMut`] closure.
///
/// To use generics and lifetimes outside of the closure, put them in the `use`
/// item first.
///
/// # Examples
///
/// ```
/// use komadori::clb_mut;
///
/// # fn foo<'b, T: 'b>() {
/// clb_mut!(use<'b, T> for<'a> |x: &'a i32, _y: &'b T| -> &'a i32 { x });
/// # }
/// ```
///
/// [`#!\[feature = closure_lifetime_binder\]`]: <https://rust-lang.github.io/rfcs/3216-closure-lifetime-binder.html>
#[macro_export]
macro_rules! clb_mut {
    (
        use<$($use_lts:lifetime,)* $($use_tys:ident),*>
        for<$($lts:lifetime),* $(,)?>
        $($move_kw:ident)?
        |$($params:ident: $param_tys:ty),*| -> $ret_ty:ty $block:block
    ) => {
        ({
            fn __closure__<$($use_lts,)* __F__, $($use_tys),*>(f: __F__) -> __F__
            where
                __F__: for<$($lts),*> ::core::ops::FnMut($($param_tys),*) -> $ret_ty,
            {
                f
            }

            __closure__::<$($use_lts,)* _, $($use_tys),*>
        })($($move_kw)? |$($params),*| $block)
    };

    (
        for<$($lts:lifetime),* $(,)?>
        $($move_kw:ident)?
        |$($params:ident: $param_tys:ty),*| -> $ret_ty:ty $block:block
    ) => {
        $crate::clb_mut!(
            use<>
            for<$($lts),*>
            $($move_kw)?
            |$($params: $param_tys),*| -> $ret_ty $block
        )
    };

    (
        $($move_kw:ident)?
        |$($params:ident: $param_tys:ty),*| -> $ret_ty:ty $block:block
    ) => {
        $crate::clb_mut!(
            for<>
            $($move_kw)?
            |$($params: $param_tys),*| -> $ret_ty $block
        )
    };
}

/// Introduces the [`#!\[feature = closure_lifetime_binder\]`] to help dealing with
/// poor lifetime inference issues of the compiler while using collectors.
///
/// This macro creates an [`Fn`] closure.
///
/// To use generics and lifetimes outside of the closure, put them in the `use`
/// item first.
///
/// # Examples
///
/// ```
/// use komadori::clb;
///
/// # fn foo<'b, T: 'b>() {
/// clb!(use<'b, T> for<'a> |x: &'a i32, _y: &'b T| -> &'a i32 { x });
/// # }
/// ```
///
/// [`#!\[feature = closure_lifetime_binder\]`]: <https://rust-lang.github.io/rfcs/3216-closure-lifetime-binder.html>
#[macro_export]
macro_rules! clb {
    (
        use<$($use_lts:lifetime,)* $($use_tys:ident),*>
        for<$($lts:lifetime),* $(,)?>
        $($move_kw:ident)?
        |$($params:ident: $param_tys:ty),*| -> $ret_ty:ty $block:block
    ) => {
        ({
            fn __closure__<$($use_lts,)* __F__, $($use_tys),*>(f: __F__) -> __F__
            where
                __F__: for<$($lts),*> ::core::ops::Fn($($param_tys),*) -> $ret_ty,
            {
                f
            }

            __closure__::<$($use_lts,)* _, $($use_tys),*>
        })($($move_kw)? |$($params),*| $block)
    };

    (
        for<$($lts:lifetime),* $(,)?>
        $($move_kw:ident)?
        |$($params:ident: $param_tys:ty),*| -> $ret_ty:ty $block:block
    ) => {
        $crate::clb!(
            use<>
            for<$($lts),*>
            $($move_kw)?
            |$($params: $param_tys),*| -> $ret_ty $block
        )
    };

    (
        $($move_kw:ident)?
        |$($params:ident: $param_tys:ty),*| -> $ret_ty:ty $block:block
    ) => {
        $crate::clb!(
            for<>
            $($move_kw)?
            |$($params: $param_tys),*| -> $ret_ty $block
        )
    };
}

#[cfg(feature = "unstable")]
#[inline(always)]
const fn assert_iterator<I: Iterator>(iterator: I) -> I {
    iterator
}

#[allow(
    clippy::extra_unused_type_parameters,
    clippy::extra_unused_lifetimes,
    clippy::clone_on_copy,
    clippy::drop_non_drop
)]
fn _test_clb<'b, T: 'b>() {
    macro_rules! test_clb {
        ($macro:ident -> $fn_trait:ident) => {{
            fn assert_this_fn<F, T0, T1, R>(f: F) -> F
            where
                F: $fn_trait(T0, T1) -> R,
            {
                f
            }

            let _ = assert_this_fn($macro!(|x: i32, _y: &i32| -> i32 {
                x
            }))
            .clone();

            let _ = assert_this_fn($macro!(for<'a> move |x: &'a i32, _y: &i32| -> &'a i32 {
                x
            }))
            .clone();

            let _ = assert_this_fn($macro!(use<T> for<'a> |x: &'a i32, _y: T| -> &'a i32 { x }));
            let _ = assert_this_fn($macro!(use<'b, T> for<'a> |x: &'a i32, _y: &'b T| -> &'a i32 { x }));
        }};
    }

    test_clb!(clb_once -> FnOnce);
    test_clb!(clb_mut -> FnMut);
    test_clb!(clb -> Fn);
}
