//! Module contains traits and `struct`s for collectors.
//!
//! Collectors let you express reduction operations in a declarative
//! and composable way. If you want to "reduce" a collection
//! or a stream of items into another collection or computation,
//! you will likely reach for collectors.
//!
//! To use a collector with an [`Iterator`], call
//! [`feed_into()`](crate::iter::IteratorExt::feed_into) on the iterator.
//! The collector will drive the iteration and produce the final result.
//!
//! All traits in this module are re-exported in [`prelude`](crate::prelude).
//! You rarely need to import individual traits from here directly.
//!
//! [`CollectorBase`] and [`Collector`] defines a collector.
//! See their documentation for more.
//!
//! # Forms of collector
//!
//! Similar to how you obtain an [`Iterator`],
//! if you want to create a collector from a value,
//! use one of the following general-purpose methods:
//!
//! - [`into_collector()`](IntoCollectorBase::into_collector):
//!   Creates a collector from an own value.
//! - [`collector_mut()`](CollectorByMut::collector_mut):
//!   Creates a collector from a mutable reference.
//! - [`collector()`](CollectorByRef::collector) (rare):
//!   Creates a collector from a shared reference.
//!
//! In many cases, you can pass the value or its reference directly, as long as
//! the function signature expects an [`IntoCollectorBase`] or [`IntoCollector`].
//!
//! There are also specialized methods outside of this module for specific
//! use cases, such as [`into_concat()`](crate::slice::Concat::into_concat)
//! and [`adding()`](crate::ops::Adding::adding).
//!
//! # Adapters
//!
//! Like [`Iterator`], you can use [`tee()`](CollectorBase::tee),
//! [`map()`](CollectorBase::map), [`take()`](CollectorBase::take) and more
//! to enhance the capabilities of a collector. They are adapters, which
//! take a collector and produce another collector.
//! Adapters make collectors composable and allow you to express
//! complex reduction patterns.
//!
//! All adapters are defined on [`CollectorBase`] and nowhere else.
//!
//! There are different adapters to "tee" items into multiple collectors
//! (let each item be collected by multiple collectors).
//! Conceptually, they differ in how the item is passed
//! from one collector to another, either by cloning or by reference:
//!
//! - [`tee()`](CollectorBase::tee): a [`Copy`] of the item is passed.
//! - [`tee_clone()`](CollectorBase::tee_clone): the item is [`Clone`]d before being passed.
//! - [`tee_funnel()`](CollectorBase::tee_funnel): the item is passed by mutable reference,
//!   while the original collector takes ownership.
//! - [`tee_mut()`](CollectorBase::tee_mut): the item is passed by mutable reference,
//!   while the original collector also takes a mutable reference.
//!
//! Due to Rust's emphasis on ownership and borrowing, multiple "tee" adapters
//! are provided so that you can explicitly choose between cloning and borrowing
//! based on your needs. Usually, you should avoid cloning whenever possible,
//! and the method name `tee_clone` expresses your intent of cloning clearly.
//! It is recommended to check each adapter's documentation
//! for detailed semantics and examples.
//!
//! # Implementing a collector
//!
//! If the provided adapters are not enough for your use case,
//! or you want to create your own collector,
//! here is the minimal types and methods methods you must override:
//!
//! - [`CollectorBase::Output`]: what is the output type?
//! - [`CollectorBase::finish()`]: after finishing collecting, how to produce the output?
//! - `T` in [`Collector<T>`]: what item types does the collector accept?
//!   (A collector can accept more than one item type!)
//! - [`Collector::collect()`]: how to collect those items?
//!   (Do not forget when to stop accumulating too!)
//! - (Optional) Default methods for optimization. (You can see them
//!   in the documentations of [`CollectorBase`] and [`Collector`])
//!
//! See the [Examples](#examples) section for example implementations.
//!
//! # Unspecified behaviors
//!
//! Unless stated otherwise by the collector’s implementation, after any of
//! [`Collector::collect()`], [`Collector::collect_many()`], or
//! [`CollectorBase::break_hint()`] have returned [`Break(())`] once,
//! behaviors of subsequent calls to any method other than
//! [`finish()`](CollectorBase::finish) are unspecified.
//! They may panic, overflow, or even resume accumulation
//! (similar to how [`Iterator::next()`] might yield again after returning [`None`]).
//! Callers should generally call [`finish()`](CollectorBase::finish) once a collector
//! has signaled a stop.
//! If this invariant cannot be upheld, wrap it with [`fuse()`](CollectorBase::fuse).
//! Furthermore, a collector is in an unspecified state if panicked.
//!
//! This looseness allows for optimizations (for example, omitting an internal "stopped” flag).
//!
//! Although the behavior is unspecified, none of the aforementioned methods are `unsafe`.
//! Implementors must **not** cause memory corruption, undefined behavior,
//! or any other safety violations, and callers must **not** rely on such outcomes.
//!
//! # Limitations and workarounds
//!
//! In some cases, you may need to explicitly annotate the parameter types in closures,
//! especially for adapters that accept generic functions.
//! This is due to current limitations in Rust’s type inference for closure parameters.
//!
//! If you ever encounter compile errors such as
//! "`collect()` exists on ____ but the trait bound is not satisfied"
//! or "`FnMut` is not general enough," it is likely because the compiler
//! cannot promote *lifetimes* to be higher-ranked when applicable.
//! There are a few workarounds (for now):
//!
//! - If the return type of the closure has any lifetimes tied to
//!   types in the parameters, you can use [`clb_once!`](crate::clb_once),
//!   [`clb_mut!`](crate::clb_mut) and [`clb!`](crate::clb) to explicitly
//!   introduce higher-ranked lifetimes using `for<>` syntax,
//!   similar to how you would write `Fn` trait bounds.
//!   Most of the time, you are looking for [`clb_mut!`](crate::clb_mut).
//!   Usage examples can be found throughout documentation in this crate, or
//!   [here](https://rust-lang.github.io/rfcs/3216-closure-lifetime-binder.html).
//!
//! - If the return type of the closure does not involve such lifetimes,
//!   you can use those macros too, plus this hack (cons: clippy will warn about it):
//!
//!   ```ignore
//!   .map({
//!       let f = /* your closure here */;
//!       f
//!   })
//!   ```
//!
//! # Examples
//!
//! Suppose we are building a tokenizer to process text for an NLP model.
//! We will skip all complicated details for now and simply collect every word we see.
//!
//! ```
//! use std::{ops::ControlFlow, collections::HashMap};
//! use komadori::prelude::*;
//!
//! #[derive(Default)]
//! struct Tokenizer {
//!     indices: HashMap<String, usize>,
//!     words: Vec<String>,
//! }
//!
//! impl Tokenizer {
//!     fn tokenize(&self, sentence: &str) -> Vec<usize> {
//!         sentence
//!             .split_whitespace()
//!             .map(|word| self.indices.get(word).copied().unwrap_or(0))
//!             .collect()
//!     }
//! }
//!
//! // We have to implement this trait first.
//! impl CollectorBase for Tokenizer {
//!     // For now, for simplicity, we just return the struct itself.
//!     type Output = Self;
//!
//!     fn finish(self) -> Self::Output {
//!         // Just return itself.
//!         self
//!     }
//! }
//!
//! impl Collector<String> for Tokenizer {
//!     fn collect(&mut self, word: String) -> ControlFlow<()> {
//!         self.indices
//!             .entry(word)
//!             .or_insert_with_key(|word| {
//!                 self.words.push(word.clone());
//!                 // Reserve index 0 for out-of-vocabulary words.
//!                 self.words.len()
//!             });
//!
//!         // Tokenizer never stops accumulating.
//!         ControlFlow::Continue(())
//!     }
//! }
//!
//! let sentence = "the noble and the singer";
//! let tokenizer = sentence
//!     .split_whitespace()
//!     .map(String::from)
//!     .feed_into(Tokenizer::default());
//!
//! // "the" should only appear once.
//! assert_eq!(tokenizer.words, ["the", "noble", "and", "singer"]);
//! assert_eq!(tokenizer.tokenize("the singer and the swordswoman"), [1, 4, 3, 1, 0]);
//! ```
//!
//! [`Break(())`]: std::ops::ControlFlow::Break

mod adapters;
#[allow(clippy::module_inception)]
mod collector;
mod collector_base;
mod collector_by_mut;
mod collector_by_ref;
mod into_collector;

pub use adapters::*;
pub use collector::*;
pub use collector_base::*;
pub use collector_by_mut::*;
pub use collector_by_ref::*;
pub use into_collector::*;

#[inline(always)]
pub(crate) const fn assert_collector_base<C>(collector: C) -> C
where
    C: CollectorBase,
{
    collector
}

#[inline(always)]
pub(crate) const fn assert_collector<C, T>(collector: C) -> C
where
    C: Collector<T>,
{
    collector
}
