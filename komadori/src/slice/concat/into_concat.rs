use std::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase};

use super::ConcatItem;

/// A collector that concatenates items.
///
/// This `struct` is created by [`Concat::into_concat()`]. See its documentation for more.
///
/// [`Concat::into_concat()`]: super::Concat::into_concat
#[derive(Debug, Clone, Default)]
pub struct IntoConcat<S> {
    owned_slice: S,
}

impl<S> IntoConcat<S> {
    pub(super) fn new(owned_slice: S) -> Self {
        Self { owned_slice }
    }
}

impl<S> CollectorBase for IntoConcat<S> {
    type Output = S;

    #[inline]
    fn finish(self) -> Self::Output {
        self.owned_slice
    }
}

impl<S, T> Collector<T> for IntoConcat<S>
where
    T: ConcatItem<S>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        item.push_into(&mut self.owned_slice);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        T::bulk_push_into(items, &mut self.owned_slice);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        T::bulk_push_into(items, &mut self.owned_slice);
        self.owned_slice
    }
}
