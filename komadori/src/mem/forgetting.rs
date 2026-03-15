use std::{fmt::Debug, mem::forget, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// A collector that "[forgets](forget)" every item it collects.
///
/// # Examples
///
/// ```no_run
/// use komadori::{prelude::*, mem::Forgetting};
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
///     .feed_into(Forgetting);
///
/// // The destructor was never run once.
/// assert_eq!(count.get(), 0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Forgetting;

impl CollectorBase for Forgetting {
    type Output = ();

    fn finish(self) -> Self::Output {}
}

impl<T> Collector<T> for Forgetting {
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        forget(item);
        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        items.into_iter().for_each(forget);
        ControlFlow::Continue(())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        items.into_iter().for_each(forget);
    }
}
