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
    pub use proptest::collection::vec as propvec;
    pub use proptest::prelude::*;
    pub use proptest::test_runner::TestCaseResult;

    pub use crate::{
        collector::IntoParallelCollectorBase,
        test_utils::{
            BasicParallelCollectorTester, CoroutinePool, DEFAULT_MAX_DEPTH, IndexedSplitDecision,
            IndexedSplitStrategy, ParallelCollectorTester, ParallelIterator, ParallelIteratorByRef,
            PredError, UnindexedParallelCollectorTester, UnindexedSplitDecision, UnindexedSplitStrategy,
        },
    };
}
