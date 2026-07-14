use std::{
    fmt::{Debug, Display},
    ops::ControlFlow,
};

use proptest::test_runner::{Reason, TestCaseError};

use crate::collector::Collector;

use super::{CollectorModel, EndSeqNode, FuzzyExecSeq, MiddleSeqNode};

#[derive(Debug)]
pub enum FuzzyExecError<T, EO, AO> {
    MaxAfford {
        expected: usize,
        actual: usize,
    },
    Collect {
        expected: ControlFlow<()>,
        actual: ControlFlow<()>,
    },
    AssumeReservedCollect {
        expected: ControlFlow<()>,
        actual: ControlFlow<()>,
    },
    CollectMany {
        expected: ControlFlow<()>,
        actual: ControlFlow<()>,
    },
    IncorrectOutput {
        expected: EO,
        actual: AO,
    },
    IncorrectIterRemaining {
        expected: Vec<T>,
        actual: Vec<T>,
    },
    Overreached,
}

pub(super) fn fuzzy_execute<I, IM, C, P, EO: Debug>(
    iter: I,
    mut iter_for_model: IM,
    mut collector: C,
    seq: &FuzzyExecSeq,
    mut model: P,
) -> Result<(), FuzzyExecError<I::Item, EO, C::Output>>
where
    I: Iterator<Item: PartialEq>,
    IM: Iterator<Item = I::Item>,
    C: Collector<I::Item>,
    P: CollectorModel<I::Item, EO, C::Output>,
{
    let mut iter = iter.overreach_detector();
    // This is tracked according to the contract of the Reserve API
    // to guard against malformed test cases regarding `assume_reserved_collect()`.
    let mut reserved_amount = 0;

    for &node in &seq.middles {
        const MISSING_ITEM_MSG: &str = "there should be an element";

        match node {
            MiddleSeqNode::Reserve { additional } => {
                collector.reserve(additional);
                reserved_amount = additional;
            }
            MiddleSeqNode::MaxAfford { request } => {
                let actual = collector.max_afford(request);
                let expected = model.expected_max_afford(request);

                if actual != expected {
                    return Err(FuzzyExecError::MaxAfford { expected, actual });
                }
            }
            MiddleSeqNode::Collect => {
                reserved_amount = reserved_amount.saturating_sub(1);

                let item = iter.next().expect(MISSING_ITEM_MSG);
                let actual = collector.collect(item);

                let item = iter_for_model.next().expect(MISSING_ITEM_MSG);
                model.advance(item);
                let expected = model.expected_cf();

                if actual != expected {
                    return Err(FuzzyExecError::Collect { expected, actual });
                }

                if actual.is_break() {
                    break;
                }
            }
            MiddleSeqNode::AssumeReservedCollect => {
                if reserved_amount == 0 {
                    panic!("malformed test case");
                }
                reserved_amount -= 1;

                let item = iter.next().expect(MISSING_ITEM_MSG);
                // SAFETY: We've guarded against non-reservation.
                let actual = unsafe { collector.assume_reserved_collect(item) };

                let item = iter_for_model.next().expect(MISSING_ITEM_MSG);
                model.advance(item);
                let expected = model.expected_cf();

                if actual != expected {
                    return Err(FuzzyExecError::Collect { expected, actual });
                }

                if actual.is_break() {
                    break;
                }
            }
            MiddleSeqNode::CollectMany { n } => {
                reserved_amount = 0;

                let actual = collector.collect_many(iter.by_ref().take(n));

                for item in iter_for_model.by_ref().take(n) {
                    model.advance(item);
                }
                let expected = model.expected_cf();

                if actual != expected {
                    return Err(FuzzyExecError::Collect { expected, actual });
                }

                if actual.is_break() {
                    break;
                }
            }
        }
    }

    let actual = match seq.end {
        EndSeqNode::Finish => collector.finish(),
        EndSeqNode::CollectThenFinish => {
            for item in iter_for_model.by_ref() {
                model.advance(item);
            }
            // Keep the iterator for the overreaching check.
            collector.collect_then_finish(&mut iter)
        }
    };
    if iter.overreached() {
        return Err(FuzzyExecError::Overreached);
    }

    let items = iter.collect::<Vec<_>>();
    let items_for_model = iter_for_model.collect::<Vec<_>>();
    if items != items_for_model {
        return Err(FuzzyExecError::IncorrectIterRemaining {
            expected: items_for_model,
            actual: items,
        });
    }

    let (expected, pred) = model.into_expected_output_and_pred();
    if !pred(&expected, &actual) {
        return Err(FuzzyExecError::IncorrectOutput { expected, actual });
    }

    Ok(())
}

impl<T, EO, AO> Display for FuzzyExecError<T, EO, AO>
where
    T: Debug,
    EO: Debug,
    AO: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn expected_actual(
            f: &mut std::fmt::Formatter<'_>,
            expected: impl Debug,
            actual: impl Debug,
        ) -> std::fmt::Result {
            f.write_fmt(format_args!("expected {expected:?}, got {actual:?}"))
        }

        match self {
            Self::MaxAfford { expected, actual } => {
                f.write_str("`max_afford()` returned incorrectly: ")?;
                expected_actual(f, expected, actual)
            }
            Self::Collect { expected, actual } => {
                f.write_str("`collect()` returned incorrectly: ")?;
                expected_actual(f, expected, actual)
            }
            Self::AssumeReservedCollect { expected, actual } => {
                f.write_str("`assume_reserved_collect()` returned incorrectly: ")?;
                expected_actual(f, expected, actual)
            }
            Self::CollectMany { expected, actual } => {
                f.write_str("`collect_many()` returned incorrectly: ")?;
                expected_actual(f, expected, actual)
            }
            Self::IncorrectOutput { expected, actual } => {
                f.write_str("incorrect output: ")?;
                expected_actual(f, expected, actual)
            }
            Self::IncorrectIterRemaining { expected, actual } => {
                f.write_str("incorrect iterator remaining: ")?;
                expected_actual(f, expected, actual)
            }
            Self::Overreached => {
                f.write_str("the iterator was pulled after it had returned `None`")
            }
        }
    }
}

impl<T, EO, AO> From<FuzzyExecError<T, EO, AO>> for TestCaseError
where
    T: Debug,
    EO: Debug,
    AO: Debug,
{
    fn from(e: FuzzyExecError<T, EO, AO>) -> Self {
        Self::Fail(Reason::from(format!("{e}")))
    }
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
