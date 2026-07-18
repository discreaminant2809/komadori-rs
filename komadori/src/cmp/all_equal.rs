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

    fn max_afford(&self, request: usize) -> usize {
        if matches!(self.state, State::StillEqual { .. }) {
            request
        } else {
            0
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
    use itertools::Itertools;

    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(collector {
        iter_data: propvec(prop_oneof![Just(1), Just(2)], ..=3),
        collector_data: any::<()>(),
        iter_f: Vec::clone,
        collector_f: |_: &_| AllEqual::new(),
        output_f: |mut iter, _| (&mut iter).all_equal(),
        model_f: |_| BasicCollectorModel {
            state: ModelState {
                prev: None,
                all_equal: true,
            },
            advance_f: |state: &mut ModelState, num| {
                match state.prev {
                    Some(prev) if prev != num => state.all_equal = false,
                    _ => state.prev = Some(num),
                }
            },
            max_afford_f: |state, request| if state.all_equal { request } else { 0 },
            cf_f: |state| if state.all_equal {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            },
            output_and_pred_f: |ModelState { all_equal, .. }| (all_equal, PartialEq::eq)
        },
    });

    struct ModelState {
        prev: Option<i32>,
        all_equal: bool,
    }
}
