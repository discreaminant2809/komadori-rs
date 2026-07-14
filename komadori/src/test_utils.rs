mod collector_model;
mod collector_tester;
mod fuzzy_exec;
mod fuzzy_exec_seq;
mod fuzzy_executor;
mod two_iter_fac;
mod two_iter_mut_fac;

pub use collector_model::*;
pub use collector_tester::*;
pub use fuzzy_exec_seq::*;
pub use fuzzy_executor::*;
pub use two_iter_fac::*;
pub use two_iter_mut_fac::*;

use fuzzy_exec::*;

// For now this is the form that rustfmt formats fairly nicely,
// and IntelliSense works fairly properly.
macro_rules! collector_test {
    (
        $name:ident {
            iter_data: $iter_data_strat:expr,
            collector_data: $collector_data_strat:expr,
            iter_f: $iter_f:expr,
            collector_f: $collector_f:expr,
            model_f: $model_f:expr $(,)?
        }
    ) => {
        ::proptest::proptest! {
            #[test]
            fn $name(
                executor in $crate::test_utils::FuzzyExecutor::strategy(
                    $iter_data_strat,
                    $collector_data_strat,
                    $iter_f,
                    $collector_f,
                )
            ) {
                let mut executor = executor;
                executor.execute($model_f)?;
            }
        }
    };
}

pub(super) use collector_test;
