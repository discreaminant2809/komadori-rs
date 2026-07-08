use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, Fuse, break_hint};

/// A collector that feeds the first collector until it stop accumulating,
/// then feeds the second collector.
///
/// This `struct` is created by [`CollectorBase::chain()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Chain<C1, C2> {
    collector1: Fuse<C1>,
    collector2: C2,
    // It's to guard against the case when the first collector lies.
    // See the test in `miri_tests` for why.
    reserve_for_2: usize,
}

impl<C1, C2> Chain<C1, C2>
where
    C1: CollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2) -> Self {
        Self {
            collector1: collector1.fuse(),
            collector2,
            reserve_for_2: 0,
        }
    }
}

impl<C1, C2> CollectorBase for Chain<C1, C2>
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
    fn reserve(&mut self, additional: usize) {
        self.collector1.reserve(additional);

        let max_afford1 = self.collector1.max_afford(additional);
        // We can't accept wrapping around because it's safe,
        // but the code in `assume_reserved_collect()` isn't!
        // FIXME: maybe we have a way, like `TrustedMaxAfford`?
        assert!(
            additional >= max_afford1,
            "`max_afford()` of the first collector is implemented incorrectly"
        );
        self.reserve_for_2 = additional - max_afford1;

        self.collector2.reserve(self.reserve_for_2);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        let max_afford1 = self.collector1.max_afford(request);
        max_afford1 + self.collector2.max_afford(request - max_afford1)
    }
}

impl<T, C1, C2> Collector<T> for Chain<C1, C2>
where
    C1: Collector<T>,
    C2: Collector<T>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if self.collector1.max_afford(1) == 0 {
            self.reserve_for_2 = self.reserve_for_2.saturating_sub(1);
            self.collector2.collect(item)
        } else if self.collector1.collect(item).is_continue() {
            ControlFlow::Continue(())
        } else {
            break_hint(&self.collector2)
        }
    }

    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        if self.collector1.max_afford(1) == 0 {
            // Safety guard in case the first collector lies in `reserve()`.
            if self.reserve_for_2 > 0 {
                self.reserve_for_2 -= 1;
                // SAFETY: The caller has reserved one item for the second collector.
                unsafe { self.collector2.assume_reserved_collect(item) }
            } else {
                self.collector2.collect(item)
            }
        } else if unsafe {
            // SAFETY: The caller has reserved one item for the first collector.
            // We don't need another number to guard like the second collector,
            // because if it lies in `reserve()` it's its fault for violating
            // an API contract that `reserve()` and `max_afford()` must be implemented
            // correctly if `assume_reserved_collect()` is overriden.
            self.collector1.assume_reserved_collect(item).is_continue()
        } {
            ControlFlow::Continue(())
        } else {
            break_hint(&self.collector2)
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // Per the Reserve API's contract, we can reset this to 0.
        self.reserve_for_2 = 0;

        let mut items = items.into_iter();

        if self.collector1.collect_many(&mut items).is_break() {
            self.collector2.collect_many(items)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let mut items = items.into_iter();

        // Be careful! The first collector may have exhausted the iterator,
        // and collect_then_finish can't tell that!
        // Yes, `fuse()` helps, but this way removes the need
        // of iterator adapters!
        if self.collector1.collect_many(&mut items).is_break() {
            (
                self.collector1.finish(),
                self.collector2.collect_then_finish(items),
            )
        } else {
            (self.collector1.finish(), self.collector2.finish())
        }
    }
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
            nums in propvec(any::<i32>(), ..=7),
            first_count in 0..=3_usize,
            second_count in 0..=3_usize,
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
                    .take(first_count)
                    .chain(vec![].into_collector().take(second_count))
            },
            should_break_pred: |iter| iter.count() >= first_count + second_count,
            pred: |mut iter, output, remaining| {
                let first = iter.by_ref().take(first_count).collect::<Vec<_>>();
                let second = iter.by_ref().take(second_count).collect::<Vec<_>>();

                if output != (first, second) {
                    Err(PredError::IncorrectOutput)
                } else if iter.ne(remaining) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}

#[cfg(test)]
mod miri_tests {
    #[test]
    fn lying_first_collector() {
        use crate::prelude::*;
        use std::{cell::Cell, ops::ControlFlow, rc::Rc};

        struct Malicious(Rc<Cell<usize>>);

        impl CollectorBase for Malicious {
            type Output = ();

            fn finish(self) -> Self::Output {}

            fn max_afford(&self, request: usize) -> usize {
                self.0.get().min(request)
            }
        }

        impl<T> Collector<T> for Malicious {
            fn collect(&mut self, _item: T) -> ControlFlow<()> {
                self.0.update(|count| count.saturating_sub(1));
                if self.0.get() > 0 {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            }
        }

        // As we can see for now, our entire implementation of a collector
        // is entirely safe!

        let count = Rc::new(Cell::new(3));
        let mut collector = Malicious(Rc::clone(&count)).chain(Vec::with_capacity(3));

        // By now, the Vec reserves 3 items.
        // Actually, we did `with_capacity(3)` earlier so that the capacity
        // still stays 3, which helps miri catch over-pushing.
        collector.reserve(6);

        // But then, the first collectors decide to lie.
        count.set(0);

        for i in 0..6 {
            // Now every collector is forwarded to the second collector.
            unsafe {
                // SAFETY: We've reserved for 6 items.
                // That should make sense with the API's contract.
                // However, without `reserve_for_2`, we'd accidentally feed
                // more than 3 items to the Vec which only reserves for 3 items.
                // 3 to 5 should survive miri!
                assert!(collector.assume_reserved_collect(i).is_continue());
            }
        }

        assert_eq!(collector.finish(), ((), vec![0, 1, 2, 3, 4, 5]));
    }
}
