use std::{fmt::Debug, ops::ControlFlow};

use proptest::test_runner::{TestCaseError, TestCaseResult};

use crate::collector::{ParallelCollector, UnindexedParallelCollector};

use super::{
    CoroutinePool, IndexedParallelIterator, IndexedSplitDecision, ParallelIterator, UnindexedSplitDecision,
};

#[expect(clippy::too_many_arguments)]
pub fn check_parts_method<C, T, S, EO>(
    pool: &mut CoroutinePool,
    split_decision: &IndexedSplitDecision,

    mut iter: impl IndexedParallelIterator<Item = T>,
    mut collector: C,

    expected_output: &EO,
    output_pred: impl FnOnce(&C::Output, &EO) -> bool,

    starting_bh: ControlFlow<()>,
    ending: &ControlFlow<(), S>,
    state_pred: impl FnOnce(&C, &S) -> bool,
) -> TestCaseResult
where
    C: ParallelCollector<T, Output: Debug> + Debug,
    S: Debug,
    EO: Debug,
{
    const METHOD_NAME: &str = "parts";

    assert_starting_cf(collector.break_hint(), starting_bh)?;

    let (_, consumer, commit) = collector.parts(iter.len());
    let output = pool.bridge(iter.take_indexed_producer(), consumer, split_decision);
    assert_ending_cf(commit(output), raw_cf(ending), METHOD_NAME)?;

    if let ControlFlow::Continue(expected_state) = ending {
        assert_state(&collector, expected_state, state_pred, METHOD_NAME)?;
    }

    assert_output(&collector.finish(), expected_output, output_pred, METHOD_NAME)
}

pub fn check_take_parts_method<T, AO, EO>(
    pool: &mut CoroutinePool,
    split_decision: &IndexedSplitDecision,

    mut iter: impl IndexedParallelIterator<Item = T>,
    mut collector: impl ParallelCollector<T, Output = AO>,

    expected_output: &EO,
    output_pred: impl FnOnce(&AO, &EO) -> bool,

    starting_bh: ControlFlow<()>,
) -> TestCaseResult
where
    EO: Debug,
    AO: Debug,
{
    assert_starting_cf(collector.break_hint(), starting_bh)?;

    let (_, consumer, commit) = collector.take_parts(iter.len());
    let output = pool.bridge(iter.take_indexed_producer(), consumer, split_decision);
    commit(output);

    assert_output(&collector.finish(), expected_output, output_pred, "take_parts")
}

#[expect(clippy::too_many_arguments)]
pub fn check_parts_unindexed_method<C, T, S, EO>(
    pool: &mut CoroutinePool,
    split_decision: &UnindexedSplitDecision,

    mut iter: impl ParallelIterator<Item = T>,
    mut collector: C,

    expected_output: &EO,
    output_pred: impl FnOnce(&C::Output, &EO) -> bool,

    starting_bh: ControlFlow<()>,
    ending: &ControlFlow<(), S>,
    state_pred: impl FnOnce(&C, &S) -> bool,
) -> TestCaseResult
where
    C: UnindexedParallelCollector<T, Output: Debug> + Debug,
    S: Debug,
    EO: Debug,
{
    const METHOD_NAME: &str = "parts_unindexed";

    assert_starting_cf(collector.break_hint(), starting_bh)?;

    let (consumer, commit) = collector.parts_unindexed();
    let output = pool.bridge_unindexed(iter.take_producer(), consumer, split_decision);
    assert_ending_cf(commit(output), raw_cf(ending), METHOD_NAME)?;

    if let ControlFlow::Continue(expected_state) = ending {
        assert_state(&collector, expected_state, state_pred, METHOD_NAME)?;
    }

    assert_output(&collector.finish(), expected_output, output_pred, METHOD_NAME)
}

pub fn check_take_parts_unindexed_method<T, AO, EO>(
    pool: &mut CoroutinePool,
    split_decision: &UnindexedSplitDecision,

    mut iter: impl ParallelIterator<Item = T>,
    mut collector: impl UnindexedParallelCollector<T, Output = AO>,

    expected_output: &EO,
    output_pred: impl FnOnce(&AO, &EO) -> bool,

    starting_bh: ControlFlow<()>,
) -> TestCaseResult
where
    EO: Debug,
    AO: Debug,
{
    assert_starting_cf(collector.break_hint(), starting_bh)?;

    let (consumer, commit) = collector.take_parts_unindexed();
    let output = pool.bridge_unindexed(iter.take_producer(), consumer, split_decision);
    commit(output);

    assert_output(
        &collector.finish(),
        expected_output,
        output_pred,
        "take_parts_unindexed",
    )
}

fn assert_starting_cf(actual: ControlFlow<()>, expected: ControlFlow<()>) -> TestCaseResult {
    if actual == expected {
        Ok(())
    } else {
        Err(TestCaseError::fail(format!(
            "starting `break_hint()` mismatched: expected {expected:?}, got {actual:?}"
        )))
    }
}

fn assert_ending_cf(actual: ControlFlow<()>, expected: ControlFlow<()>, method_name: &str) -> TestCaseResult {
    if actual == expected {
        Ok(())
    } else {
        Err(TestCaseError::fail(format!(
            "method `{method_name}`: returning `ControlFlow` mismatched: expected {expected:?}, got {actual:?}"
        )))
    }
}

fn assert_output<AO, EO>(
    actual: &AO,
    expected: &EO,
    output_pred: impl FnOnce(&AO, &EO) -> bool,
    method_name: &str,
) -> TestCaseResult
where
    EO: Debug,
    AO: Debug,
{
    if output_pred(actual, expected) {
        Ok(())
    } else {
        Err(TestCaseError::fail(format!(
            "method `{method_name}`: output mismatched: expected {expected:?}, got {actual:?}"
        )))
    }
}

fn assert_state<C, S>(
    actual: &C,
    expected: &S,
    state_pred: impl FnOnce(&C, &S) -> bool,
    method_name: &str,
) -> TestCaseResult
where
    C: Debug,
    S: Debug,
{
    if state_pred(actual, expected) {
        Ok(())
    } else {
        Err(TestCaseError::fail(format!(
            "in method `{method_name}`: incorrect state: expected {expected:?}, got {actual:?}"
        )))
    }
}

fn raw_cf<B, C>(cf: &ControlFlow<B, C>) -> ControlFlow<()> {
    match cf {
        ControlFlow::Continue(_) => ControlFlow::Continue(()),
        ControlFlow::Break(_) => ControlFlow::Break(()),
    }
}
