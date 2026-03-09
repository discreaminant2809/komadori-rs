#[cfg(feature = "unstable")]
use super::Driver;

use crate::collector::{Collector, IntoCollector};
#[cfg(feature = "unstable")]
use crate::{assert_iterator, collector::CollectorBase};

/// Extends [`Iterator`] with various methods to work with [`Collector`]s.
///
/// This trait is automatically implemented for all [`Iterator`] types.
pub trait IteratorExt: Iterator {
    /// Feeds items from this iterator into the provided collector till
    /// the collector stops accumulating or the iterator is exhausted.
    /// and returns the collector’s output.
    ///
    /// Even though this method takes `self`, the collector will try their
    /// best to consume only as many items as it needs. To keep the iterator afterwards,
    /// use [`by_ref()`](Iterator::by_ref) before this method.
    ///
    /// To use this method, import the [`IteratorExt`] trait.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{prelude::*, cmp::Max};
    ///
    /// let (max, nums) = [4, 2, 6, 3]
    ///     .into_iter()
    ///     .feed_into(Max::new().tee(vec![]));
    ///
    /// assert_eq!(max, Some(6));
    /// assert_eq!(nums, [4, 2, 6, 3]);
    /// ```
    #[inline]
    fn feed_into<C>(self, collector: C) -> C::Output
    where
        Self: Sized,
        C: IntoCollector<Self::Item>,
    {
        collector.into_collector().collect_then_finish(self)
    }

    /// Extracts items from this iterator into the provided collector as far as the
    /// puller drives the iterator, then returns both the collector’s output and
    /// the puller’s result.
    ///
    /// The `puller` is a closure that receives an [`Iterator`] as a *driver*
    /// and produces an additional result.
    /// An item is collected only when the driver advances pass that item.
    /// If the driver is not fully exhausted, the iterator will not be fully
    /// collected either.
    ///
    /// Be careful when using short-circuiting methods on [`Iterator`] such as
    /// [`try_fold()`](Iterator::try_fold) or [`any()`](Iterator::any).
    /// They stop when something is satisfied, preventing the collector
    /// from collecting every item.
    /// Consider `for_each(drop)` the iterator before returning
    /// if you want to exhaust the iterator.
    ///
    /// To use this method, import the [`IteratorExt`] trait.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use komadori::prelude::*;
    ///
    /// let (s_no_ws, len_no_ws) = "the noble and the singer"
    ///     .split_whitespace()
    ///     .feed_into_with_puller(
    ///         String::new()
    ///             .into_concat()
    ///             .map({
    ///                 fn f<'a>(s: &mut &'a str) -> &'a str {
    ///                     s
    ///                 }
    ///                 f
    ///             }),
    ///         |driver| driver.count(),
    ///     );
    ///
    /// assert_eq!(s_no_ws, "thenobleandthesinger");
    /// assert_eq!(len_no_ws, 5);
    /// ```
    #[cfg(feature = "unstable")]
    fn feed_into_with_puller<C, R>(
        self,
        collector: C,
        puller: impl FnOnce(Driver<'_, Self, C::IntoCollector>) -> R,
    ) -> (C::Output, R)
    where
        Self: Sized,
        C: for<'a> IntoCollector<&'a mut Self::Item>,
    {
        let mut collector = collector.into_collector();
        let driver = assert_iterator(Driver::new(self, &mut collector));
        let ret = puller(driver);
        (collector.finish(), ret)
    }
}

impl<I> IteratorExt for I where I: Iterator + ?Sized {}
