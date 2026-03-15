use std::ops::ControlFlow;

use itertools::Itertools;

use crate::collector::{Collector, CollectorBase};

/// A collector that determines whether all collected items are equal to each other.
///
/// The [`Output`](CollectorBase::Output) is `true` if no items were collected.
///
/// This corresponds to [`Itertools::all_equal()`].
#[derive(Debug, Clone)]
pub struct AllEqual<T> {
    state: State<T>,
}

#[derive(Debug, Clone)]
enum State<T> {
    // This state is deliberately here so that it may have
    // a tag of 0, matching `false`.
    NotEqual,
    StillEqual { prev: Option<T> },
}

impl<T> AllEqual<T>
where
    T: PartialEq,
{
    /// Creates a new instance of this collector.
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: State::StillEqual { prev: None },
        }
    }
}

impl<T> CollectorBase for AllEqual<T> {
    type Output = bool;

    fn finish(self) -> Self::Output {
        matches!(self.state, State::StillEqual { .. })
    }

    fn break_hint(&self) -> ControlFlow<()> {
        if matches!(self.state, State::StillEqual { .. }) {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}

impl<T> Collector<T> for AllEqual<T>
where
    T: PartialEq,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self.state {
            State::StillEqual {
                prev: ref mut prev @ None,
            } => {
                *prev = Some(item);
                ControlFlow::Continue(())
            }
            State::StillEqual {
                prev: Some(ref prev),
            } if *prev == item => ControlFlow::Continue(()),
            State::StillEqual { .. } => {
                self.state = State::NotEqual;
                ControlFlow::Break(())
            }
            State::NotEqual => ControlFlow::Break(()),
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match &mut self.state {
            State::StillEqual { prev: prev @ None } => {
                let mut items = items.into_iter();
                let Some(first_item) = items.next() else {
                    return ControlFlow::Continue(());
                };

                let prev = prev.insert(first_item);

                if items.all(move |item| *prev == item) {
                    ControlFlow::Continue(())
                } else {
                    self.state = State::NotEqual;
                    ControlFlow::Break(())
                }
            }
            State::StillEqual { prev: Some(prev) } => {
                if items.into_iter().all(move |item| *prev == item) {
                    ControlFlow::Continue(())
                } else {
                    self.state = State::NotEqual;
                    ControlFlow::Break(())
                }
            }
            State::NotEqual => ControlFlow::Break(()),
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.state {
            State::NotEqual => false,
            State::StillEqual { prev: None } => items.into_iter().all_equal(),
            State::StillEqual { prev: Some(prev) } => {
                items.into_iter().all(move |item| prev == item)
            }
        }
    }
}

impl<T> Default for AllEqual<T>
where
    T: PartialEq,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::option::of as prop_option;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use itertools::Itertools;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::*;

    proptest! {
        #[test]
        fn all_collect_methods(
            nums in propvec(prop_oneof![Just(1), Just(2)], ..=3),
            first_num in prop_option(prop_oneof![Just(1), Just(2)]),
        ) {
            all_collect_methods_impl(nums, first_num)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>, first_num: Option<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: move || {
                // We test the `new` method also.
                let mut collector = AllEqual::new();
                if let Some(num) = first_num {
                    let _ = collector.collect(num);
                }
                collector
            },
            should_break_pred: |iter| !iter.chain(first_num).all_equal(),
            pred: |mut iter, output, remaining| {
                if first_num.into_iter().chain(&mut iter).all_equal() != output {
                    Err(PredError::IncorrectOutput)
                } else if remaining.ne(iter) {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
