use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{
    Collector, CollectorBase, IntoCollector, IntoCollectorBase, finish_boxed_impl,
};

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
            state: if collector.max_afford(1) > 0 {
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

    finish_boxed_impl!();

    fn reserve(&mut self, additional: usize) {
        match &mut self.state {
            State::Invalid => invalid_state(),
            State::First { collector, .. } => collector.reserve(additional),
            State::Second { collector } => collector.reserve(additional),
        }
    }

    fn max_afford(&self, request: usize) -> usize {
        match &self.state {
            State::Invalid => invalid_state(),
            // We still have the second collector. Be careful!
            // Even if the first returns less than `request`,
            // we can't know how many the second can afford.
            State::First { .. } => request,
            State::Second { collector } => collector.max_afford(request),
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
            State::First { collector, f } if collector.max_afford(1) == 0 => {
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
                let cf = if collector.max_afford(1) > 0 {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                };
                self.state = State::Second { collector };
                cf
            }
            State::Second { collector } => collector.collect(item),
        }
    }

    // TODO: Override `assume_reserved_collect()` later when this adapter is stabilized.

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
    use crate::test_utils::prelude::*;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=6);
        },
        other_data: {
            let first_n = ..=3_usize;
            let second_n = ..=3_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![]
            .into_collector()
            .take(first_n)
            .then(|v| v.into_collector().take(second_n)),
        expected_f: |iter, count| {
            (
                iter.take(first_n + second_n).collect::<Vec<_>>(),
                count >= first_n + second_n,
            )
        },
        output_pred: PartialEq::eq,
        model: CollectorModel {
            state: Counts {
                first: first_n,
                second: second_n
            },
            advance_f: |counts: &mut Counts, _| {
                if counts.first > 0 {
                    counts.first -= 1;
                } else {
                    counts.second = counts.second.saturating_sub(1);
                }
            },
            max_afford_f: |counts: &Counts, request| {
                if counts.first > 0 {
                    request
                } else {
                    counts.second.min(request)
                }
            },
        },
    });

    struct Counts {
        first: usize,
        second: usize,
    }
}
