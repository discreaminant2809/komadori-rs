use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, IntoCollector, IntoCollectorBase};

/// A collector that feeds every item in the first collector until it stops accumulating,
/// then creates a second collector from the output of the first collector
/// and continues feeding the rest of the items into the second one.
///
/// This `struct` is created by [`CollectorBase::then()`]. See its documentation for more.
pub struct Then<C1, C2, F> {
    state: State<C1, C2, F>,
}

enum State<C1, C2, F> {
    Invalid,
    First { collector: C1, f: F },
    Second { collector: C2 },
}

impl<C1, C2, F> Then<C1, C2::IntoCollector, F>
where
    C1: CollectorBase,
    C2: IntoCollectorBase,
    F: FnOnce(C1::Output) -> C2,
{
    pub(in crate::collector) fn new(collector: C1, f: F) -> Self {
        Self {
            state: if collector.break_hint().is_continue() {
                State::First { collector, f }
            } else {
                State::Second {
                    collector: f(collector.finish()).into_collector(),
                }
            },
        }
    }
}

impl<C1, C2, F> State<C1, C2, F> {
    fn take_first_state(&mut self) -> (C1, F) {
        let State::First { collector, f } = std::mem::replace(self, Self::Invalid) else {
            unreachable!("must be First");
        };
        (collector, f)
    }
}

impl<C1, C2, F> Debug for State<C1, C2, F>
where
    C1: Debug,
    C2: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => invalid_state(),
            Self::First { collector, .. } => f
                .debug_struct("First")
                .field("collector", collector)
                .field("f", &std::any::type_name::<F>())
                .finish(),
            Self::Second { collector } => f
                .debug_struct("Second")
                .field("collector", collector)
                .finish(),
        }
    }
}

impl<C1, C2, F> Debug for Then<C1, C2, F>
where
    C1: Debug,
    C2: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Then").field("state", &self.state).finish()
    }
}

impl<C1, C2, F> CollectorBase for Then<C1, C2::IntoCollector, F>
where
    C1: CollectorBase,
    C2: IntoCollectorBase<Output = C1::Output>,
    F: FnOnce(C1::Output) -> C2,
{
    type Output = C1::Output;

    fn finish(self) -> Self::Output {
        match self.state {
            State::Invalid => invalid_state(),
            State::First { collector, .. } => collector.finish(),
            State::Second { collector } => collector.finish(),
        }
    }

    fn break_hint(&self) -> ControlFlow<()> {
        match &self.state {
            State::Invalid => invalid_state(),
            // We still have the second collector. Be careful!
            State::First { .. } => ControlFlow::Continue(()),
            State::Second { collector } => collector.break_hint(),
        }
    }
}

impl<C1, C2, F, T> Collector<T> for Then<C1, C2::IntoCollector, F>
where
    C1: Collector<T>,
    C2: IntoCollector<T, Output = C1::Output>,
    F: FnOnce(C1::Output) -> C2,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match &mut self.state {
            State::Invalid => invalid_state(),
            State::First { collector, f } if collector.break_hint().is_break() => {
                let (collector, f) = self.state.take_first_state();
                let mut collector = f(collector.finish()).into_collector();
                let cf = collector.collect(item);
                self.state = State::Second { collector };
                cf
            }
            State::First { collector, .. } => {
                if collector.collect(item).is_continue() {
                    return ControlFlow::Continue(());
                }

                let (collector, f) = self.state.take_first_state();
                let collector = f(collector.finish()).into_collector();
                let cf = collector.break_hint();
                self.state = State::Second { collector };
                cf
            }
            State::Second { collector } => collector.collect(item),
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        let mut items = items.into_iter();

        match &mut self.state {
            State::Invalid => invalid_state(),
            State::First { collector, .. } => {
                if collector.collect_many(&mut items).is_continue() {
                    return ControlFlow::Continue(());
                }

                let (collector, f) = self.state.take_first_state();
                let mut collector = f(collector.finish()).into_collector();
                let cf = collector.collect_many(items);
                self.state = State::Second { collector };
                cf
            }
            State::Second { collector } => collector.collect_many(items),
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let mut items = items.into_iter();

        match self.state {
            State::Invalid => invalid_state(),
            State::First { mut collector, f } => {
                if collector.collect_many(&mut items).is_continue() {
                    collector.finish()
                } else {
                    f(collector.finish())
                        .into_collector()
                        .collect_then_finish(items)
                }
            }
            State::Second { collector } => collector.collect_then_finish(items),
        }
    }
}

fn invalid_state() -> ! {
    panic!("invalid state for `Then`")
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
                    .then(|v| v.into_collector().take(second_count))
            },
            should_break_pred: |iter| iter.count() >= first_count + second_count,
            pred: |mut iter, output, remaining| {
                if iter.by_ref().take(first_count + second_count).ne(output) {
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
