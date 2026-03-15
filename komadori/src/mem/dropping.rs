use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

/// A collector that [`drops`](drop) every collected items.
///
/// # Examples
///
/// ```no_run
/// use komadori::{prelude::*, mem::Dropping};
/// use std::cell::Cell;
///
/// #[derive(Clone)]
/// struct IncCountOnDrop<'a>(&'a Cell<i32>);
///
/// impl Drop for IncCountOnDrop<'_> {
///     fn drop(&mut self) {
///         self.0.update(|count| count + 1);
///     }
/// }
///
/// let count = Cell::new(0);
///
/// std::iter::repeat_n(IncCountOnDrop(&count), 100)
///     .feed_into(Dropping);
///
/// assert_eq!(count.get(), 100);
/// ```
#[derive(Clone, Debug, Default)]
pub struct Dropping;

impl CollectorBase for Dropping {
    type Output = ();

    fn finish(self) -> Self::Output {}
}

impl<T> Collector<T> for Dropping {
    #[inline]
    fn collect(&mut self, _item: T) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        items.into_iter().for_each(drop);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        items.into_iter().for_each(drop);
    }
}

// #[cfg(all(test, feature = "std"))]
// mod proptests {
//     use std::{borrow::Borrow, cell::Cell, rc::Rc};

//     use proptest::prelude::*;
//     use proptest::test_runner::TestCaseResult;

//     use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

//     use super::*;

//     proptest! {
//         #[test]
//         fn all_collect_methods(
//             count in ..5_usize,
//         ) {
//             all_collect_methods_impl(count)?;
//         }
//     }

//     fn all_collect_methods_impl(count: usize) -> TestCaseResult {
//         let actual_count = Rc::new(Cell::new(0_usize));

//         #[derive(Clone)]
//         struct IncCountOnDrop<T: Borrow<Cell<usize>>>(T);

//         impl<T: Borrow<Cell<usize>>> Drop for IncCountOnDrop<T> {
//             fn drop(&mut self) {
//                 self.0.borrow().update(|count| count + 1);
//             }
//         }

//         BasicCollectorTester {
//             iter_factory: || std::iter::repeat_with(move || IncCountOnDrop(actual_count)).take(count),
//             collector_factory: || Dropping,
//             should_break_pred: |_| false,
//             pred: |_, _, remaining| {
//                 if actual_count.get() != count {
//                     Err(PredError::IncorrectOutput)
//                 } else if remaining.count() > 0 {
//                     Err(PredError::IncorrectIterConsumption)
//                 } else {
//                     Ok(())
//                 }
//             },
//         }
//         .test_collector()
//     }
// }
