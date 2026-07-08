use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that can "safely" collect items even after
/// the underlying collector has stopped accumulating,
/// without triggering undesired behaviors.
///
/// This `struct` is created by [`CollectorBase::fuse()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Fuse<C> {
    collector: C,
    stopped: bool,
}

impl<C> Fuse<C>
where
    C: CollectorBase,
{
    #[inline]
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self {
            stopped: collector.max_afford(1) == 0,
            collector,
        }
    }
}

impl<C> Fuse<C>
where
    C: CollectorBase,
{
    #[inline]
    fn collect_impl(&mut self, f: impl FnOnce(&mut C) -> ControlFlow<()>) -> ControlFlow<()> {
        if self.stopped {
            ControlFlow::Break(())
        } else if f(&mut self.collector).is_continue() {
            ControlFlow::Continue(())
        } else {
            self.stopped = true;
            ControlFlow::Break(())
        }
    }
}

impl<C> CollectorBase for Fuse<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        if !self.stopped {
            self.collector.reserve(additional);
        }
    }

    #[inline]
    fn max_afford(&self, amount: usize) -> usize {
        if self.stopped {
            0
        } else {
            self.collector.max_afford(amount)
        }
    }
}

impl<C, T> Collector<T> for Fuse<C>
where
    C: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.collect_impl(|collector| collector.collect(item))
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.collect_impl(|collector| collector.collect_many(items))
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        if self.stopped {
            self.finish()
        } else {
            self.collector.collect_then_finish(items)
        }
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        self.collect_impl(|collector| unsafe {
            // SAFETY: We've reserved for at least one item.
            collector.assume_reserved_collect(item)
        })
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{CollectorTestParts, CollectorTester, CollectorTesterExt, PredError};

    proptest! {
        /// We use
        ///
        /// Precondition:
        /// - [`crate::collector::Collector::take_while()`]
        /// - [`crate::vec::IntoCollector`]
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=5),
            // We only simulate whether the collector has stopped on construction,
            // or stops later (rely on `take_while()` to stop).
            take_count in prop_oneof![
                1 => Just(0),
                9 => Just(999),
            ],
        ) {
            all_collect_methods_impl(nums, take_count)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>, take_count: usize) -> TestCaseResult {
        Tester { nums, take_count }.test_collector()
    }

    struct Tester {
        nums: Vec<i32>,
        take_count: usize,
    }

    impl CollectorTester for Tester {
        type Item<'a> = i32;

        type Output<'a> = Vec<i32>;

        fn collector_test_parts<'a>(
            &'a mut self,
        ) -> crate::test_utils::CollectorTestParts<
            impl Iterator<Item = Self::Item<'a>>,
            impl Collector<Self::Item<'a>, Output = Self::Output<'a>>,
            impl FnMut(
                Self::Output<'a>,
                &mut dyn Iterator<Item = Self::Item<'a>>,
            ) -> Result<(), PredError>,
            impl Iterator<Item = Self::Item<'a>>,
        > {
            CollectorTestParts {
                iter: self.nums.iter().copied(),
                collector: vec![]
                    .into_collector()
                    .take(self.take_count)
                    .take_while(|&num| num > 0)
                    .fuse(),
                should_break: self.take_count == 0 || !self.nums.iter().all(|&num| num > 0),
                pred: |output, remaining| {
                    let mut iter = self.nums.iter().copied();
                    let expected = iter
                        .by_ref()
                        .take_while(|&num| num > 0)
                        .take(self.take_count);

                    if expected.ne(output) {
                        Err(PredError::IncorrectOutput)
                    } else if iter.ne(remaining) {
                        Err(PredError::IncorrectIterConsumption)
                    } else {
                        Ok(())
                    }
                },
                iter_for_fuse_test: Some([1, 2].into_iter()),
            }
        }
    }
}
