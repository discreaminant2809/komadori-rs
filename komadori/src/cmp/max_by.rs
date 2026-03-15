use std::{cmp::Ordering, fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{Collector, CollectorBase, assert_collector},
    iter::Fold,
};

/// A collector that computes the maximum value among the items it collects
/// according to a comparison function.
///
/// Its [`Output`](CollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the maximum item otherwise.
///
/// This collector is constructed by [`Max::by()`](super::Max::by).
///
/// This collector corresponds to [`Iterator::max_by()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::Max};
///
/// let mut collector = Max::by(f64::total_cmp);
///
/// assert!(collector.collect(1.1).is_continue());
/// assert!(collector.collect(-2.3).is_continue());
/// assert!(collector.collect(f64::NEG_INFINITY).is_continue());
/// assert!(collector.collect(1E2).is_continue());
/// assert!(collector.collect(99.0_f64.sqrt()).is_continue());
///
/// assert_eq!(collector.finish(), Some(1E2));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use komadori::{prelude::*, cmp::Max};
///
/// assert_eq!(Max::by(f64::total_cmp).finish(), None);
/// ```
#[derive(Clone)]
pub struct MaxBy<T, F> {
    max: Option<T>,
    f: F,
}

impl<T, F> MaxBy<T, F>
where
    F: FnMut(&T, &T) -> Ordering,
{
    #[inline]
    pub(super) const fn new(f: F) -> Self {
        assert_collector(Self { max: None, f })
    }
}

impl<T, F> CollectorBase for MaxBy<T, F> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.max
    }
}

impl<T, F> Collector<T> for MaxBy<T, F>
where
    F: FnMut(&T, &T) -> Ordering,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self.max {
            None => self.max = Some(item),
            Some(ref mut max) => max_assign_by(max, item, &mut self.f),
        }

        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match self.max {
            // If we haven't collected at all, we can avoid `chain()`'s overhead.
            // See the below also.
            None => self.max = items.into_iter().max_by(&mut self.f),
            Some(ref mut max) => {
                // We can't just `max.max(items.into_iter().max_by())`.
                // We have to preserve the order of which is compared to which.
                // This is basically `chain()`, which doesn't override `max_by()`!
                items.into_iter().for_each({
                    let mut f = &mut self.f;
                    move |item| {
                        max_assign_by(max, item, &mut f);
                    }
                });
            }
        };

        ControlFlow::Continue(())
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.max {
            None => items.into_iter().max_by(self.f),
            // We don't use the std's `fold()` to account for large states.
            Some(max) => Some(
                Fold::new(max, move |max, item| max_assign_by(max, item, &mut self.f))
                    .collect_then_finish(items),
            ),
        }
    }
}

impl<T: Debug, F> Debug for MaxBy<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaxBy")
            .field("max", &self.max)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

fn max_assign_by<T, F>(max: &mut T, value: T, compare: F)
where
    F: FnOnce(&T, &T) -> Ordering,
{
    // See: https://doc.rust-lang.org/beta/src/core/cmp.rs.html#1704-1708
    if compare(max, &value).is_gt() {
    } else {
        *max = value;
    }
}
