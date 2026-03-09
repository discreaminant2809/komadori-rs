use std::{cmp::Ordering, fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{Collector, CollectorBase, assert_collector},
    iter::Fold,
};

/// A collector that computes the minimum value among the items it collects
/// according to a comparison function.
///
/// Its [`Output`](CollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the minimum item otherwise.
///
/// This collector is constructed by [`Min::by()`](super::Min::by).
///
/// This collector corresponds to [`Iterator::min_by()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::Min};
///
/// let mut collector = Min::by(f64::total_cmp);
///
/// assert!(collector.collect(1.1).is_continue());
/// assert!(collector.collect(-2.3).is_continue());
/// assert!(collector.collect(f64::INFINITY).is_continue());
/// assert!(collector.collect(-1E2).is_continue());
/// assert!(collector.collect((-1_f64).sin()).is_continue());
///
/// assert_eq!(collector.finish(), Some(-1E2));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use komadori::{prelude::*, cmp::Min};
///
/// assert_eq!(Min::by(f64::total_cmp).finish(), None);
/// ```
#[derive(Clone)]
pub struct MinBy<T, F> {
    min: Option<T>,
    f: F,
}

impl<T, F> MinBy<T, F>
where
    F: FnMut(&T, &T) -> Ordering,
{
    #[inline]
    pub(super) const fn new(f: F) -> Self {
        assert_collector(Self { min: None, f })
    }
}

impl<T, F> CollectorBase for MinBy<T, F> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.min
    }
}

impl<T, F> Collector<T> for MinBy<T, F>
where
    F: FnMut(&T, &T) -> Ordering,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self.min {
            None => self.min = Some(item),
            Some(ref mut min) => min_assign_by(min, item, &mut self.f),
        }

        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match self.min {
            // If we haven't collected at all, we can avoid `chain()`'s overhead.
            // See the below also.
            None => self.min = items.into_iter().min_by(&mut self.f),
            Some(ref mut min) => {
                // We can't just `min.min(items.into_iter().min_by())`.
                // We have to preserve the order of which is compared to which.
                // This is basically `chain()`, which doesn't override `min_by()`!
                items.into_iter().for_each({
                    let mut f = &mut self.f;
                    move |item| {
                        min_assign_by(min, item, &mut f);
                    }
                });
            }
        };

        ControlFlow::Continue(())
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.min {
            None => items.into_iter().min_by(self.f),
            // We don't use the std's `fold()` to account for large states.
            Some(min) => Some(
                Fold::new(min, move |min, item| min_assign_by(min, item, &mut self.f))
                    .collect_then_finish(items),
            ),
        }
    }
}

impl<T: Debug, F> Debug for MinBy<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinBy").field("min", &self.min).finish()
    }
}

fn min_assign_by<T, F>(min: &mut T, value: T, compare: F)
where
    F: FnOnce(&T, &T) -> Ordering,
{
    // See: https://doc.rust-lang.org/beta/src/core/cmp.rs.html#1704-1708
    if compare(min, &value).is_le() {
    } else {
        *min = value;
    }
}
