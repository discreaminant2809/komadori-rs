//! Low-level details of a parallel collector.
//!
//! # Overview
//!
//! The idea behind parallel collectors is:
//!
//! - First, shared states in a parallel collector are parked
//!   in the thread that orchestrates the operation.
//!
//! - Next, the parallel collector creates two "parts":
//!   - a consumer whose lifetime is bound into the collector.
//!   - a "committer" whose lifetime is bound into the collector.
//!     Its job is to "commit" the output of the consumer back
//!     to the parallel collector.
//!
//! - The consumer is used (converted to a serial collector, or split further)
//!   and eventually produces an output (produced directly, or combined from
//!   two outputs).
//!
//! - The committer commits the consumer's output back to the parallel collector.
//!
//! - A cycle completes! The parallel collector can be used again, or
//!   [`finish()`](super::ParallelCollectorBase) to produce the "grand final" output.
//!
//! Understanding this is crucial to understand the design of parallel collectors.
//!
//! And, unlike `rayon` which supports two "modes" (*pull mode* for producers
//! and *push mode* for consumers), this crate supports one and only one mode: consumers.
//!
//! # Consumer
//!
//! Although the signatures are different, a consumer here are pretty close to
//! `rayon`'s consumers: it supports splitting (either at a given index or approximately),
//! converting itself into something to collect items serially, producing and output,
//! and reducing with other outputs.
//!
//! The biggest difference is [`Consumer...Base`](ConsumerBase) which is without the item type,
//! and the real consumer, [`Consumer<T>: ConsumerBase`](Consumer), which is with the item type.
//! It sounds like some kinds of OOP cargo cult, but this is a technique
//! `komadori` uses to "delay" the item type commitment until being fed.
//! Without it, many adapters would not work, such as `take()` and `fuse()`
//! (`type annotations needed` compile error!), and `tee_mut()` and `tee_funnel()`
//! (the first collector must work with mutable references of any lifetimes,
//! and early item type commitment destroys it).
//!
//! Another difference is, unlike `rayon`, consumer types have to be `pub` to support
//! that "delayed item commitment" trick. It is kind of a limitation for now.
//! As of now, we require all consumer types to be `pub`, but also `#[doc(hidden)]` to
//! not polute the API surface and become semver-friendly.
//! Caller must **not** refer to those types directly since they do not follow the semver,
//! but indirectly via [`DefineConsumer`] and [`DefineUnindexedConsumer`].
//!
//! Earlier, it is said that a consumer is borrowed from the parallel collector,
//! and consumer types vary between parallel collectors, and they share the fact
//! that they hold a lifetime. Is it a perfect use case of generic assosciated
//! types (GAT), right? In an ideal world when
//! [this limitation](https://blog.rust-lang.org/2022/10/28/gats-stabilization/#implied-static-requirement-from-higher-ranked-trait-bounds)
//! did not exist, we could use them and the API surface would look more elegant
//! than ever! But, back to reality, it exists, so we choose not to use it, but a
//! [hack by Sabrina Jewson](https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats#the-better-gats),
//! reflected by [`DefineConsumer`] and [`DefineUnindexedConsumer`].
//!
//! # Why "committer"?
//!
//! Simple: consumer's output ≠ parallel collector's output. Consumer's output
//! should only be treated as an intermediate result. For example, for [`Vec`],
//! its consumer's output is just a "write proof" that holds how many slots
//! have been written for the indexed path, and a linked list of [`Vec`] chunks
//! for the unindexed path. Can [`Vec`] use them immediately? No!
//!
//! Moreover, arguably, consumer's output is not actually an "output" in a sense
//! (it got its name due to [`ConsumerBase: IntoCollectorBase`](IntoCollectorBase)),
//! because that "output" is also used as an input of something else, which is...
//! a "committer."
//!
//! A committer is simply an [`FnOnce`] that takes the consumer's output and "commit"
//! it back to the parallel collector, completing a "cycle." Back to the previous examples,
//! for the indexed path, the job of the [`Vec`]'s committer is to verify
//! the number of writes match the expectation, and for the unindexed path,
//! the job is to concatenate those small [`Vec`] chunks into the bigger [`Vec`].
//!
//! Its type is a return-position `impl` trait in trait (RPITIT)
//! because knowing the type and putting bounds on it do not make sense whatsoever,
//! and since it does not partitipate in the "delayed item commitment" trick above,
//! it should be completely hidden.
//! Implementors can define a committer with simply... a closure!
//! Also, RPITIT does also work as a GAT that borrow from the paralle collector,
//! so it can hold a mutable reference to the paralle collector too!
//!
//! # Where are `bridge()` and its friends?
//!
//! Damn, this crate is **not** a thread pool library! The philosophy is different:
//! The crate only defines parallel reductions,
//! and it is up to the callers to choose how to drive them.
//! Originally it was not planned to be that way. It was later when I could see
//! its thread-pool-agnostic potential.
//! The main driver is `rayon` (which is the original plan), but you can use
//! other drivers too, such as `chili` like in `par_iter` crate.
//! Note that [`feed_into()`](crate::iter::RayonParallelIteratorExt)
//! disappears as long as you "reject" `rayon`, since it only works
//! with `rayon`'s parallel iterators, but it is also easy to build
//! a wrapper around the crate's abstractions!
//!
//! # How to implement a parallel collector?
//!
//! Coming soon...

