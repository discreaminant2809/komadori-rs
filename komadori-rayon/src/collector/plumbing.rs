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
//! A difference, however, is unlike `rayon`, consumer types have to be `pub` to support
//! that "delayed item commitment" trick. It is kind of a limitation for now.
//! As of now, we require all consumer types to be `pub`, but behind private modules to
//! not polute the API surface and become more semver-friendly.
//! Caller must **not** refer to those types directly since they do not follow the semver,
//! but indirectly via [`DefineSerial`] and [`DefineUnindexedSerial`].
//!
//! Earlier, it is said that a consumer is borrowed from the parallel collector,
//! and consumer types vary between parallel collectors, and they share the fact
//! that they hold a lifetime. Is it a perfect use case of generic assosciated
//! types (GAT), right? In an ideal world when
//! [this limitation](https://blog.rust-lang.org/2022/10/28/gats-stabilization/#implied-static-requirement-from-higher-ranked-trait-bounds)
//! did not exist, we could use them and the API surface would look more elegant
//! than ever! But, back to reality, it exists, so we choose not to use it, but a
//! [hack by Sabrina Jewson](https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats#the-better-gats),
//! reflected by [`DefineSerial`] and [`DefineUnindexedSerial`].
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

/// Re-exported so that you do not need to import `komadori`.
pub use komadori::collector::{Collector, CollectorBase, IntoCollector, IntoCollectorBase};

/// Defines the serial collector type used by an (indexed) parallel collector.
///
/// Implementors should implement this for every lifetime outlived by the implemented type.
/// [`ParallelCollectorBase`](super::ParallelCollectorBase) extends this trait
/// with *any* lifetimes, so it is pointless to not doing so.
pub trait DefineSerial<'this, Binder: self_binder::Sealed = self_binder::Binder<'this, Self>> {
    /// Which (indexed) consumer being produced?
    type Serial: CollectorBase<Output: Send>;
}

/// Defines the serial collector type used by an unindexed parallel collector.
///
/// Implementors should implement this for every lifetime outlived by the implemented type.
/// [`UnindexedParallelCollectorBase`](super::UnindexedParallelCollectorBase)
/// extends this trait with *any* lifetimes, so it is pointless to not doing so.
pub trait DefineUnindexedSerial<'this, Binder: self_binder::Sealed = self_binder::Binder<'this, Self>> {
    /// Which unindexed consumer being produced?
    type UnindexedSerial: CollectorBase<Output: Send>;
}

/// Used for the hack. Should not be able to be referred outside.
mod self_binder {
    use std::marker::PhantomData;

    pub trait Sealed {}
    #[allow(missing_debug_implementations)]
    pub struct Binder<'a, T: ?Sized>(PhantomData<&'a mut T>);
    impl<'a, T: ?Sized> Sealed for Binder<'a, T> {}
}

/// An (indexed) consumer that can be split at a given index.
///
/// A consumer is able to convert into a serial collector, hence
/// [`: IntoCollectorBase`](IntoCollectorBase) exists.
///
/// After the two split consumers are processed to two outputs,
/// you use a provided combiner to combine those two.
pub trait Consumer: IntoCollectorBase<Output: Send> + Send + Sized {
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

    // fn map_collector<F, C>(self, f: F, g: G) -> impl Consumer<IntoCollector = C>
    // where
    //     F: FnOnce(Self::IntoCollector) -> C + Clone,
    //     C: CollectorBase,
    // {
    //     struct ConsumerAdapter<C, F> {
    //         consumer: C,
    //         f: F,
    //     }

    //     struct CombinerAdapter<C> {
    //         combiner: C,
    //     }

    //     impl<C, F, SC> IntoCollectorBase for ConsumerAdapter<C, F>
    //     where
    //         C: IntoCollectorBase,
    //         F: FnOnce(C::IntoCollector) -> SC,
    //         SC: CollectorBase,
    //     {
    //         type Output = SC::Output;

