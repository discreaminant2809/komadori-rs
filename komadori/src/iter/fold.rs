use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector, finish_boxed_impl};

/// A collector that accumulates items using a function.
///
/// This collector corresponds to [`Iterator::fold()`], except that
/// the accumulated value is mutated in place.
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::Fold};
///
/// let mut collector = Fold::new(0, |sum, num| *sum += num);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert_eq!(collector.finish(), 6);
/// ```
#[derive(Clone)]
pub struct Fold<A, F> {
    accum: A,
    f: F,
}

impl<A, F> Fold<A, F> {
    /// Creates a new instance of this collector with an initial value and an accumulator.
    pub const fn new<T>(init: A, f: F) -> Self
    where
        F: FnMut(&mut A, T),
    {
        assert_collector::<_, T>(Self { accum: init, f })
    }
}

impl<A, F> CollectorBase for Fold<A, F> {
    type Output = A;

    #[inline]
    fn finish(self) -> Self::Output {
        self.accum
    }

    finish_boxed_impl!();
}

impl<A, T, F> Collector<T> for Fold<A, F>
where
    F: FnMut(&mut A, T),
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        (self.f)(&mut self.accum, item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        items
            .into_iter()
            .for_each(|item| (self.f)(&mut self.accum, item));
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        items.into_iter().for_each({
            let accum = &mut self.accum;
            move |item| (self.f)(accum, item)
        });

        self.accum
    }
}

impl<A: Debug, F> Debug for Fold<A, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fold")
            .field("accum", &self.accum)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    use super::*;

    collector_test!(collector {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {},
        iter: nums.iter().copied(),
        collector: Fold::new(KADANE_INIT, |(sum, max_sum), num| {
            kadane_fold(sum, max_sum, num)
        }),
        expected_f: |iter, _| (
            iter.fold(KADANE_INIT, |(mut sum, mut max_sum), num| {
                kadane_fold(&mut sum, &mut max_sum, num);
                (sum, max_sum)
            }),
            false
        ),
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    fn kadane_fold(sum: &mut i32, max_sum: &mut Option<i32>, num: i32) {
        *sum = num;
        *max_sum = (*max_sum).max(Some(*sum));
        *sum = (*sum).max(0);
    }

    const KADANE_INIT: (i32, Option<i32>) = (0, None);
}