use std::ops::ControlFlow;

use komadori::prelude::*;

/// Defines the (indexed) consumer type used by an indexed parallel collector.
///
/// Implementors should implement this for every lifetime outlived by the implemented type.
/// [`ParallelCollectorBase`](super::ParallelCollectorBase) extends this trait
/// with *any* lifetimes, so it is pointless to not doing so.
pub trait DefineConsumer<'this, Binder: self_binder::Sealed = self_binder::Binder<'this, Self>>:
    Sized
{
    /// Which (indexed) consumer being produced?
    type Consumer: ConsumerBase;
}

/// Defines the unindexed consumer type used by an unindexed parallel collector.
///
/// Implementors should implement this for every lifetime outlived by the implemented type.
/// [`UnindexedParallelCollectorBase`](super::UnindexedParallelCollectorBase)
/// extends this trait with *any* lifetimes, so it is pointless to not doing so.
pub trait DefineUnindexedConsumer<
    'this,
    Binder: self_binder::Sealed = self_binder::Binder<'this, Self>,
>: Sized
{
    /// Which unindexed consumer being produced?
    type UnindexedConsumer: UnindexedConsumerBase;
}

/// Used for the hack. Should not be able to be referred outside.
mod self_binder {
    use std::marker::PhantomData;

    pub trait Sealed {}
    #[allow(missing_debug_implementations)]
    pub struct Binder<'a, T>(PhantomData<&'a mut T>);
    impl<'a, T> Sealed for Binder<'a, T> {}
}

/// An (indexed) consumer that can be split at a given index.
///
/// A consumer is able to convert into a serial collector, hence
/// [`: IntoCollectorBase`](IntoCollectorBase) exists.
///
/// After the two split consumers are processed to two outputs,
/// you use a provided combiner to combine those two.
pub trait ConsumerBase: IntoCollectorBase<Output: Send> + Send + Sized {
    /// Which combiner being produced?
    type Combiner: Combiner<Self::Output>;

    /// Produces the "left" consumer and a combiner. After calling this method,
    /// this consumer should be treated as the "right" consumer,
    /// effectively being split.
    /// After both produce outputs, the outputs are combined
    /// using that combiner.
    fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner);

    /// Returns whether th serial collector stops accumulating
    /// after being converted into.
    ///
    /// Note that even if this method returns [`Break(())`](ControlFlow::Break),
    /// the consumer can still be split freely.
    /// It is a hint used for the driver to stop splitting further,
    /// but adapters may still ignore the hint and split anyway.
    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}

/// An unindexed consumer that can be split freely without an index.
///
/// After the two split consumers are processed to two outputs,
/// you use a provided combiner to combine those two.
pub trait UnindexedConsumerBase: ConsumerBase {
    /// Produces the "left" consumer. After calling this method,
    /// this consumer should be treated as the "right" consumer,
    /// effectively being split.
    /// After both produce outputs, the outputs are combined
    /// using the combiner produced by [`to_combiner()`](Self::to_combiner).
    fn split_off_left(&self) -> Self;

    /// Produces a combiner to combine the outputs
    /// of the two split of the consumers.
    fn to_combiner(&self) -> Self::Combiner;
}

/// A combiner used to combine the outputs of the two split of the consumers.
pub trait Combiner<O> {
    /// Combines two outputs by merging the "right" output
    /// into the "left" one.
    fn combine(self, left: &mut O, right: O);
}

/// Defines what item types are collected in an (indexed) consumer.
///
/// You cannot implement this trait directly. You should instead define the item type
/// of serial collectors produced by this consumer.
pub trait Consumer<T>: ConsumerBase<IntoCollector: Collector<T>> {}
impl<C, T> Consumer<T> for C where C: ConsumerBase<IntoCollector: Collector<T>> {}

/// Defines what item types are collected in an unindexed consumer.
///
/// You cannot implement this trait directly. You should instead define the item type
/// of serial collectors produced by this consumer.
pub trait UnindexedConsumer<T>: UnindexedConsumerBase<IntoCollector: Collector<T>> {}
impl<C, T> UnindexedConsumer<T> for C where C: UnindexedConsumerBase<IntoCollector: Collector<T>> {}

fn _unindexed_substitutable_to_indexed<T>(consumer: impl UnindexedConsumer<T>) {
    fn check_consumer<T>(_: impl Consumer<T>) {}
    check_consumer::<T>(consumer);
}
