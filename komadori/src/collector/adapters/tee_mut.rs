use std::{iter, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

use super::Fuse;

/// A collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`CollectorBase::tee_mut()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct TeeMut<C1, C2> {
    collector1: Fuse<C1>,
    collector2: Fuse<C2>,
}

impl<C1, C2> TeeMut<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            collector1: Fuse::new(collector1),
            collector2: Fuse::new(collector2),
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
        (self.collector1.finish(), self.collector2.finish())
    }

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        // We're sure that whether this collector has finished or not is
        // entirely based on the 2nd collector.
        // Also, by this method being called it is assumed that
        // this collector has not finished, which mean the 2nd collector
        // has not finished, which means it's always sound to call here.
        //
        // Since the 1st collector is fused, we won't cause any unsoundness
        // by repeatedly calling it.
        if self.collector1.break_hint().is_break() && self.collector2.break_hint().is_break() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

impl<'i, T, C1, C2> Collector<&'i mut T> for TeeMut<C1, C2>
where
    C1: for<'a> Collector<&'a mut T>,
    C2: Collector<&'i mut T>,
    T: ?Sized,
{
    fn collect(&mut self, item: &'i mut T) -> ControlFlow<()> {
        match (self.collector1.collect(item), self.collector2.collect(item)) {
            (ControlFlow::Break(_), ControlFlow::Break(_)) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut T>) -> ControlFlow<()> {
        self.break_hint()?;

        let mut items = items.into_iter();

        match items.try_for_each(|item| {
            if self.collector1.collect(item).is_break() {
                ControlFlow::Break(Which::First(item))
            } else {
                self.collector2.collect(item).map_break(|_| Which::Second)
            }
        }) {
            ControlFlow::Break(Which::First(item)) => {
                self.collector2.collect_many(iter::once(item).chain(items))
            }
            ControlFlow::Break(Which::Second) => self.collector1.collect_many(items),
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
        }
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = &'i mut T>) -> Self::Output {
        if self.break_hint().is_break() {
            return self.finish();
        }

        let mut items = items.into_iter();

        match items.try_for_each(|item| {
            if self.collector1.collect(item).is_break() {
                ControlFlow::Break(Which::First(item))
            } else {
                self.collector2.collect(item).map_break(|_| Which::Second)
            }
        }) {
            ControlFlow::Break(Which::First(item)) => (
                self.collector1.finish(),
                self.collector2
                    .collect_then_finish(iter::once(item).chain(items)),
            ),
            ControlFlow::Break(Which::Second) => (
                self.collector1.collect_then_finish(items),
                self.collector2.finish(),
            ),
            ControlFlow::Continue(_) => self.finish(),
        }
    }
}

enum Which<T> {
    First(T),
    Second,
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
