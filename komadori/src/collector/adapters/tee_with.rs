use std::{fmt::Debug, iter, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, Fuse};

/// A collector that lets both collectors collect the same item.
///
/// This `struct` is created by [`CollectorBase::tee_with()`].
/// See its documentation for more.
#[derive(Clone)]
pub struct TeeWith<C1, C2, F> {
    collector1: Fuse<C1>,
    collector2: Fuse<C2>,
    f: F,
}

impl<C1, C2, F> TeeWith<C1, C2, F>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    pub(in crate::collector) fn new(collector1: C1, collector2: C2, f: F) -> Self {
        Self {
            collector1: collector1.fuse(),
            collector2: collector2.fuse(),
            f,
        }
    }
}

impl<C1, C2, F> CollectorBase for TeeWith<C1, C2, F>
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
        if self.collector1.break_hint().is_break() && self.collector2.break_hint().is_break() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

// The implementation here is basically similar to `tee_clone`.
impl<T, C1, C2, F, U> Collector<T> for TeeWith<C1, C2, F>
where
    C1: Collector<U> + Collector<T>,
    C2: Collector<T>,
    F: FnMut(&mut T) -> U,
{
    fn collect(&mut self, mut item: T) -> ControlFlow<()> {
        if self.collector1.break_hint().is_break() {
            self.collector2.collect(item)
        } else if self.collector2.break_hint().is_break() {
            self.collector1.collect(item)
        } else {
            let (item1, item2) = ((self.f)(&mut item), item);
            match (
                self.collector1.collect(item1),
                self.collector2.collect(item2),
            ) {
                (ControlFlow::Break(_), ControlFlow::Break(_)) => ControlFlow::Break(()),
                _ => ControlFlow::Continue(()),
            }
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.break_hint()?;

        let mut items = items.into_iter();

        match items.try_for_each(|mut item| {
            // We don't need to check like the `collect` implementation.
            // `self.break_hint()?` has already handled it,
            // and we trust that both underlying collectors
            // return `Break` as soon as it can't afford more items.
            if self.collector1.collect((self.f)(&mut item)).is_break() {
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

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        if self.break_hint().is_break() {
            return self.finish();
        }

        let mut items = items.into_iter();

        match items.try_for_each(|mut item| {
            // We don't need to check like the `collect` implementation.
            // `self.break_hint()?` has already handled it,
            // and we trust that both underlying collectors
            // return `Break` as soon as it can't afford more items.
            if self.collector1.collect((self.f)(&mut item)).is_break() {
                ControlFlow::Break(Which::First(item))
            } else {
                self.collector2.collect(item).map_break(|_| Which::Second)
            }
        }) {
            // If one of the collectors has stopped, we can avoid cloning
            // for the rest of the items!
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

impl<C1, C2, F> Debug for TeeWith<C1, C2, F>
where
    C1: Debug,
    C2: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeeWith")
            .field("collector1", &self.collector1)
            .field("collector2", &self.collector2)
            .field("f", &std::any::type_name::<F>())
            .finish()
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
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    proptest! {
        /// Precondition:
        /// - [`crate::collector::CollectorBase::take()`]
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
                    .take(first_count)
                    .tee_with(|&mut num| num, vec![].into_collector().take(second_count))
            },
            should_break_pred: |iter| iter.count() >= first_count.max(second_count),
            pred: |iter, (output1, output2), remaining| {
                let max_len = first_count.max(second_count);

                if output1.into_iter().ne(iter.clone().take(first_count))
                    || output2.into_iter().ne(iter.clone().take(second_count))
                {
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
