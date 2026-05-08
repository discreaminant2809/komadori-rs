//! Module containing traits and `struct`s for parallel collectors.
//!
//! Parallel collectors let you express parallel reduction operations
//! in a declarative and composable way.
//! If you want to "reduce" a collection
//! or a stream of items into another collection or computation in parallel,
//! you will likely reach for parallel collectors.
//!
//! All traits in this module are re-exported in [`prelude`](crate::prelude).
//! You rarely need to import individual traits from here directly.
//!
//! [`ParallelCollectorBase`] and [`ParallelCollector`] define an indexed collector,
//! and [`UnindexedParallelCollectorBase`] and [`UnindexedParallelCollector`]
//! define an unindexed collector.
//! See their documentation for more.
//!
//! If you want to implement your own parallel collector, see [`plumbing`].
//!
//! # Indexed and unindexed
//!
//! This crate follows the `rayon`'s model, which distinguishes between "indexed" and "unindexed."
//!
//! In a nutshell:
//!
//! - "Indexed" means each item lands in a pre-determined index and the amount
//!   can be known upfront.
//!
//! - "Unindexed" means each item may land randomly in anywhere on a collector,
//!   and the amount cannot be known upfront.
//!
//! Therefore, this crate provides 4 traits (2 from `komadori` × 2 from indexed-ness):
//!
//! - [`ParallelCollectorBase`] and [`ParallelCollector`]:
//!   A parallel collector that can only prepare a fixed "region" for items to land on
//!   and only allows items to land on specific indices it set up.
//!   Its consumers can only be split with an index, and when converted into a
//!   (serial) collector, it must be fed the exact amount of items before finishing.
//!
//!   For example, `enumerate()` is a such parallel collector. It only allows
//!   items to land on preset indices since it has to produce correct indices for
//!   the underlying collector.
//!
//! - [`UnindexedParallelCollectorBase`] and [`UnindexedParallelCollector`]:
//!   A parallel collector that allows items to land on wherever they like.
//!   Typically, its consumers can affort any number of items (including
//!   no items at all) without assuming any amount upfront.
//!
//! Since consumers of an unindexed parallel collector can also be split
//! with indices (by ignoring it) and allow deterministic landing,
//! `UnindexedParallelCollector: ParallelCollector`.
//! This means that unindexed parallel collectors can also be used when
//! deterministic item landing is expected, but the reverse is not true.
//!
//! Every adapters work on every kind of parallel collectors,
//! but only some adapters work on unindexed parallel collectors.
//! For example: [`filter()`]. It is because this adapter
//! filters out items, which makes the amount of items potentially
//! less than the reported amount, violating the expectation of
//! the indexed path. Hence, the underlying collector must provide
//! an unindexed path, which only unindexed parallel collectors can do.
//! [`filter()`] has an indexed path too, which... still uses the unindexed path
//! of the underlying collector regardless.
//!
//! # `tee()` adapter variants
//!
//! Similar to serial collectors, parallel collectors have multiple
//! variants of `tee()`. They are similar to the serial one.
//!
//! See [here](komadori::collector#tee-adapter-variants)
//! for more infomation.
//!
//! # Unspecified behaviors
//!
//! Unless stated otherwise by the parallel collector’s implementation,
//! after [`break_hint()`] or the committer from [`parts()`] or [`parts_unindexed()`]
//! have returned [`Break(())`] once,
//! behaviors of subsequent calls to [`break_hint()`] are unspecified.
//! You can still call [`parts()`], [`parts_unindexed()`],
//! [`take_parts()`] and [`take_parts_unindexed()`] and split the resulting consumers,
//! but the converted serial collectors are counted to have returned [`Break(())`] before
//! (see [here](komadori::collector#unspecified-behaviors) for what happens
//! for such serial collectors and what should be done next).
//! They may panic, overflow, or even resume accumulation
//! (similar to how [`Iterator::next()`] might yield again after returning [`None`]).
//! Callers should generally call [`finish()`](ParallelCollectorBase::finish)
//! once a parallel collector has signaled a stop.
//! If this invariant cannot be upheld, wrap it with [`fuse()`](ParallelCollectorBase::fuse).
//! Furthermore, a parallel collector is in an unspecified state if panicked.
//!
//! Additionally, after calling [`take_parts()`] and [`take_parts_unindexed()`],
//! a parallel collector is counted to have been "taken," and behaviors of
//! subsequent calls to [`break_hint()`], [`parts()`], [`parts_unindexed()`],
//! [`take_parts()`] and [`take_parts_unindexed()`] are also unspecified.
//! In this case, caller should generally call [`finish()`](ParallelCollectorBase::finish)
//! afterwards. Unlike the previous one, [`fuse()`](ParallelCollectorBase::fuse)
//! does **not** save you here.
//!
//! In [`parts()`] and [`take_parts()`], the returning `usize` is referred as
//! the "maximum len" the (indexed) parallel collector can actually affort.
//! Implementations must **not** report a "maximum len" greater the given `len`,
//! otherwise the behavior is unspecified.
//!
//! For a serial collector obtained by a consumer of [`parts()`] and [`take_parts()`],
//! at a time, you must feed it at **most** the remaining amount
//! the serial collector would affort.
//! Furthermore, before the serial collector is finished, the remaining amount
//! must be `0`.
//! Behaviors of violating the above are unspecified.
//! The consequence of this is that for such serial collectors,
//! you do not need to check the break hint of
//! [`break_hint()`](komadori::collector::CollectorBase::break_hint)
//! [`collect()`] and [`collect_many()`]; you track outside instead.
//!
//! These loosenesses allows for optimizations (for example, omitting an internal "stopped” flag).
//!
//! Although the behavior is unspecified, none of the aforementioned methods are `unsafe`.
//! Implementors must **not** cause memory corruption, undefined behavior,
//! or any other safety violations, and callers must **not** rely on such outcomes.
//!
//! # Limitations and workarounds
//!
//! Parallel collectors inherit limitations of the serial ones.
//!
//! See [here](komadori::collector#limitations-and-workarounds)
//! for the limitations and workarounds.
//!
//! [`Break(())`]: std::ops::ControlFlow::Break
//! [`break_hint()`]: ParallelCollectorBase::break_hint
//! [`collect()`]: komadori::collector::Collector::collect
//! [`collect_many()`]: komadori::collector::Collector::collect_many
//! [`parts()`]: ParallelCollectorBase::parts
//! [`take_parts()`]: ParallelCollectorBase::take_parts
//! [`parts_unindexed()`]: UnindexedParallelCollectorBase::parts_unindexed
//! [`take_parts_unindexed()`]: UnindexedParallelCollectorBase::take_parts_unindexed
//! [`filter()`]: UnindexedParallelCollectorBase::filter

