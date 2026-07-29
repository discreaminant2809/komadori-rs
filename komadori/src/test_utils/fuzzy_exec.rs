use std::{
    fmt::{Debug, Display},
    ops::ControlFlow,
};

use proptest::{
    prop_assert, prop_assert_eq,
    test_runner::{Reason, TestCaseError},
};

use crate::collector::Collector;

use super::{EndSeqNode, FuzzyExecSeq, MiddleSeqNode};

pub struct CollectorModel<S, AF, MAF> {
    pub state: S,
    pub advance_f: AF,
    pub max_afford_f: MAF,
}

/// Model for theoretically infinite collectors.
///
/// It's applied for:
///
/// - (True) infinite collectors.
/// - Collectors that are infinite when you do not feed items that
///   makes them stop (e.g. `Any`, `Find`, `TryFold::with_output()` etc.)
pub fn theo_inf_collector_model<T>()
-> CollectorModel<(), impl FnMut(&mut (), T), impl FnMut(&(), usize) -> usize> {
    CollectorModel {
        state: (),
        advance_f: |_: &mut _, _| {},
        max_afford_f: |_: &_, request| request,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fuzzy_execute<T, C, EO, S>(
    iter_for_collector: impl Iterator<Item = T>,
    iter_for_model: impl Iterator<Item = T>,

    mut expected_remaining_count: usize,

    expected_output: EO,
    output_pred: impl FnOnce(&EO, &C::Output) -> bool,

    break_after: Option<usize>,

    mut collector: C,
    seq: &FuzzyExecSeq,

    mut model: CollectorModel<S, impl FnMut(&mut S, T), impl FnMut(&S, usize) -> usize>,
) -> Result<EO, TestCaseError>
where
    T: Debug,
    EO: Debug,
    C: Collector<T, Output: Debug>,
{
    let mut iter_for_collector = iter_for_collector.overreach_detector();
    let mut iter_for_model = iter_for_model.fuse();
    // This is tracked according to the contract of the Reserve API
    // to guard against malformed test cases regarding `assume_reserved_collect()`.
    let mut reserved_amount = 0;
    let mut actual_final_cf = None;
    let mut last_i = None;
    let mut collected_count = 0_usize;

    // The `middles` iterator is fused anyway.
    for (i, &node) in seq.middles.iter().enumerate() {
        const MISSING_ITEM_MSG: &str = "there should be an element";

        last_i = Some(i);

        match node {
            MiddleSeqNode::Reserve { additional } => {
                collector.reserve(additional);
                reserved_amount = additional;
            }
            MiddleSeqNode::MaxAfford { request } => {
                let actual = collector.max_afford(request);
                let expected = (model.max_afford_f)(&model.state, request);

                if actual != expected {
                    return Err(mismatch_after_step_error(i + 1, expected, actual));
                }
            }
            MiddleSeqNode::Collect => {
                collected_count += 1;
                reserved_amount = reserved_amount.saturating_sub(1);

                let item = iter_for_collector.next().expect(MISSING_ITEM_MSG);
                let actual_cf = collector.collect(item);
                // We've collected one item prematurely!
                if break_after == Some(0) {
                    expected_remaining_count = expected_remaining_count
                        .checked_sub(1)
                        .expect("the provided iterator and the remaining count are not equivalent");
                }

                let item = iter_for_model.next().expect(MISSING_ITEM_MSG);
                (model.advance_f)(&mut model.state, item);

                if let Some(break_after) = break_after
                    && break_after <= collected_count
                    && actual_cf.is_continue()
                {
                    return Err(mismatch_after_step_error(
                        i + 1,
                        format_args!("{:?}", ControlFlow::<()>::Break(())),
                        format_args!("{actual_cf:?}"),
                    ));
                }

                if actual_cf.is_break() {
                    actual_final_cf = Some(actual_cf);
                    break;
                }
            }
            MiddleSeqNode::AssumeReservedCollect => {
                collected_count += 1;
                assert!(reserved_amount > 0, "malformed test case");
                reserved_amount -= 1;

                let item = iter_for_collector.next().expect(MISSING_ITEM_MSG);
                // SAFETY: We've guarded against non-reservation.
                let actual_cf = unsafe { collector.assume_reserved_collect(item) };
                // We've collected one item prematurely!
                if break_after == Some(0) {
                    expected_remaining_count = expected_remaining_count
                        .checked_sub(1)
                        .expect("the provided iterator and the remaining count are not equivalent");
                }

                let item = iter_for_model.next().expect(MISSING_ITEM_MSG);
                (model.advance_f)(&mut model.state, item);

                if let Some(break_after) = break_after
                    && break_after <= collected_count
                    && actual_cf.is_continue()
                {
                    return Err(mismatch_after_step_error(
                        i + 1,
                        format_args!("{:?}", ControlFlow::<()>::Break(())),
                        format_args!("{actual_cf:?}"),
                    ));
                }

                if actual_cf.is_break() {
                    actual_final_cf = Some(actual_cf);
                    break;
                }
            }
            MiddleSeqNode::CollectMany { n } => {
                collected_count += n;
                reserved_amount = 0;

                let actual_cf = collector.collect_many(iter_for_collector.by_ref().take(n));
                iter_for_model.by_ref().take(n).for_each(|item| {
                    (model.advance_f)(&mut model.state, item);
                });

                if let Some(break_after) = break_after
                    && break_after <= collected_count
                    && actual_cf.is_continue()
                {
                    return Err(mismatch_after_step_error(
                        i + 1,
                        format_args!("{:?}", ControlFlow::<()>::Break(())),
                        format_args!("{actual_cf:?}"),
                    ));
                }

                if actual_cf.is_break() {
                    actual_final_cf = Some(actual_cf);
                    break;
                }
            }
        }
    }

    match (break_after, collected_count, actual_final_cf) {
        (None, _, Some(ControlFlow::Break(()))) => {
            return Err(mismatch_after_step_error(
                last_i.unwrap_or(0),
                format_args!("{:?}", ControlFlow::<()>::Continue(())),
                format_args!("{:?}", ControlFlow::<()>::Break(())),
            ));
        }
        (Some(break_after), collected_count, Some(ControlFlow::Continue(())))
            if break_after >= collected_count =>
        {
            return Err(mismatch_after_step_error(
                last_i.unwrap_or(0),
                format_args!("{:?}", ControlFlow::<()>::Break(())),
                format_args!("{:?}", ControlFlow::<()>::Continue(())),
            ));
        }
        _ => {}
    }

    let actual_output = match (actual_final_cf, seq.end) {
        (Some(ControlFlow::Continue(())) | None, EndSeqNode::CollectThenFinish) => {
            iter_for_model.for_each(|item| {
                (model.advance_f)(&mut model.state, item);
            });

            // Keep the iterator for the overreaching check.
            collector.collect_then_finish(&mut iter_for_collector)
        }
        (_, EndSeqNode::Finish | EndSeqNode::CollectThenFinish) => collector.finish(),
        (_, EndSeqNode::FinishBoxed) => Box::new(collector).finish_boxed(),
    };
    if iter_for_collector.overreached() {
        return Err(TestCaseError::Fail(Reason::from(
            "the iterator was pulled after returning `None`",
        )));
    }

    let actual_remaining_count = iter_for_collector.count();
    prop_assert_eq!(
        expected_remaining_count,
        actual_remaining_count,
        "the iterator was consumed incorrectly: expected {} remaining, got {} remaining",
        expected_remaining_count,
        actual_remaining_count,
    );

    prop_assert!(
        output_pred(&expected_output, &actual_output),
        "mismatched output: expected {expected_output:?}, got {actual_output:?}"
    );

    Ok(expected_output)
}

fn mismatch_after_step_error(
    step: usize,
    expected: impl Display,
    actual: impl Display,
) -> TestCaseError {
    TestCaseError::Fail(Reason::from(format!(
        "after step {step}, expected {expected}, got {actual}"
    )))
}

trait IteratorExt: Iterator {
    fn overreach_detector(self) -> OverreachDetector<Self>
    where
        Self: Sized,
    {
        OverreachDetector::StillGoing(self)
    }
}
impl<I> IteratorExt for I where I: Iterator {}

#[derive(Debug)]
enum OverreachDetector<I> {
    StillGoing(I),
    Stopped(bool),
}

impl<I> OverreachDetector<I> {
    fn overreached(&self) -> bool {
        matches!(self, Self::Stopped(true))
    }
}

impl<I> Iterator for OverreachDetector<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::StillGoing(iter) => {
                if let Some(item) = iter.next() {
                    Some(item)
                } else {
                    *self = Self::Stopped(false);
                    None
                }
            }
            Self::Stopped(overreached) => {
                *overreached = true;
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::StillGoing(iter) => iter.size_hint(),
            Self::Stopped(_) => (0, Some(0)),
        }
    }
}
