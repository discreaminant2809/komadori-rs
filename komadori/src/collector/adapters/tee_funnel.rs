use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

use super::{DefinePassDown, TeeBase, Teer};

/// A collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`CollectorBase::tee_funnel()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct TeeFunnel<C1, C2> {
    base: TeeBase<C1, C2, FunnelTeer>,
}

impl<C1, C2> TeeFunnel<C1, C2>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            base: TeeBase::new(collector1, collector2, FunnelTeer),
        }
    }
}

impl<C1, C2> CollectorBase for TeeFunnel<C1, C2>
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

impl<T, C1, C2> Collector<T> for TeeFunnel<C1, C2>
where
    C1: for<'a> Collector<&'a mut T>,
    C2: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.base.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.base.collect_many(items)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.base.collect_then_finish(items)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        // SAFETY: `TeeBase` alerady handles the invariants.
        unsafe { self.base.assume_reserved_collect(item) }
    }
}

#[derive(Debug, Clone)]
struct FunnelTeer;

impl<'a, T> DefinePassDown<'a, T> for FunnelTeer {
    type PassDown = &'a mut T;
}

impl<T> Teer<T> for FunnelTeer {
    #[inline]
    fn pass_down<'a>(&mut self, item: &'a mut T) -> &'a mut T {
        item
    }

    // Cannot meaningfully override anything else.
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    proptest! {
        /// Precondition:
        /// - [`crate::collector::Collector::take()`]
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
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || {
                vec![]
                    .into_collector()
                    .copying()
                    .take(first_count)
                    .tee_funnel(vec![].into_collector().take(second_count))
            },
            should_break_pred: |iter| iter.count() >= first_count.max(second_count),
            pred: |iter, output, remaining| {
                let first = iter.clone().take(first_count).collect::<Vec<_>>();
                let second = iter.clone().take(second_count).collect::<Vec<_>>();
                let max_len = first_count.max(second_count);

                if output != (first, second) {
                    Err(PredError::IncorrectOutput)
                } else if iter.skip(max_len).ne(remaining) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
