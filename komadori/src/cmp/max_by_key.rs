use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector};

use super::{Max, ValueKey};

/// A collector that computes the item among the items it collects
/// that gives the maximum value from a function.
///
/// Its [`Output`](CollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the maximum item otherwise.
///
/// This collector is constructed by [`Max::by_key()`](super::Max::by_key).
///
/// This collector corresponds to [`Iterator::max_by_key()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::Max};
///
/// let mut collector = Max::by_key(|s: &&str| s.len());
///
/// assert!(collector.collect("a").is_continue());
/// assert!(collector.collect("the").is_continue());
/// assert!(collector.collect("is").is_continue());
/// assert!(collector.collect("among").is_continue());
/// assert!(collector.collect("not").is_continue());
///
/// assert_eq!(collector.finish(), Some("among"));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use komadori::{prelude::*, cmp::Max};
///
/// assert_eq!(Max::by_key(|s: &&str| s.len()).finish(), None);
/// ```
#[derive(Clone)]
pub struct MaxByKey<T, K, F> {
    value_key_collector: Max<ValueKey<T, K>>,
    f: F,
}

impl<T, K, F> MaxByKey<T, K, F>
where
    K: Ord,
    F: FnMut(&T) -> K,
{
    #[inline]
    pub(super) const fn new(f: F) -> Self {
        assert_collector(Self {
            value_key_collector: Max::new(),
            f,
        })
    }
}

impl<T, K, F> CollectorBase for MaxByKey<T, K, F> {
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.value_key_collector.finish().map(ValueKey::into_value)
    }
}

impl<T, K, F> Collector<T> for MaxByKey<T, K, F>
where
    K: Ord,
    F: FnMut(&T) -> K,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        let item_value_key = ValueKey::new(item, &mut self.f);
        self.value_key_collector.collect(item_value_key)
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.value_key_collector.collect_many(
            items
                .into_iter()
                .map(|item| ValueKey::new(item, &mut self.f)),
        )
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let Self {
            value_key_collector,
            mut f,
        } = self;

        value_key_collector
            .collect_then_finish(
                items
                    .into_iter()
                    .map(move |item| ValueKey::new(item, &mut f)),
            )
            .map(ValueKey::into_value)
    }
}

impl<T: Debug, K: Debug, F> Debug for MaxByKey<T, K, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaxByKey")
            .field("max_value_key", &self.value_key_collector.max)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}
