use super::{CollectorBase, break_hint};

use std::ops::ControlFlow;

/// Defines what item types are accepted and how items are collected.
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
    /// For callers who choose to manually repeatedly call
    /// [`collect()`](Self::collect), to avoid consuming one item prematurely,
    /// you should check whether the collector can even afford a single item
    /// with `max_afford(request)`, with `request` being non-zero.
    /// If this returns `0`, it means it is not willing to accept any further items,
    /// and feeding it further is pointless.
    /// You do not need to worry about that with [`collect_many()`](Self::collect_many)
    /// and [`collect_then_finish()`](Self::collect_then_finish).
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
    /// # Notes
    ///
    /// When [`Break(())`] is returned, it implies that the iterator
    /// is safe to have [`next()`](Iterator::next) or similar methods called
    /// "safely."
    /// When [`Continue(())`] is returned, it implies that the iterator
    /// is exhausted, and future calls to [`next()`](Iterator::next) or similar methods
    /// may result in unspecified behaviors.
    ///
    /// Implementations must adhere to this semantics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![1, 2].into_collector();
    /// assert!(collector.collect_many([3, 4, 5]).is_continue());
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3, 4, 5]);
    /// ```
    ///
    /// [`Continue(())`]: ControlFlow::Continue
    /// [`Break(())`]: ControlFlow::Break
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()>
    where
        Self: Sized,
    {
        // Guard against something like `take(0)` when we can avoid
        // consuming one item prematurely.
        break_hint(self)?;

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

    /// Same as [`collect()`](Self::collect), but this does not check whether
    /// the reserved amount is enough or not.
    ///
    /// # Safety
    ///
    /// You must ensure that you have reserved for **at least one** item.
    /// Calling this method without any reservation could lead to undefined behavior,
    /// memory corruption, or other kinds of unsafety.
    ///
    /// See the ["Safety" section of the module-level documentation][safety-section] for more.
    ///
    /// [safety-section]: super#safety
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![].into_collector();
    /// collector.reserve(3);
    ///
    /// unsafe {
    ///     // SAFETY: We reserved for 3 items.
    ///     assert!(collector.assume_reserved_collect(1).is_continue());
    ///     assert!(collector.assume_reserved_collect(2).is_continue());
    ///     assert!(collector.assume_reserved_collect(3).is_continue());
    ///
    ///     // DO NOT do this. We didn't reserved for the 4th item.
    ///     // Potential UB. ⚠️
    ///     // collector.assume_reserved_collect(4);
    /// }
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3]);
    /// ```
    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        self.collect(item)
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

// `Output` shouldn't be required to be specified.
fn _dyn_compatible<T, O>(_: &mut dyn Collector<T, Output = O>) {}
