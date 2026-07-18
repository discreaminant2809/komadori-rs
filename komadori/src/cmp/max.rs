use std::{cmp::Ordering, ops::ControlFlow};

use super::{MaxBy, MaxByKey, max_assign};

use crate::{
    collector::{Collector, CollectorBase, assert_collector},
    iter::Fold,
};

/// A collector that computes the maximum value among the items it collects.
///
/// Its [`Output`](CollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the maximum item otherwise.
///
/// This collector corresponds to [`Iterator::max()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::Max};
///
/// let mut collector = Max::new();
///
/// assert!(collector.collect(1).is_continue());
/// assert!(collector.collect(3).is_continue());
/// assert!(collector.collect(2).is_continue());
/// assert!(collector.collect(5).is_continue());
/// assert!(collector.collect(3).is_continue());
///
/// assert_eq!(collector.finish(), Some(5));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use komadori::{prelude::*, cmp::Max};
///
/// assert_eq!(Max::<i32>::new().finish(), None);
/// ```
#[derive(Debug, Clone)]
pub struct Max<T> {
    // For `Debug` impl used by `MaxByKey`.
    pub(super) max: Option<T>,
}

impl<T> Max<T> {
    /// Creates a new instance of this collector.
    #[inline]
    pub const fn new() -> Self
    where
        T: Ord,
    {
        assert_collector(Self { max: None })
    }

    /// Creates a new instance of [`MaxBy`] with a given comparison function.
    #[inline]
    pub const fn by<F>(f: F) -> MaxBy<T, F>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        assert_collector(MaxBy::new(f))
    }

    /// Creates a new instance of [`MaxByKey`] with a given key-extraction function.
    #[inline]
    pub const fn by_key<K, F>(f: F) -> MaxByKey<T, K, F>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        assert_collector(MaxByKey::new(f))
    }
}

impl<T: Ord> Default for Max<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CollectorBase for Max<T> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.max
    }
}

impl<T: Ord> Collector<T> for Max<T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        // Somehow it yields a little bit codegen!
        // For now max-vec benefits from it.
        self.max = Some(match self.max.take() {
            None => item,
            Some(max) => max.max(item),
        });

        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match self.max {
            // If we haven't collected at all, we can avoid `chain()`'s overhead.
            // See the below also.
            None => self.max = items.into_iter().max(),
            Some(ref mut max) => {
                // We can't just `max.max(items.into_iter().max())`.
                // We have to preserve the order of which is compared to which.
                // This is basically `chain()`, which doesn't override `max()`!
                items.into_iter().for_each(move |item| {
                    max_assign(max, item);
                });
            }
        };

        ControlFlow::Continue(())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.max {
            None => items.into_iter().max(),
            // We don't use the std's `fold()` to account for large states.
            Some(max) => Some(Fold::new(max, max_assign).collect_then_finish(items)),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use super::*;
    use crate::test_utils::prelude::*;

    use super::super::test_utils::{Id, TriIterIdData, TriIterIdFactory};

    collector_test!(collector {
        iter_data: TriIterIdData::strategy(),
        collector_data: any::<()>(),
        iter_f: TriIterIdFactory,
        collector_f: |_: &_| Max::new(),
        output_f: |iter, _| iter.max(),
        model_f: |_| BasicCollectorModel {
            state: None,
            advance_f: |max: &mut Option<Id>, id| *max = Some(match max.take() {
                Some(max) => std::cmp::max(max, id),
                None => id,
            }),
            max_afford_f: |_, request| request,
            cf_f: |_| ControlFlow::Continue(()),
            output_and_pred_f: |max| (max, Id::full_eq_opt_ref)
        },
    });
}
