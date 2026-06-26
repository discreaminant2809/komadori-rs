mod coroutine_pool;
mod indexed_split_strategy;
mod par_collector_tester;
mod par_iter;
mod producer;
mod unindexed_split_strategy;

pub use coroutine_pool::*;
pub use indexed_split_strategy::*;
pub use par_collector_tester::*;
pub use par_iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator, ParallelIteratorByRef};
pub use producer::*;
pub use unindexed_split_strategy::*;

pub const DEFAULT_MAX_DEPTH: usize = 4;

pub mod prelude {
    pub use super::is_subsequence;
    pub use proptest::collection::vec as propvec;
    pub use proptest::option::of as prop_opt;
    pub use proptest::prelude::*;
    pub use proptest::test_runner::TestCaseResult;

    pub use crate::{
        collector::{IntoParallelCollectorBase, ParallelCollectorBase, UnindexedParallelCollectorBase},
        test_utils::{
            BasicParallelCollectorTester, CoroutinePool, DEFAULT_MAX_DEPTH, IndexedSplitDecision,
            IndexedSplitStrategy, IntoParallelIterator, ParallelCollectorTester, ParallelIterator,
            ParallelIteratorByRef, PredError, UnindexedParallelCollectorTester, UnindexedSplitDecision,
            UnindexedSplitStrategy,
        },
    };
}

/// Used for testing unindexed parallel collectors.
pub fn is_subsequence<T>(iter1: impl IntoIterator<Item = T>, iter2: impl IntoIterator<Item = T>) -> bool
where
    T: PartialEq,
{
    let mut iter2 = iter2.into_iter();
    iter1.into_iter().all(|x1| iter2.any(move |x2| x1 == x2))
}
