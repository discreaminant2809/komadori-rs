//!

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

#[inline(always)]
pub(crate) fn assert_par_collector_base<C>(x: C) -> C
where
    C: ParallelCollectorBase,
{
    x
}

#[inline(always)]
pub(crate) fn assert_unindexed_par_collector_base<C>(x: C) -> C
where
    C: UnindexedParallelCollectorBase,
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

#[inline(always)]
pub(crate) fn assert_unindexed_par_collector<C, T>(x: C) -> C
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
