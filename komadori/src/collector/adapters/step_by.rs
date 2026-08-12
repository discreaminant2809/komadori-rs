use core::{num::NonZero, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, break_hint, finish_boxed_impl};

/// A collector that enters a fixed "cooldown" after accumulating one item.
/// Items that are collected during the cooldown will be ignored.
///
/// This `struct` is created by [`CollectorBase::step_by()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct StepBy<C> {
    collector: C,
    stepper: Stepper,
}

impl<C> StepBy<C> {
    pub(in crate::collector) fn new(collector: C, step: usize) -> Self {
        Self {
            collector,
            stepper: Stepper::new(NonZero::new(step).expect("`step_by()` by zero")),
        }
    }
}

impl<C> CollectorBase for StepBy<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    finish_boxed_impl! {}

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.collector.reserve(self.stepper.true_amount(additional));
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        let (cooldown, step) = (self.stepper.cooldown, self.stepper.step.get());

        let true_request = if request == 0 {
            return 0;
        } else if request <= cooldown {
            1
        } else {
            // If the request amount is between two "collected points,"
            // we round up to the collected point in the right,
            // acting as an extra.
            (request - cooldown - 1).div_ceil(step) + 1
        };

        let max_afford = self.collector.max_afford(true_request);

        if max_afford == 0 {
            0
        } else if max_afford < true_request {
            // `max_afford` can be `ceil((request - cooldown - 1) / n)` at max (`true_request - 1`)
            // in this branch.
            // We have:
            // (ceil((request - cooldown - 1) / n) - 1) * n + cooldown + 1
            // < ((request - cooldown - 1) / n + 1 - 1) * n + cooldown + 1
            // = request
            // So we don't have any overflow risks, and no underflow risks either
            // since we only enter this branch with a positive `max_afford`.
            (max_afford - 1) * step + cooldown + 1
        } else {
            // It can afford all `request` items if it can even accept
            // the right collected point.
            request
        }
    }
}

impl<C, T> Collector<T> for StepBy<C>
where
    C: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        if self.stepper.collect_now() {
            self.collector.collect(item)
        } else {
            break_hint(&self.collector)
        }
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        if self.stepper.collect_now() {
            unsafe {
                // SAFETY: The caller has reserved
                self.collector.assume_reserved_collect(item)
            }
        } else {
            break_hint(&self.collector)
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        if self.stepper.step.get() == 1 {
            return self.collector.collect_many(items);
        }

        break_hint(self)?;

        let mut items = items.into_iter();

        // Discarding items under the cooldown first
        let cooldown = &mut self.stepper.cooldown;
        while *cooldown > 0 {
            if items.next().is_none() {
                return ControlFlow::Continue(());
            }

            *cooldown -= 1;
            break_hint(&self.collector)?;
        }

        let sh = crate::iter::SizeHint::from_iter(&items);
        let reservation = self.stepper.true_amount(sh.lower());
        self.collector.reserve(reservation);

        // We manually unroll the iterator's loop from now on.

        for _ in 0..reservation {
            let Some(item) = items.next() else {
                return ControlFlow::Continue(());
            };

            self.stepper.collect_now();
            unsafe {
                self.collector.assume_reserved_collect(item)?;
            }

            let cooldown = &mut self.stepper.cooldown;
            while *cooldown > 0 {
                if items.next().is_none() {
                    return ControlFlow::Continue(());
                }

                *cooldown -= 1;
                break_hint(&self.collector)?;
            }
        }

        loop {
            let Some(item) = items.next() else {
                return ControlFlow::Continue(());
            };

            self.stepper.collect_now();
            self.collector.collect(item)?;

            let cooldown = &mut self.stepper.cooldown;
            while *cooldown > 0 {
                if items.next().is_none() {
                    return ControlFlow::Continue(());
                }

                *cooldown -= 1;
                break_hint(&self.collector)?;
            }
        }
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let (cooldown, step) = (self.stepper.cooldown, self.stepper.step.get());

        if step == 1 {
            return self.collector.collect_then_finish(items);
        }

        if self.collector.max_afford(1) == 0 {
            return self.collector.finish();
        }

        let mut items = items
            .into_iter()
            // The iterator might be exhausted after `try_for_each`.
            .fuse();

        // Discarding items under the cooldown first
        if items
            .by_ref()
            .take(cooldown)
            .try_for_each(|_| break_hint(&self.collector))
            .is_break()
        {
            return self.collector.finish();
        }

        let sh = crate::iter::SizeHint::from_iter(&items);
        let reservation = self.stepper.true_amount(sh.lower());
        self.collector.reserve(reservation);

        // We manually unroll the iterator's loop from now on.

        for _ in 0..reservation {
            let Some(item) = items.next() else {
                return self.collector.finish();
            };

            unsafe {
                if self.collector.assume_reserved_collect(item).is_break() {
                    return self.collector.finish();
                }
            }

            if items
                .by_ref()
                .take(step - 1)
                .try_for_each(|_| break_hint(&self.collector))
                .is_break()
            {
                return self.collector.finish();
            }
        }

        loop {
            let Some(item) = items.next() else {
                return self.collector.finish();
            };

            if self.collector.collect(item).is_break() {
                return self.collector.finish();
            }

            if items
                .by_ref()
                .take(step - 1)
                .try_for_each(|_| break_hint(&self.collector))
                .is_break()
            {
                return self.collector.finish();
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Stepper {
    step: NonZero<usize>,
    cooldown: usize,
}

impl Stepper {
    #[inline]
    fn new(step: NonZero<usize>) -> Self {
        Self { step, cooldown: 0 }
    }

    #[inline]
    fn collect_now(&mut self) -> bool {
        if self.cooldown == 0 {
            self.cooldown = self.step.get() - 1;
            true
        } else {
            self.cooldown -= 1;
            false
        }
    }

    #[inline]
    fn true_amount(&self, amount: usize) -> usize {
        amount
            .saturating_sub(self.cooldown)
            .div_ceil(self.step.get())
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
            let step = 1..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![].into_collector().take(n).step_by(step),
        expected_f: |iter, count| {
            // To help the type inference.
            let count: usize = count;
            let res: Vec<_> = iter.step_by(step).take(n).collect();
            (res, count.div_ceil(step) >= n)
        },
        output_pred: PartialEq::eq,
        model: CollectorModel {
            state: State { n, cooldown: 0 },
            advance_f: |state: &mut State, _| state.update(step),
            max_afford_f: |state: &State, request| {
                let mut state = state.clone();

                let mut request_range = 0..request;
                while state.n > 0 && request_range.next().is_some() {
                    state.update(step);
                }

                request_range.start
            },
        },
    });

    #[derive(Clone)]
    struct State {
        n: usize,
        cooldown: usize,
    }

    impl State {
        fn update(&mut self, step: usize) {
            if self.cooldown == 0 {
                self.n = self.n.saturating_sub(1);
                self.cooldown = step - 1;
            } else {
                self.cooldown -= 1;
            }
        }
    }
}
