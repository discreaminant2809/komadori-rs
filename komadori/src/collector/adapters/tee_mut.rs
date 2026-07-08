use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

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
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{
        CollectorTestParts, CollectorTester, CollectorTesterExt, PredError, none_iter_for_fuse_test,
    };

    proptest! {
        /// Precondition:
        /// - [`crate::collector::CollectorBase::take()`]
        /// - [`crate::collector::CollectorBase::copying()`]
        /// - [`crate::collector::CollectorBase::funnel()`]
        /// - [`crate::vec::IntoCollector`]
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=4),
            first_count in ..=4_usize,
            second_count in ..=4_usize,
        ) {
            all_collect_methods_impl(nums, first_count, second_count)?;
        }
    }

    fn all_collect_methods_impl(
        nums: Vec<i32>,
        first_count: usize,
        second_count: usize,
    ) -> TestCaseResult {
        Tester::new(nums, first_count, second_count).test_collector()
    }

    struct Tester {
        nums: Vec<i32>,
        nums_for_iter: Vec<i32>,
        first_count: usize,
        second_count: usize,
    }

    impl Tester {
        fn new(nums: Vec<i32>, first_count: usize, second_count: usize) -> Self {
            Self {
                nums_for_iter: nums.clone(),
                nums,
                first_count,
                second_count,
            }
        }
    }

    impl CollectorTester for Tester {
        type Item<'a> = &'a mut i32;
        type Output<'a> = (Vec<i32>, Vec<i32>);

        fn collector_test_parts<'a>(
            &'a mut self,
        ) -> CollectorTestParts<
            impl Iterator<Item = Self::Item<'a>>,
            impl Collector<Self::Item<'a>, Output = Self::Output<'a>>,
            impl FnMut(
                Self::Output<'a>,
                &mut dyn Iterator<Item = Self::Item<'a>>,
            ) -> Result<(), PredError>,
            impl Iterator<Item = Self::Item<'a>>,
        > {
            let Self {
                first_count,
                second_count,
                ref mut nums,
                ref mut nums_for_iter,
                ..
            } = *self;

            CollectorTestParts {
                iter: nums_for_iter.iter_mut(),
                collector: vec![]
                    .into_collector()
                    .copying()
                    .take(first_count)
                    .tee_mut(vec![].into_collector().copying().take(second_count)),
                should_break: first_count.max(second_count) <= nums.len(),
                pred: move |(first_output, second_output), remaining| {
                    let max_len = first_count.max(second_count);

                    if first_output != nums[..first_count.min(nums.len())]
                        || second_output != nums[..second_count.min(nums.len())]
                    {
                        Err(PredError::IncorrectOutput)
                    } else if nums[max_len.min(nums.len())..]
                        .iter()
                        .copied()
                        .ne(remaining.map(|&mut item| item))
                    {
                        Err(PredError::IncorrectIterConsumption)
                    } else {
                        Ok(())
                    }
                },
                iter_for_fuse_test: none_iter_for_fuse_test(),
            }
        }
    }
}
