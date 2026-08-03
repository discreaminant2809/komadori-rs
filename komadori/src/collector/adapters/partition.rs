use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, Fuse, and_break, break_hint, finish_boxed_impl};
use crate::either::Either;

/// A collector that distributes items between two collectors based on
/// whether an item is "left" or "right."
///
/// This `struct` is created by [`CollectorBase::partition()`]. See its documentation for more.
#[derive(Debug, Clone)]
pub struct Partition<L, R> {
    // `Fuse` is neccessary since we need to assess one's finishing state while assessing another.
    // (See the `collect` implementation)
    left: Fuse<L>,
    right: Fuse<R>,
}

impl<L, R> Partition<L, R>
where
    L: CollectorBase,
    R: CollectorBase,
{
    pub(in crate::collector) fn new(left: L, right: R) -> Self {
        Self {
            left: Fuse::new(left),
            right: Fuse::new(right),
        }
    }
}

impl<L, R> CollectorBase for Partition<L, R>
where
    L: CollectorBase,
    R: CollectorBase,
{
    type Output = (L::Output, R::Output);

    #[inline]
    fn finish(self) -> Self::Output {
        (self.left.finish(), self.right.finish())
    }

    finish_boxed_impl! {}

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        // If one of them, let's say `left`, can still afford,
        // the caller can theoretically feed only `Either::Right`!
        if self.left.max_afford(1) == 0 && self.right.max_afford(1) == 0 {
            0
        } else {
            request
        }
    }
}

impl<L, R, LT, RT> Collector<Either<LT, RT>> for Partition<L, R>
where
    L: Collector<LT>,
    R: Collector<RT>,
{
    #[inline]
    fn collect(&mut self, item: Either<LT, RT>) -> ControlFlow<()> {
        match item {
            Either::Left(item) => and_break(self.left.collect(item), break_hint(&self.right)),
            Either::Right(item) => and_break(break_hint(&self.left), self.right.collect(item)),
        }
    }

    // We can't meaningfully override other methods:
    // - `assume_reserved_collect`: We don't reserve anything
    // - `collect_many` and `collect_then_finish`: We're like tee adapters,
    //   and tee adapters don't override `collect_then_finish`
    //   and only override `collect_many` because they do reserve.
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::either::Either;

    use crate::test_utils::prelude::*;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(
                any::<bool>().prop_flat_map(|is_right| {
                    any::<i32>().prop_map(move |num| {
                        if !is_right {
                            Either::Left(num)
                        } else {
                            Either::Right(num)
                        }
                    })
                }),
                ..=5,
            );
        },
        other_data: {
            let left_n = ..=5_usize;
            let right_n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: vec![]
            .into_collector()
            .take(left_n)
            .partition(vec![].into_collector().take(right_n)),
        expected_f: |mut iter, _| {
            // We truly can't compute the result in an declarative iterator way.

            let (mut left, mut right) = (vec![], vec![]);
            let (mut left_n, mut right_n) = (left_n, right_n);
            while (left_n > 0 || right_n > 0)
                && let Some(num) = iter.next()
            {
                match num {
                    Either::Left(num) if left_n > 0 => {
                        left.push(num);
                        left_n -= 1;
                    }
                    Either::Right(num) if right_n > 0 => {
                        right.push(num);
                        right_n -= 1;
                    }
                    _ => {}
                }
            }

            ((left, right), left_n == 0 && right_n == 0)
        },
        output_pred: PartialEq::eq,
        model: CollectorModel {
            state: State {
                left: left_n,
                right: right_n
            },
            advance_f: |state: &mut State, item: Either<i32, i32>| if item.is_left() {
                state.left = state.left.saturating_sub(1);
            } else {
                state.right = state.right.saturating_sub(1);
            },
            max_afford_f: |state: &State, request| {
                if state.left == 0 && state.right == 0 {
                    0
                } else {
                    request
                }
            }
        },
    });

    struct State {
        left: usize,
        right: usize,
    }
}
