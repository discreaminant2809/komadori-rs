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

#[cfg(test)]
mod proptests {
    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let mut n1 = ..=5_usize;
            let mut n2 = ..=5_usize;
        },
        iter: nums.par_iter().cloned(),
        collector: vec![]
            .into_par_collector()
            .take(n1)
            .tee(vec![].into_par_collector().take(n2)),
        starting_bh: if n1.max(n2) > 0 { Continue(()) } else { Break(()) },
        expected_f: |mut iter, count| {
            let max_n = n1.max(n2);
            let min_n = n1.min(n2);

            let (mut first, mut second): (Vec<_>, Vec<_>) =
                iter.by_ref().take(min_n).map(|num| (num, num)).collect();

            if n1 < n2 {
                second.extend(iter.take(max_n - min_n));
            } else {
                first.extend(iter.take(max_n - min_n));
            };

            (
                (first, second),
                if count < n1.max(n2) {
                    Continue(())
                } else {
                    Break(())
                },
            )
        },
        output_pred: PartialEq::eq,
        state_pred: state_is_irrelevant(),
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            let mut nums1 = propvec(any::<i32>(), ..=3);
            let mut nums2 = propvec(any::<i32>(), ..=3);
        },
        other_data: {
            let mut n1 = ..=5_usize;
            let mut n2 = ..=5_usize;
        },
        iter: nums1
            .par_iter()
            .chain(nums2.par_iter().filter(|&&num| num >= 0))
            .cloned(),
        collector: vec![]
            .into_par_collector()
            .take(n1)
            .tee(vec![].into_par_collector().take(n2)),
        starting_bh: if n1.max(n2) > 0 { Continue(()) } else { Break(()) },
        expected_f: |iter, count| (
            iter.collect::<Vec<_>>(),
            if count < n1.max(n2) {
                Continue(())
            } else {
                Break(())
            },
        ),
        output_pred: |(actual1, actual2), nums| {
            actual1.len() == nums.len().min(n1)
                && is_subsequence(actual1, nums)
                && actual2.len() == nums.len().min(n2)
                && is_subsequence(actual2, nums)
        },
        state_pred: state_is_irrelevant(),
    });
}
