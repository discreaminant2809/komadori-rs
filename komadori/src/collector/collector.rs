use super::CollectorBase;

use std::ops::ControlFlow;

/// Defines what item types are accepted and how items are collected.
///
/// # Dyn Compatibility
///
/// This trait is *dyn-compatible*, meaning it can be used as a trait object.
/// You do not need to specify the [`Output`](CollectorBase::Output) type;
/// providing the item type `T` is enough.
/// The compiler will even emit a warning if you add the
/// [`Output`](CollectorBase::Output) type.
///
/// For example:
///
/// ```no_run
/// # use komadori::prelude::*;
/// # fn foo(_:
/// &mut dyn Collector<i32>
/// # ) {}
/// ```
///
/// [`Break(())`]: std::ops::ControlFlow::Break
pub trait Collector<T>: CollectorBase {
    /// Collects an item and returns a [`ControlFlow`] indicating whether
    /// the collector has stopped accumulating right after this operation.
    ///
    /// Return [`Continue(())`] to indicate the collector can still accumulate more items,
    /// or [`Break(())`] if it will not anymore and hence should no longer be fed further.
    ///
    /// This is analogous to [`Iterator::next()`], which returns an item (instead of collecting one)
    /// and signals with [`None`] whenever it finishes.
    ///
    /// Implementors should inform the caller about it as early as possible.
    /// This can usually be upheld, but not always.
    /// Some collectors, such as [`take(0)`](CollectorBase::take) and [`take_while()`],
    /// only know when they are done after collecting an item, which might be too late
    /// if the item cannot be “afforded” and is lost forever.
    /// In this case, call [`break_hint()`](CollectorBase::break_hint)
    /// **once and only once** before collecting (see its documentation to use it correctly).
    /// For "infinite" collectors (like most collections), this is not an issue
    /// since they can simply return  [`Continue(())`] every time.
    ///
    /// If the collector is uncertain, like "maybe I won’t accumulate… uh, fine, I will,"
    /// it is recommended to just return [`Continue(())`].
    /// For example, [`filter()`](CollectorBase::filter) might skip some items it collects,
    /// but still returns [`Continue(())`] as long as the underlying collector can still accumulate.
    /// The filter just denies "undesirable" items, not signal termination
    /// (this is the job of [`take_while()`] instead).
    ///
    /// Collectors with limited capacity (e.g., a `Vec` stored on the stack) will eventually
    /// return [`Break(())`] once full, right after the last item is accumulated.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![].into_collector().take(3); // only takes 3 items
    ///
    /// // It has not reached its 3-item quota yet.
    /// assert!(collector.collect(1).is_continue());
    /// assert!(collector.collect(2).is_continue());
    ///
    /// // After collecting `3`, it meets the quota, so it signals `Break` immediately.
    /// assert!(collector.collect(3).is_break());
    /// # // Internal assertion.
    /// # assert!(collector.collect(4).is_break());
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3]);
    /// ```
    ///
    /// Most collectors can accumulate indefinitely.
    ///
    /// ```
    /// use komadori::{prelude::*, iter::Last};
    ///
    /// let mut last = Last::new();
    /// for num in 0..100 {
    ///     assert!(last.collect(num).is_continue(), "cannot collect {num}");
    /// }
    ///
    /// assert_eq!(last.finish(), Some(99));
    /// ```
    ///
    /// [`Continue(())`]: ControlFlow::Continue
    /// [`Break(())`]: ControlFlow::Break
    /// [`take_while()`]: CollectorBase::take_while
    fn collect(&mut self, item: T) -> ControlFlow<()>;

    /// Collects items from an iterator and returns a [`ControlFlow`] indicating whether
    /// the collector has stopped collecting right after this operation.
    ///
    /// This method can be overridden for optimization and/or to avoid consuming one item prematurely.
    /// Implementors may choose a more efficient way to consume an iterator than a simple `for` loop
    /// ([`Iterator`] offers many alternative consumption methods), depending on the collector’s needs.
    ///
    /// Unlike [`collect()`](Self::collect), callers are **not** required to check for
    /// [`break_hint()`](CollectorBase::break_hint)
    /// and the implementors should guard against empty iterators.
    /// As a result, `collector.collect_many(empty_iter)` is an alternative
    /// way to check whether this collector has stopped accumulating.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![1, 2].into_collector();
    /// collector.collect_many([3, 4, 5]);
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3, 4, 5]);
    /// ```
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()>
    where
        Self: Sized,
    {
        self.break_hint()?;

        // Use `try_for_each` instead of `for` loop since the iterator may not be optimal for `for` loop
        // (e.g. `skip`, `chain`, etc.)
        items.into_iter().try_for_each(|item| self.collect(item))
    }

    /// Collects items from an iterator, consumes the collector, and produces the accumulated result.
    ///
    /// This is equivalent to calling [`collect_many`](Collector::collect_many)  
    /// followed by [`finish`](CollectorBase::finish) (which is the default implementation),
    /// but it can be overridden for optimization (e.g., to skip tracking internal state)
    /// because the collector will be dropped anyway.
    /// For instance, [`take()`](CollectorBase::take) overrides this method to avoid tracking
    /// how many items have been collected.
    ///
    /// Unlike [`collect()`](Self::collect), callers are **not** required to check for
    /// [`break_hint()`](CollectorBase::break_hint)
    /// and the implementors should guard against empty iterators.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use komadori::prelude::*;
    ///
    /// let collector = vec![1, 2].into_collector();
    ///
    /// assert_eq!(collector.collect_then_finish([3, 4, 5]), [1, 2, 3, 4, 5]);
    /// ```
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output
    where
        Self: Sized,
    {
        // Do this instead of putting `mut` in `self` since some IDEs are stupid
        // and just put `mut self` in every generated code.
        let mut this = self;

        // We don't care whether the collector breaks or not, since if it doesn't it'll have
        // completely depleted the iterator so... we just finish--nothing changed.
        let _ = this.collect_many(items);
        this.finish()
    }

    // /// A special case for [`map()`](Collector::map) that works around
    // /// lifetime inference issues in closure parameters.
    // ///
    // /// This maps an item of type `&U` to `T`. If `T` is a reference
    // /// whose lifetime is tied to `&U`,
    // /// consider using [`map_ref_ref()`](CollectorBase::map_ref_ref).
    // #[inline]
    // fn map_ref<F, U>(self, f: F) -> Map<Self, F>
    // where
    //     Self: Sized,
    //     F: FnMut(&U) -> T,
    //     U: ?Sized,
    // {
    //     assert_collector::<_, &U>(Map::new(self, f))
    // }

    // /// A special case for [`map()`](Collector::map) that works around
    // /// lifetime inference issues in closure parameters.
    // ///
    // /// This maps an item of type `&mut U` to `T`. If `T` is a (mutable) reference
    // /// whose lifetime is tied to `&mut U`,
    // /// consider using [`map_mut_ref()`](CollectorBase::map_mut_ref)
    // /// or [`map_mut_mut()`](CollectorBase::map_mut_mut).
    // #[inline]
    // fn map_mut<F, U>(self, f: F) -> Map<Self, F>
    // where
    //     Self: Sized,
    //     F: FnMut(&mut U) -> T,
    //     U: ?Sized,
    // {
    //     assert_collector::<_, &mut U>(Map::new(self, f))
    // }
}

/// A mutable reference to a collect produce nothing.
///
/// This is useful when you *just* want to feed items to a collector without
/// finishing it.
impl<C, T> Collector<T> for &mut C
where
    C: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        C::collect(self, item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // FIXED: specialization for unsized type.
        // We can't add `?Sized` to the bound of `C` because this method requires `Sized`.
        C::collect_many(self, items)
    }

    // The default implementation for `collect_then_finish()` is sufficient.
}

macro_rules! dyn_impl {
    ($($traits:ident)*) => {
        impl<'a, T> Collector<T> for &mut (dyn Collector<T> $(+ $traits)* + 'a) {
            #[inline]
            fn collect(&mut self, item: T) -> ControlFlow<()> {
                <dyn Collector<T>>::collect(*self, item)
            }

            // The default implementations are sufficient.
        }
    };
}

dyn_impl!();
dyn_impl!(Send);
dyn_impl!(Sync);
dyn_impl!(Send Sync);

// `Output` shouldn't be required to be specified.
fn _dyn_compatible<T>(_: &mut dyn Collector<T>) {}