mod adapters;
mod into_par_collector;
mod par_collector_base;
mod par_collector_by_mut;
mod par_collector_by_ref;
pub mod plumbing;
mod unindexed_par_collector_base;

pub use adapters::*;
pub use into_par_collector::*;
pub use par_collector_base::*;
pub use par_collector_by_mut::*;
pub use par_collector_by_ref::*;
pub use unindexed_par_collector_base::*;

#[allow(unused)]
#[inline(always)]
pub(crate) const fn assert_par_collector_base<C>(x: C) -> C
where
    C: ParallelCollectorBase,
{
    x
}

#[allow(unused)]
#[inline(always)]
pub(crate) const fn assert_unindexed_par_collector_base<C>(x: C) -> C
where
    C: UnindexedParallelCollectorBase,
{
    x
}

#[allow(unused)]
#[inline(always)]
pub(crate) const fn assert_par_collector<C, T>(x: C) -> C
where
    C: ParallelCollector<T>,
{
    x
}

#[allow(unused)]
#[inline(always)]
pub(crate) const fn assert_unindexed_par_collector<C, T>(x: C) -> C
where
    C: UnindexedParallelCollector<T>,
{
    x
}

fn _unindexed_substitutable_to_indexed<C, T>(collector: C)
where
    C: UnindexedParallelCollector<T>,
{
    fn check_collector<C, T>(_: C)
    where
        C: ParallelCollector<T>,
    {
    }
    check_collector::<C, T>(collector);
}
