use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

use super::ConcatItem;

/// A collector that concatenates items.
///
/// This `struct` is created by [`Concat::concat_mut()`]. See its documentation for more.
///
/// [`Concat::concat_mut()`]: super::Concat::concat_mut
#[derive(Debug)]
pub struct ConcatMut<'a, S> {
    owned_slice: &'a mut S,
}

impl<'a, S> ConcatMut<'a, S> {
    pub(super) fn new(owned_slice: &'a mut S) -> Self {
        Self { owned_slice }
    }
}

impl<'a, S> CollectorBase for ConcatMut<'a, S> {
    type Output = &'a mut S;

    #[inline]
    fn finish(self) -> Self::Output {
        self.owned_slice
    }
}

impl<'a, S, T> Collector<T> for ConcatMut<'a, S>
where
    T: ConcatItem<S>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        item.push_into(self.owned_slice);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        T::bulk_push_into(items, self.owned_slice);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        T::bulk_push_into(items, self.owned_slice);
        self.owned_slice
    }
}
