mod collector_tester;
mod fuzzy_exec;
mod fuzzy_exec_seq;

pub use collector_tester::*;
pub use fuzzy_exec::*;
pub use fuzzy_exec_seq::*;

// For now this is the form that rustfmt formats fairly nicely,
// and IntelliSense works fairly properly.
macro_rules! collector_test {
    (
        $name:ident {
            iter_data: { $(let mut $iter_data_ident:ident = $iter_data_strat:expr;)* },
            other_data: { $(let $other_data_pat:pat = $other_data_strat:expr;)* },

            iter: $iter:expr,
            collector: $collector:expr,
            expected_f: |$iter_ident:ident| $expected:expr,
            output_pred: $output_pred:expr,

            model: $model:expr $(,)?
        }
    ) => {
        $crate::test_utils::collector_test!($name {
            iter_data: { $(let mut $iter_data_ident = $iter_data_strat;)* },
            other_data: { $(let $other_data_pat = $other_data_strat;)* },

            iters: {
                ($iter, $iter, $iter,)
            },
            collector: $collector,
            expected_f: |$iter_ident| $expected,
            output_pred: $output_pred,

            model: $model,
        });
    };

    (
        $name:ident {
            iter_data: { $(let mut $iter_data_ident:ident = $iter_data_strat:expr;)* },
            other_data: { $(let $other_data_pat:pat = $other_data_strat:expr;)* },

            iters: {
                $(let $mid_iter_data_pat:pat = $mid_iter_data:expr;)*
                (
                    $iter_base:expr,
                    $iter_for_output:expr,
                    $iter_for_model:expr $(,)?
                )
            },
            collector: $collector:expr,
            expected_f: |$iter_ident:ident| $expected:expr,
            output_pred: $output_pred:expr,

            model: $model:expr $(,)?
        }
    ) => {
        ::proptest::proptest! {
            #[test]
            fn $name(
                ($($iter_data_ident,)* seq) in ($($iter_data_strat),*)
                    .prop_flat_map(|#[allow(unused_parens, unused_mut)] ($(mut $iter_data_ident),*)| {
                        let n = ::core::iter::Iterator::count($iter_base);
                        (
                            $(::proptest::strategy::Just($iter_data_ident),)*
                            $crate::test_utils::FuzzyExecSeqStrategy::new(n)
                        )
                    }),
                $($other_data_pat in $other_data_strat,)*
            ) {
                $(#[allow(unused_mut)] let mut $iter_data_ident = $iter_data_ident;)*
                $(let $mid_iter_data_pat = $mid_iter_data;)*

                let mut collected_amount = 0_usize;
                let mut expected_remaining = ::core::iter::Iterator::fuse($iter_for_output);
                let (expected_output, is_break) = (
                    |$iter_ident: &mut ::core::iter::Inspect<_, _>| $expected
                )(
                    &mut expected_remaining
                        .by_ref()
                        .inspect(|_| collected_amount += 1)
                );

                $crate::test_utils::fuzzy_execute(
                    $iter_base,
                    $iter_for_model,

                    expected_remaining.count(),

                    expected_output,
                    $output_pred,

                    is_break.then_some(collected_amount),

                    $collector,
                    &seq,

                    $model,
                )?;
            }
        }
    };
}

pub(super) use collector_test;

#[allow(unused_imports)]
pub mod prelude {
    pub use std::{
        // `Debug` for manual implementation of `TwoIterMutFactory` and `CollectorFactoryBase`.
        fmt::Debug,
        ops::ControlFlow,
    };

    pub use proptest::collection::vec as propvec;
    pub use proptest::option::of as prop_opt5050;
    pub use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    pub use crate::prelude::*;

    pub(crate) use super::collector_test;
    pub use super::{CollectorModel, theo_inf_collector_model};
}
