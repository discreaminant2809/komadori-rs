use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::ParallelCollectorBase;

use super::{DefinePassDown, TeeBase, Teer};

/// A parallel collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`ParallelCollectorBase::tee()`].
/// See its documentation for more.
pub type Tee<C1, C2> = TeeBase<C1, C2, CopyTeer>;

#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct CopyTeer(());

pub(in crate::collector) fn tee<C1, C2>(collector1: C1, collector2: C2) -> Tee<C1, C2>
where
    C1: ParallelCollectorBase,
    C2: ParallelCollectorBase,
{
    TeeBase::new(collector1, collector2, CopyTeer(()))
}

impl<'this, T> DefinePassDown<'this, T> for CopyTeer
where
    T: Copy,
{
    type PassDown = T;
}

impl<T> Teer<T> for CopyTeer
where
    T: Copy,
{
    const ITEM_IS_COPY: bool = true;

    #[inline]
    fn pass_down(&mut self, item: &mut T) -> T {
        *item
    }

    #[inline]
    fn no_tee_collect(&mut self, collector: &mut impl Collector<T>, item: T) -> ControlFlow<()> {
        collector.collect(item)
    }

    #[inline]
    fn no_tee_collect_many(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: &mut impl Collector<T>,
    ) -> ControlFlow<()> {
        collector.collect_many(items)
    }

    #[inline]
    fn no_tee_collect_then_finish<O>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        collector: impl Collector<T, Output = O>,
    ) -> O {
        collector.collect_then_finish(items)
    }
}

// #[cfg(test)]
// mod proptests {
//     use crate::{collector::ParallelCollectorBase, test_utils::prelude::*};

//     fn indexed_impl(
//         mut pool: CoroutinePool,
//         split_decision: IndexedSplitDecision,
//         starting_nums: Vec<i32>,
//         nums: Vec<i32>,
//         take_count: usize,
//     ) -> TestCaseResult {

//     }
// }
