//! [`Collector`]s for the unit type `()`.
//!
//! [`Collector`]: crate::collector::Collector

use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{CollectorBase, IntoCollectorBase};

/// A collector that always stops accumulating.
/// Its [`Output`](CollectorBase::Output) is `()`.
///
/// This struct is created by `().into_collector()`
/// and `().collector()`.
///
/// [`Collector`]: crate::collector::Collector
#[derive(Clone, Default)]
pub struct Collector(());

macro_rules! into_collector_impl {
    ($ty:ty) => {
        impl IntoCollectorBase for $ty {
            type Output = ();

            type IntoCollector = Collector;

            #[inline]
            fn into_collector(self) -> Self::IntoCollector {
                Collector(())
            }
        }
    };
}

into_collector_impl!(());
into_collector_impl!(&());

impl CollectorBase for Collector {
    type Output = ();

    fn finish(self) -> Self::Output {}

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Break(())
    }
}

impl<T> crate::collector::Collector<T> for Collector {
    #[inline]
    fn collect(&mut self, _item: T) -> ControlFlow<()> {
        ControlFlow::Break(())
    }

    /// It won't consume any items in an iterator.
    #[inline]
    fn collect_many(&mut self, _items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        ControlFlow::Break(())
    }

    /// It won't consume any items in an iterator.
    #[inline]
    fn collect_then_finish(self, _items: impl IntoIterator<Item = T>) -> Self::Output {
        // Nothing worth doing here
    }
}

impl Debug for Collector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Collector").finish()
    }
}
