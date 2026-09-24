use std::ops::ControlFlow;

use crate::{
    collector::{
        Fuse, IntoUnindexedParallelCollectorBase, ParallelCollectorBase,
        UnindexedParallelCollectorBase, assert_unindexed_par_collector_base,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf, and_break,
        },
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that distributes items between two parallel collectors
/// based on whether an item is "left" or "right."
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::partition()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct Partition<L, R> {
    left: Fuse<L>,
    right: Fuse<R>,
}

/// Creates a new instance of [`Partition`] from two types
/// that are convertible into parallel collectors.
///
/// Use this when the two parallel collectors are nearly equally long,
/// or you can just use this generally to express the intent more readably.
///
/// See [`UnindexedParallelCollectorBase::partition()`] for more.
#[inline]
pub fn par_partition<L, R>(left: L, right: R) -> Partition<L::IntoParCollector, R::IntoParCollector>
where
    L: IntoUnindexedParallelCollectorBase,
    R: IntoUnindexedParallelCollectorBase,
{
    assert_unindexed_par_collector_base(Partition::new(
        left.into_par_collector(),
        right.into_par_collector(),
    ))
}

impl<L, R> Partition<L, R>
where
    L: ParallelCollectorBase,
    R: ParallelCollectorBase,
{
    pub(in crate::collector) fn new(left: L, right: R) -> Self {
        Self {
            left: left.fuse(),
            right: right.fuse(),
        }
    }
}

impl<'a, L, R> DefineSerial<'a> for Partition<L, R>
where
    L: DefineUnindexedSerial<'a>,
    R: DefineUnindexedSerial<'a>,
{
    type Serial = unique::Serial<
        'a,
        Self,
        consumer::Serial<UnindexedSerialOf<'a, Fuse<L>>, UnindexedSerialOf<'a, Fuse<R>>>,
    >;
}

impl<'a, L, R> DefineUnindexedSerial<'a> for Partition<L, R>
where
    L: DefineUnindexedSerial<'a>,
    R: DefineUnindexedSerial<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<
        'a,
        Self,
        consumer::Serial<UnindexedSerialOf<'a, Fuse<L>>, UnindexedSerialOf<'a, Fuse<R>>>,
    >;
}

impl<L, R> ParallelCollectorBase for Partition<L, R>
where
    L: UnindexedParallelCollectorBase,
    R: UnindexedParallelCollectorBase,
{
    type Output = (L::Output, R::Output);

    #[inline]
    fn finish(self) -> Self::Output {
        (self.left.finish(), self.right.finish())
    }

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

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let (left_consumer, left_commit) = self.left.unindexed_parts();
        let (right_consumer, right_commit) = self.right.unindexed_parts();

        unique::uniquify((
            consumer::consumer(left_consumer, right_consumer),
            |(left_output, right_output)| {
                and_break(left_commit(left_output), right_commit(right_output))
            },
        ))
    }

    fn take_parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        let (left_consumer, left_commit) = self.left.take_unindexed_parts();
        let (right_consumer, right_commit) = self.right.take_unindexed_parts();

        unique::take_uniquify((
            consumer::consumer(left_consumer, right_consumer),
            |(left_output, right_output)| {
                left_commit(left_output);
                right_commit(right_output);
            },
        ))
    }
}

impl<L, R> UnindexedParallelCollectorBase for Partition<L, R>
where
    L: UnindexedParallelCollectorBase,
    R: UnindexedParallelCollectorBase,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let (left_consumer, left_commit) = self.left.unindexed_parts();
        let (right_consumer, right_commit) = self.right.unindexed_parts();

        unique_unindexed::uniquify((
            consumer::consumer(left_consumer, right_consumer),
            |(left_output, right_output)| {
                and_break(left_commit(left_output), right_commit(right_output))
            },
        ))
    }

    fn take_unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        let (left_consumer, left_commit) = self.left.take_unindexed_parts();
        let (right_consumer, right_commit) = self.right.take_unindexed_parts();

        unique_unindexed::take_uniquify((
            consumer::consumer(left_consumer, right_consumer),
            |(left_output, right_output)| {
                left_commit(left_output);
                right_commit(right_output);
            },
        ))
    }
}

