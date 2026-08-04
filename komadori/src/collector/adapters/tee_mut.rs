use core::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

use super::{DefinePassDown, TeeBase, Teer};

/// A collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`CollectorBase::tee_mut()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct TeeMut<C1, C2> {
    base: TeeBase<C1, C2, MutTeer>,
}

impl<C1, C2> TeeMut<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            base: TeeBase::new(collector1, collector2, MutTeer),
        }
    }
}

impl<C1, C2> CollectorBase for TeeMut<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    type Output = (C1::Output, C2::Output);

    #[inline]
    fn finish(self) -> Self::Output {
        self.base.finish()
    }

    finish_boxed_impl!();

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.base.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.base.max_afford(request)
    }
}

impl<'i, T, C1, C2> Collector<&'i mut T> for TeeMut<C1, C2>
where
    C1: for<'a> Collector<&'a mut T>,
    C2: Collector<&'i mut T>,
    T: ?Sized,
{
    #[inline]
    fn collect(&mut self, item: &'i mut T) -> ControlFlow<()> {
        self.base.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut T>) -> ControlFlow<()> {
        self.base.collect_many(items)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = &'i mut T>) -> Self::Output {
        self.base.collect_then_finish(items)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: &'i mut T) -> ControlFlow<()> {
        // SAFETY: `TeeBase` alerady handles the invariants.
        unsafe { self.base.assume_reserved_collect(item) }
    }
}

#[derive(Debug, Clone)]
struct MutTeer;

impl<'a, T> DefinePassDown<'a, &mut T> for MutTeer
where
    T: ?Sized,
{
    type PassDown = &'a mut T;
}

impl<'i, T> Teer<&'i mut T> for MutTeer
where
    T: ?Sized,
{
    const TEE_CHEAP: bool = true;

    #[inline]
    fn pass_down<'a>(
        &mut self,
        item: &'a mut &'i mut T,
    ) -> <Self as DefinePassDown<'a, &'i mut T>>::PassDown {
        item
    }

    #[inline]
    fn no_tee_collect(
        &mut self,
        collector: &mut impl for<'a> Collector<<Self as DefinePassDown<'a, &'i mut T>>::PassDown>,
        item: &'i mut T,
    ) -> ControlFlow<()> {
        collector.collect(item)
    }

    #[inline]
    unsafe fn no_tee_assume_reserved_collect(
        &mut self,
        collector: &mut impl for<'a> Collector<<Self as DefinePassDown<'a, &'i mut T>>::PassDown>,
        item: &'i mut T,
    ) -> ControlFlow<()> {
        // SAFETY: The caller has reserved for one item.
        unsafe { collector.assume_reserved_collect(item) }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::super::take_collector_model;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let first_n = ..=5_usize;
            let second_n = ..=5_usize;
        },
        iters: {
            let mut other_nums = nums.repeat(2);
            let (for_output, for_model) = other_nums.split_at_mut(nums.len());
            (nums.iter_mut(), for_output.iter_mut(), for_model.iter_mut())
        },
        collector: vec![]
            .into_collector()
            .take(first_n)
            .tee_mut(vec![].into_collector().take(second_n)),
        expected_f: |iter, count| {
            let mut iter = iter.map(|&mut num| num);

            let max_n = first_n.max(second_n);
            let min_n = first_n.min(second_n);

            let (mut first, mut second): (Vec<_>, Vec<_>) =
                iter.by_ref().take(min_n).map(|num| (num, num)).collect();

            if first_n < second_n {
                second.extend(iter.take(max_n - min_n));
            } else {
                first.extend(iter.take(max_n - min_n));
            };

            ((first, second), count >= first_n.max(second_n))
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(first_n.max(second_n)),
    });
}
