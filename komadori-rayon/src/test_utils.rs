mod coroutine_pool;
mod fuzzy_exec;
mod indexed_split_strategy;
mod par_iter;
mod producer;
mod unindexed_split_strategy;

pub use coroutine_pool::*;
pub use fuzzy_exec::*;
pub use indexed_split_strategy::*;
pub use par_iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator, ParallelIteratorByRef};
pub use producer::*;
pub use unindexed_split_strategy::*;

pub const DEFAULT_MAX_DEPTH: usize = 4;

#[expect(unused_imports)]
pub mod prelude {
    pub use std::ops::ControlFlow::{self, Break, Continue};

    pub use super::{is_subsequence, state_is_irrelevant};
    pub(crate) use super::{par_collector_test, unindexed_par_collector_test};
    pub use proptest::collection::vec as propvec;
    pub use proptest::option::of as prop_opt;
    pub use proptest::prelude::*;
    pub use proptest::test_runner::TestCaseResult;

    pub use crate::{
        collector::{
            IntoParallelCollectorBase, ParallelCollectorBase, ParallelCollectorByMut, ParallelCollectorByRef,
            UnindexedParallelCollectorBase,
        },
        test_utils::{IntoParallelIterator, ParallelIterator, ParallelIteratorByRef},
    };
}

/// Used for testing unindexed parallel collectors.
pub fn is_subsequence<T>(sub: impl IntoIterator<Item = T>, sup: impl IntoIterator<Item = T>) -> bool
where
    T: PartialEq,
{
    let mut iter2 = sup.into_iter();
    sub.into_iter().all(|x1| iter2.any(move |x2| x1 == x2))
}

pub fn state_is_irrelevant<C>() -> impl FnOnce(&C, &()) -> bool {
    |_, _| true
}

macro_rules! par_collector_test {
    (
        $name:ident {
            iter_data: { $(let mut $iter_data_ident:ident = $iter_data_strat:expr;)* },
            other_data: { $(let mut $other_data_ident:ident = $other_data_strat:expr;)* },

            iter: $iter:expr,
            collector: $collector:expr,
            starting_bh: $starting_bh:expr,
            expected_f: |$iter_pat:pat_param, $count_pat:pat_param $(,)?| $expected:expr,
            output_pred: $output_pred:expr,
            state_pred: $state_pred:expr $(,)?
        }
    ) => {
        ::proptest::proptest! {
            #[test]
            fn $name(
                pool in $crate::test_utils::CoroutinePool::prop(),
                ($($iter_data_ident,)* count, split_decision) in ($($iter_data_strat),*)
                    .prop_flat_map(|
                        #[allow(unused_parens, unused_mut)] ($(mut $iter_data_ident),*)
                    | {
                        use ::proptest::strategy::Just;
                        use $crate::test_utils::{
                            IndexedParallelIterator,
                            IndexedSplitStrategy,
                            DEFAULT_MAX_DEPTH,
                        };

                        let count = IndexedParallelIterator::len(&$iter);

                        (
                            $(Just($iter_data_ident),)*
                            Just(count),
                            IndexedSplitStrategy::new(count, DEFAULT_MAX_DEPTH),
                        )
                    }),
                $($other_data_ident in $other_data_strat,)*
            ) {
                $(#[allow(unused_mut)] let mut $iter_data_ident = $iter_data_ident;)*
                let starting_bh = $starting_bh;

                let (expected_output, ending) = (
                    |$iter_pat: ::core::iter::Fuse<_>, $count_pat: usize| $expected
                )(
                    ::core::iter::Iterator::fuse(
                        $crate::test_utils::ParallelIterator::take_iter(&mut $iter)
                    ),
                    count
                );

                {
                    // Clone the pool so that we can reproduct the latter method
                    // directly with the seed instead of running this first.
                    let mut pool = ::core::clone::Clone::clone(&pool);
                    $(#[allow(unused_mut)] let mut $other_data_ident = ::core::clone::Clone::clone(&$other_data_ident);)*

                    $crate::test_utils::check_parts_method(
                        &mut pool,
                        &split_decision,
                        $iter,
                        $collector,
                        &expected_output,
                        $output_pred,
                        starting_bh,
                        &ending,
                        $state_pred,
                    )?;
                }

                {
                    let mut pool = pool;
                    $(#[allow(unused_mut)] let mut $other_data_ident = $other_data_ident;)*
                    $crate::test_utils::check_take_parts_method(
                        &mut pool,
                        &split_decision,
                        $iter,
                        $collector,
                        &expected_output,
                        $output_pred,
                        starting_bh,
                    )?;
                }
            }
        }
    };
}
pub(super) use par_collector_test;

macro_rules! unindexed_par_collector_test {
    (
        $name:ident {
            iter_data: { $(let mut $iter_data_ident:ident = $iter_data_strat:expr;)* },
            other_data: { $(let mut $other_data_ident:ident = $other_data_strat:expr;)* },

            iter: $iter:expr,
            collector: $collector:expr,
            starting_bh: $starting_bh:expr,
            expected_f: |$iter_pat:pat_param, $count_pat:pat_param $(,)?| $expected:expr,
            output_pred: $output_pred:expr,
            state_pred: $state_pred:expr $(,)?
        }
    ) => {
        ::proptest::proptest! {
            #[test]
            fn $name(
                pool in $crate::test_utils::CoroutinePool::prop(),
                ($($iter_data_ident,)* count, split_decision) in ($($iter_data_strat),*)
                    .prop_flat_map(|
                        #[allow(unused_parens, unused_mut)] ($(mut $iter_data_ident),*)
                    | {
                        use ::proptest::strategy::Just;
                        use $crate::test_utils::{
                            ParallelIterator,
                            UnindexedSplitStrategy,
                            DEFAULT_MAX_DEPTH,
                        };

                        let count = ParallelIterator::count($iter);

                        (
                            $(Just($iter_data_ident),)*
                            Just(count),
                            UnindexedSplitStrategy::new(DEFAULT_MAX_DEPTH),
                        )
                    }),
                $($other_data_ident in $other_data_strat,)*
            ) {
                $(#[allow(unused_mut)] let mut $iter_data_ident = $iter_data_ident;)*
                let starting_bh = $starting_bh;

                let (expected_output, ending) = (
                    |$iter_pat: ::core::iter::Fuse<_>, $count_pat: ::core::primitive::usize| $expected
                )(
                    ::core::iter::Iterator::fuse(
                        $crate::test_utils::ParallelIterator::take_iter(&mut $iter)
                    ),
                    count
                );

                {
                    // Clone the pool so that we can reproduct the latter method
                    // directly with the seed instead of running this first.
                    let mut pool = ::core::clone::Clone::clone(&pool);
                    $(#[allow(unused_mut)] let mut $other_data_ident = ::core::clone::Clone::clone(&$other_data_ident);)*

                    $crate::test_utils::check_parts_unindexed_method(
                        &mut pool,
                        &split_decision,
                        $iter,
                        $collector,
                        &expected_output,
                        $output_pred,
                        starting_bh,
                        &ending,
                        $state_pred,
                    )?;
                }

                {
                    let mut pool = pool;
                    $(#[allow(unused_mut)] let mut $other_data_ident = $other_data_ident;)*
                    $crate::test_utils::check_take_parts_unindexed_method(
                        &mut pool,
                        &split_decision,
                        $iter,
                        $collector,
                        &expected_output,
                        $output_pred,
                        starting_bh,
                    )?;
                }
            }
        }
    };
}
pub(crate) use unindexed_par_collector_test;
