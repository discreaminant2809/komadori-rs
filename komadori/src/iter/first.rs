use core::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, assert_collector, finish_boxed_impl};

/// A collector that stops after it collected just one item.
///
/// If it collected one item, its [`Output`] is [`Some(item)`](Some)
/// containing that item, otherwise [`None`].
///
/// This collector might seem silly, but it enables many patterns,
/// especially when combining with some adapters.
/// See examples for more.
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::First};
///
/// let mut collector = First::new();
///
/// assert!(collector.collect(1).is_break());
///
/// assert_eq!(collector.finish(), Some(1));
/// ```
///
/// ```
/// use komadori::{prelude::*, iter::First};
///
/// assert_eq!(First::<i32>::new().finish(), None);
/// ```
///
/// `First::new().filter(f)` corresponds to
/// [`Iterator::find(f)`](Iterator::find):
///
/// ```
/// use komadori::{prelude::*, iter::First};
///
/// let mut collector = First::new().filter(|&x| x % 3 == 0);
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(5).is_continue());
/// assert!(collector.collect(6).is_break());
///
/// assert_eq!(collector.finish(), Some(6));
/// ```
///
/// `First::new().filter_map(f)` corresponds to
/// [`Iterator::find_map(f)`](Iterator::find_map):
///
/// ```
/// use komadori::{prelude::*, iter::First};
///
/// let mut collector = First::new().filter_map(|s: &str| s.parse().ok());
///
/// assert!(collector.collect("noble").is_continue());
/// assert!(collector.collect("singer").is_continue());
/// assert!(collector.collect("1").is_break());
///
/// assert_eq!(collector.finish(), Some(1));
/// ```
///
/// `First::new().skip(n)` corresponds to
/// [`Iterator::nth(n)`](Iterator::nth):
///
/// ```
/// use komadori::{prelude::*, iter::First};
///
/// fn nth<T>(n: usize) -> impl Collector<T, Output = Option<T>> {
///     First::new().skip(n)
/// }
///
/// assert_eq!(nth(0).collect_then_finish(1..100), Some(1));
/// assert_eq!(nth(2).collect_then_finish(1..100), Some(3));
/// assert_eq!(nth(1000).collect_then_finish(1..100), None);
/// ```
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Clone)]
pub struct First<T> {
    value: Option<T>,
}

impl<T> First<T> {
    /// Creates an intance of this collector.
    #[inline]
    pub const fn new() -> Self {
        assert_collector::<_, T>(Self { value: None })
    }
}

impl<T> Default for First<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CollectorBase for First<T> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.value
    }

    finish_boxed_impl! {}

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        // Be careful that `request` can be 0.
        request.min(self.value.is_none() as _)
    }
}

impl<T> Collector<T> for First<T> {
    // Implmentation idea: They are called when we've not returned `Break` yet,
    // which only occurs when we `self.value` is `None`
    // (if the trait's contract is upheld).
    // Therefore, we can ignore the underlying value (at least not reading it)!
    // However, debug assertion could help too.

    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        debug_assert!(
            self.value.is_none(),
            "`collect()` called after `Break` was returned"
        );

        // This always stops after the first ever item.
        // What if the user keeps collecting? They would violate the trait's contract.
        self.value = Some(item);
        ControlFlow::Break(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        debug_assert!(
            self.value.is_none(),
            "`collect_many()` called after `Break` was returned"
        );

        self.value = items.into_iter().next();
        if self.value.is_some() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        debug_assert!(
            self.value.is_none(),
            "`collect_then_finish()` called after `Break` was returned"
        );

        items.into_iter().next()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::{collector::take_collector_model, test_utils::prelude::*};

    use super::*;

    collector_test!(collector {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {},
        iter: nums.iter().copied(),
        collector: First::new(),
        expected_f: |mut iter, _| {
            let res = iter.next();
            (res, res.is_some())
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(1),
    });
}