#[expect(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use either::Either;

    use crate::collector::plumbing::{
        BasicUnindexedConsumer, Collector, CollectorBase, Combiner, UnindexedConsumer, and_break,
        break_hint, finish_boxed_impl,
    };

    // We don't use the `komadori` one to avoid one layer of `Fuse`.
    pub struct Serial<L, R> {
        left: L,
        right: R,
    }

    pub fn consumer<L, R>(
        left: L,
        right: R,
    ) -> impl UnindexedConsumer<
        Output = (L::Output, R::Output),
        IntoCollector = Serial<L::IntoCollector, R::IntoCollector>,
    >
    where
        L: UnindexedConsumer,
        R: UnindexedConsumer,
    {
        BasicUnindexedConsumer {
            state: (left, right),
            split_f: |(left, right)| (left.split_off_left(), right.split_off_left()),
            combiner_f: |(left, right)| {
                let left = left.to_combiner();
                let right = right.to_combiner();
                |(ll, lr): &mut _, (rl, rr)| {
                    left.combine(ll, rl);
                    right.combine(lr, rr);
                }
            },
            ma_f: |(left, right), request| {
                // If one of them, let's say `left`, can still afford,
                // the caller can theoretically feed only `Either::Right`!
                if left.max_afford(1) == 0 && right.max_afford(1) == 0 {
                    0
                } else {
                    request
                }
            },
            collector_f: |(left, right)| Serial {
                left: left.into_collector(),
                right: right.into_collector(),
            },
        }
    }

    impl<L, R> CollectorBase for Serial<L, R>
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

    impl<L, R, LT, RT> Collector<Either<LT, RT>> for Serial<L, R>
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
}

#[cfg(test)]
mod proptests {
    use either::Either;

    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
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
            let mut left_n = ..=5_usize;
            let mut right_n = ..=5_usize;
        },
        iter: nums.par_iter().cloned(),
        collector: super::par_partition(
            vec![].into_par_collector().take(left_n),
            vec![].into_par_collector().take(right_n),
        ),
        starting_ma_f: |request| if left_n > 0 || right_n > 0 {
            request
        } else {
            0
        },
        expected_f: |iter, _, request| {
            // We truly can't compute the result in an declarative iterator way.

            let (mut left, mut right) = (vec![], vec![]);
            let (mut left_count, mut right_count) = (0_usize, 0_usize);
            for num in iter {
                match num {
                    Either::Left(num) => {
                        left.push(num);
                        left_count += 1;
                    }
                    Either::Right(num) => {
                        right.push(num);
                        right_count += 1;
                    }
                }
            }

            (
                (left, right),
                if left_count < left_n || right_count < right_n {
                    Continue(((), request))
                } else {
                    Break(())
                },
            )
        },
        output_pred: |(actual_left, actual_right), (expected_left, expected_right)| {
            same_min_len_and_sub_seq(actual_left, expected_left, left_n)
                && same_min_len_and_sub_seq(actual_right, expected_right, right_n)
        },
        state_pred: state_is_irrelevant(),
    });

    unindexed_par_collector_test!(unindexed {
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
            let mut left_n = ..=5_usize;
            let mut right_n = ..=5_usize;
        },
        iter: nums.par_iter().cloned(),
        collector: super::par_partition(
            vec![].into_par_collector().take(left_n),
            vec![].into_par_collector().take(right_n),
        ),
        starting_ma_f: |request| if left_n > 0 || right_n > 0 {
            request
        } else {
            0
        },
        expected_f: |iter, _, request| {
            // We truly can't compute the result in an declarative iterator way.

            let (mut left, mut right) = (vec![], vec![]);
            let (mut left_count, mut right_count) = (0_usize, 0_usize);
            for num in iter {
                match num {
                    Either::Left(num) => {
                        left.push(num);
                        left_count += 1;
                    }
                    Either::Right(num) => {
                        right.push(num);
                        right_count += 1;
                    }
                }
            }

            (
                (left, right),
                if left_count < left_n || right_count < right_n {
                    Continue(((), request))
                } else {
                    Break(())
                },
            )
        },
        output_pred: |(actual_left, actual_right), (expected_left, expected_right)| {
            same_min_len_and_sub_seq(actual_left, expected_left, left_n)
                && same_min_len_and_sub_seq(actual_right, expected_right, right_n)
        },
        state_pred: state_is_irrelevant(),
    });

    fn same_min_len_and_sub_seq(actual: &[i32], expected: &[i32], n: usize) -> bool {
        actual.len() == expected.len().min(n) && is_subsequence(actual, expected)
    }
}