    //         type IntoCollector = SC;

    //         #[inline]
    //         fn into_collector(self) -> Self::IntoCollector {
    //             (self.f)(self.consumer.into_collector())
    //         }
    //     }

    //     impl<C, F, SC> Consumer for ConsumerAdapter<C, F>
    //     where
    //         C: Consumer,
    //         F: FnOnce(C::IntoCollector) -> SC + Clone + Send,
    //         SC: CollectorBase<Output: Send>,
    //     {
    //         type Combiner = CombinerAdapter<C::Combiner>;

    //         fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
    //             let (consumer, combiner) = self.consumer.split_off_left_at(index);
    //             (
    //                 Self {
    //                     consumer,
    //                     f: self.f.clone(),
    //                 },
    //                 CombinerAdapter { combiner },
    //             )
    //         }
    //     }
    // }
}

/// An unindexed consumer that can be split freely without an index.
///
/// After the two split consumers are processed to two outputs,
/// you use a provided combiner to combine those two.
pub trait UnindexedConsumer: Consumer {
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

/// Defines a wrapper that makes your serial collector type "unique."
#[macro_export]
macro_rules! uniquify_serial {
    ($mod_name:ident, unindexed = false) => {
        #[allow(missing_debug_implementations)]
        mod $mod_name {
            use $crate::collector::plumbing::{self, Collector, CollectorBase, IntoCollectorBase};

            use ::core::{any::Any, marker::PhantomData, ops::ControlFlow};

            type InvariantLtAndNoAutoTraits<'a, This> =
                PhantomData<(fn(&'a mut This) -> &'a mut This, dyn Any)>;

            struct Consumer<'a, This, C> {
                consumer: C,
                _marker: InvariantLtAndNoAutoTraits<'a, This>,
            }
            // SAFETY: we're JUST a marker.
            unsafe impl<This, C: Send> Send for Consumer<'_, This, C> {}

            struct Combiner<C>(C);

            pub struct Serial<'a, This, C> {
                collector: C,
                _marker: InvariantLtAndNoAutoTraits<'a, This>,
            }

            pub struct Output<'a, This, O> {
                output: O,
                _marker: InvariantLtAndNoAutoTraits<'a, This>,
            }
            // SAFETY: we're JUST a marker.
            unsafe impl<This, O: Send> Send for Output<'_, This, O> {}

            #[inline]
            pub fn uniquify<'a, This: 'a, C: CollectorBase>(
                parts: (
                    usize,
                    impl plumbing::Consumer<IntoCollector = C, Output = C::Output> + 'a,
                    impl FnOnce(C::Output) -> ControlFlow<()> + 'a,
                ),
            ) -> (
                usize,
                impl plumbing::Consumer<
                    IntoCollector = Serial<'a, This, C>,
                    Output = Output<'a, This, C::Output>,
                > + 'a,
                impl FnOnce(Output<'a, This, C::Output>) -> ControlFlow<()> + 'a,
            ) {
                let (len, consumer, commit) = parts;
                (
                    len,
                    Consumer {
                        consumer,
                        _marker: PhantomData,
                    },
                    |output| commit(output.output),
                )
            }

            #[inline]
            pub fn take_uniquify<'a, This: 'a, C: CollectorBase>(
                parts: (
                    usize,
                    impl plumbing::Consumer<IntoCollector = C, Output = C::Output> + 'a,
                    impl FnOnce(C::Output) + 'a,
                ),
            ) -> (
                usize,
                impl plumbing::Consumer<
                    IntoCollector = Serial<'a, This, C>,
                    Output = Output<'a, This, C::Output>,
                > + 'a,
                impl FnOnce(Output<'a, This, C::Output>) + 'a,
            ) {
                let (len, consumer, commit) = parts;
                (
                    len,
                    Consumer {
                        consumer,
                        _marker: PhantomData,
                    },
                    |output| commit(output.output),
                )
            }

            impl<'a, This, C> IntoCollectorBase for Consumer<'a, This, C>
            where
                C: plumbing::Consumer,
            {
                type Output = Output<'a, This, C::Output>;

                type IntoCollector = Serial<'a, This, C::IntoCollector>;

                #[inline]
                fn into_collector(self) -> Self::IntoCollector {
                    Serial {
                        collector: self.consumer.into_collector(),
                        _marker: PhantomData,
                    }
                }
            }

            impl<This, C> plumbing::Consumer for Consumer<'_, This, C>
            where
                C: plumbing::Consumer,
            {
                type Combiner = Combiner<C::Combiner>;

                #[inline]
                fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
                    let (consumer, combiner) = self.consumer.split_off_left_at(index);
                    (
                        Self {
                            consumer,
                            _marker: PhantomData,
                        },
                        Combiner(combiner),
                    )
                }

                #[inline]
                fn break_hint(&self) -> ControlFlow<()> {
                    self.consumer.break_hint()
                }
            }

            impl<'a, This, C, O> plumbing::Combiner<Output<'a, This, O>> for Combiner<C>
            where
                C: plumbing::Combiner<O>,
            {
                #[inline]
                fn combine(self, left: &mut Output<'a, This, O>, right: Output<'a, This, O>) {
                    self.0.combine(&mut left.output, right.output);
                }
            }

            impl<'a, This, C> CollectorBase for Serial<'a, This, C>
            where
                C: CollectorBase,
            {
                type Output = Output<'a, This, C::Output>;

                #[inline]
                fn finish(self) -> Self::Output {
                    Output {
                        output: self.collector.finish(),
                        _marker: PhantomData,
                    }
                }

                #[inline]
                fn break_hint(&self) -> ControlFlow<()> {
                    self.collector.break_hint()
                }
            }

            impl<C, This, T> Collector<T> for Serial<'_, This, C>
            where
                C: Collector<T>,
            {
                #[inline]
                fn collect(&mut self, item: T) -> ControlFlow<()> {
                    self.collector.collect(item)
                }

                #[inline]
                fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
                    self.collector.collect_many(items)
                }

                #[inline]
                fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
                    Output {
                        output: self.collector.collect_then_finish(items),
                        _marker: PhantomData,
                    }
                }
            }
        }
    };

    ($mod_name:ident, unindexed = true) => {
        #[allow(missing_debug_implementations, )]
        mod $mod_name {
            use $crate::collector::plumbing::{self, Collector, CollectorBase, IntoCollectorBase};

            use ::core::{any::Any, marker::PhantomData, ops::ControlFlow};

            type InvariantLtAndNoAutoTraits<'a, This> =
                PhantomData<(fn(&'a mut This) -> &'a mut This, dyn Any)>;

            struct Consumer<'a, This, C> {
                consumer: C,
                _marker: InvariantLtAndNoAutoTraits<'a, This>,
            }
            // SAFETY: we're JUST a marker.
            unsafe impl<This, C: Send> Send for Consumer<'_, This, C> {}

            struct Combiner<C>(C);

            pub struct Serial<'a, This, C> {
                collector: C,
                _marker: InvariantLtAndNoAutoTraits<'a, This>,
            }

            pub struct Output<'a, This, O> {
                output: O,
                _marker: InvariantLtAndNoAutoTraits<'a, This>,
            }
            // SAFETY: we're JUST a marker.
            unsafe impl<This, O: Send> Send for Output<'_, This, O> {}

            #[inline]
            pub fn uniquify<'a, This: 'a, C: CollectorBase>(
                parts: (
                    impl plumbing::UnindexedConsumer<IntoCollector = C, Output = C::Output> + 'a,
                    impl FnOnce(C::Output) -> ControlFlow<()> + 'a,
                ),
            ) -> (
                impl plumbing::UnindexedConsumer<
                    IntoCollector = Serial<'a, This, C>,
                    Output = Output<'a, This, C::Output>,
                > + 'a,
                impl FnOnce(Output<'a, This, C::Output>) -> ControlFlow<()> + 'a,
            ) {
                let (consumer, commit) = parts;
                (
                    Consumer {
                        consumer,
                        _marker: PhantomData,
                    },
                    |output| commit(output.output),
                )
            }

            #[inline]
            pub fn take_uniquify<'a, This: 'a, C: CollectorBase>(
                parts: (
                    impl plumbing::UnindexedConsumer<IntoCollector = C, Output = C::Output> + 'a,
                    impl FnOnce(C::Output) + 'a,
                ),
            ) -> (
                impl plumbing::UnindexedConsumer<
                    IntoCollector = Serial<'a, This, C>,
                    Output = Output<'a, This, C::Output>,
                > + 'a,
                impl FnOnce(Output<'a, This, C::Output>) + 'a,
            ) {
                let (consumer, commit) = parts;
                (
                    Consumer {
                        consumer,
                        _marker: PhantomData,
                    },
                    |output| commit(output.output),
                )
            }

            impl<'a, This, C> IntoCollectorBase for Consumer<'a, This, C>
            where
                C: plumbing::UnindexedConsumer,
            {
                type Output = Output<'a, This, C::Output>;

                type IntoCollector = Serial<'a, This, C::IntoCollector>;

                #[inline]
                fn into_collector(self) -> Self::IntoCollector {
                    Serial {
                        collector: self.consumer.into_collector(),
                        _marker: PhantomData,
                    }
                }
            }

            impl<This, C> plumbing::Consumer for Consumer<'_, This, C>
            where
                C: plumbing::UnindexedConsumer,
            {
                type Combiner = Combiner<C::Combiner>;

                #[inline]
                fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
                    let (consumer, combiner) = self.consumer.split_off_left_at(index);
                    (
                        Self {
                            consumer,
                            _marker: PhantomData,
                        },
                        Combiner(combiner),
                    )
                }

                #[inline]
                fn break_hint(&self) -> ControlFlow<()> {
                    self.consumer.break_hint()
                }
            }

            impl<This, C> plumbing::UnindexedConsumer for Consumer<'_, This, C>
            where
                C: plumbing::UnindexedConsumer,
            {
                #[inline]
                fn split_off_left(&self) -> Self {
                    Self {
                        consumer: self.consumer.split_off_left(),
                        _marker: PhantomData,
                    }
                }

                #[inline]
                fn to_combiner(&self) -> Self::Combiner {
                    Combiner(self.consumer.to_combiner())
                }
            }

            impl<'a, This, C, O> plumbing::Combiner<Output<'a, This, O>> for Combiner<C>
            where
                C: plumbing::Combiner<O>,
            {
                #[inline]
                fn combine(self, left: &mut Output<'a, This, O>, right: Output<'a, This, O>) {
                    self.0.combine(&mut left.output, right.output);
                }
            }

            impl<'a, This, C> CollectorBase for Serial<'a, This, C>
            where
                C: CollectorBase,
            {
                type Output = Output<'a, This, C::Output>;

                #[inline]
                fn finish(self) -> Self::Output {
                    Output {
                        output: self.collector.finish(),
                        _marker: PhantomData,
                    }
                }

                #[inline]
                fn break_hint(&self) -> ControlFlow<()> {
                    self.collector.break_hint()
                }
            }

            impl<C, This, T> Collector<T> for Serial<'_, This, C>
            where
                C: Collector<T>,
            {
                #[inline]
                fn collect(&mut self, item: T) -> ControlFlow<()> {
                    self.collector.collect(item)
                }

                #[inline]
                fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
                    self.collector.collect_many(items)
                }

                #[inline]
                fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
                    Output {
                        output: self.collector.collect_then_finish(items),
                        _marker: PhantomData,
                    }
                }
            }
        }
    };

    ($mod_name:ident) => {
        $crate::uniquify_serial!($mod_name, unindexed = false);
    };
}
