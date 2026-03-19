//!

mod adapters;
mod indexed_par_collector;
mod into_par_collector;
mod par_collector;
mod par_collector_base;
mod par_collector_by_mut;
mod par_collector_by_ref;
pub mod plumbing;

pub use adapters::*;
pub use indexed_par_collector::*;
pub use into_par_collector::*;
pub use par_collector::*;
pub use par_collector_base::*;
pub use par_collector_by_mut::*;
pub use par_collector_by_ref::*;

#[inline(always)]
pub(crate) fn assert_par_collector_base<C>(x: C) -> C
where
    C: ParallelCollectorBase,
{
    x
}

#[inline(always)]
pub(crate) fn assert_indexed_par_collector<C, T>(x: C) -> C
where
    C: IndexedParallelCollector<T>,
{
    x
}

#[inline(always)]
pub(crate) fn assert_par_collector<C, T>(x: C) -> C
where
    C: ParallelCollector<T>,
{
    x
}
