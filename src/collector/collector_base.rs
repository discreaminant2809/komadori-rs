use std::ops::ControlFlow;

#[cfg(feature = "itertools")]
use itertools::Either;

#[cfg(feature = "unstable")]
use super::{AltBreakHint, Funnel, Nest, NestExact, TeeWith};
use super::{
    Chain, Cloning, Collector, Copying, Enumerate, Filter, FilterMap, FlatMap, Flatten, Fuse,
    Inspect, IntoCollector, IntoCollectorBase, Map, MapOutput, MapWhile, Partition, Skip,
    SkipWhile, Take, TakeWhile, Tee, TeeClone, TeeFunnel, TeeMut, Unbatching, Unzip,
    assert_collector, assert_collector_base,
};
#[cfg(feature = "itertools")]
use super::{PartitionMap, Update};

/// The base trait of a collector.
///
/// This trait defines the output type and methods that do not depend on the item type.
/// It is crucial to avoid "type annotation needed" because implementors may implement
/// different output types and implement methods differently based on the item type,
/// which is not desired. A collector should only have one and only one output type.
/// Allowing the output type (and such methods) to vary with the item type would be
/// confusing regardless.
///
/// Implementors should never implement this trait alone, but also implement
/// [`Collector`](super::Collector).
///
/// See the [module-level documentation](super) for more information.
///
/// # Dyn Compatibility
///
/// This trait is *dyn-compatible*, meaning it can be used as a trait object.
/// You do not need to specify the [`Output`](CollectorBase::Output) type.
/// The compiler will even emit a warning if you add the
/// [`Output`](CollectorBase::Output) type.
///
/// However, as a trait object, it is pretty much useless, as the only method
/// available is [`break_hint()`](CollectorBase::break_hint).
pub trait CollectorBase {
    /// The result this collector yields, via the [`finish()`](CollectorBase::finish) method.
    ///
    /// This assosciated type does not appear in trait objects.
    type Output
    where
        Self: Sized;

    /// Consumes the collector and returns the accumulated result.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let v = vec![1, 2, 3]
    ///     .into_collector()
    ///     .take(999)
    ///     .fuse()
    ///     .filter(|&x: &i32| x > 0);
    ///
    /// assert_eq!(v.finish(), [1, 2, 3]);
    /// ```
    fn finish(self) -> Self::Output
    where
        Self: Sized;

    /// Returns a hint whether the collector has stopped accumulating.
    ///
    /// Returns [`Break(())`] if it is guaranteed that the collector
    /// has stopped accumulating, or returns [`Continue(())`] otherwise.
    ///
    /// As specified in the [module-level documentation](crate::collector),
    /// after the stop is signaled somewhere else,
    /// including through [`collect()`] or similar methods or this method itself,
    /// the behavior of this method is unspecified.
    /// This may include returning [`Break(())`] even if the collector has conceptually stopped.
    ///
    /// This method should be called once and only once before collecting
    /// items in a loop to avoid consuming one item prematurely.
    /// It is not intended for repeatedly checking whether the
    /// collector has stopped. Use [`fuse()`](CollectorBase::fuse)
    /// if you find yourself needing such behavior.
    ///
    /// If the collector is uncertain, like "maybe I won’t accumulate… uh, fine, I will,"
    /// it is recommended to just return [`Continue(())`].
    /// For example, [`filter()`] might skip some items it collects,
    /// but still returns [`Continue(())`] as long as the underlying collector can still accumulate.
    /// The filter just denies "undesirable" items and does not signal termination
    /// (this is the job of [`take_while()`] instead).
    ///
    /// The default implementation always returns [`Continue(())`].
    ///
    /// # Examples
    ///
    /// Correct usage:
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .take_while(|&x| x != 3);
    ///
    /// let mut has_stopped = collector.break_hint().is_break();
    /// let mut num = 0;
    /// while !has_stopped {
    ///     has_stopped = collector.collect(num).is_break();
    ///     num += 1;
    /// }
    ///
    /// assert_eq!(collector.finish(), [0, 1, 2]);
    /// ```
    ///
    /// Incorrect usage:
    ///
    /// ```no_run
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .take_while(|&x| x != 3);
    ///
    /// let mut num = 0;
    /// // If `collect()` has returned `Break(())` in the previous iteration,
    /// // The usage of `break_hint()` here is NOT valid. ⚠️
    /// // By the current implementation, this may loop indefinitely
    /// // until your RAM explodes! (the `Vec` keeps expanding)
    /// while collector.break_hint().is_continue() {
    ///     let _ = collector.collect(num);
    ///     num += 1;
    /// }
    ///
    /// // May not be correct anymore. ⚠️
    /// assert_eq!(collector.finish(), [0, 1, 2]);
    /// ```
    ///
    /// [`Break(())`]: std::ops::ControlFlow::Break
    /// [`Continue(())`]: std::ops::ControlFlow::Continue
    /// [`Collector`]: crate::collector::Collector
    /// [`collect()`]: crate::collector::Collector::collect
    /// [`filter()`]: crate::collector::CollectorBase::filter
    /// [`take_while()`]: crate::collector::CollectorBase::take_while
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    /// Creates a collector that can "safely" collect items even after
    /// the underlying collector has stopped accumulating,
    /// without triggering undesired behaviors.
    ///
    /// Normally, a collector having stopped may behave unpredictably,
    /// including accumulating again.
    /// `fuse()` ensures that once a collector has stopped, subsequent items
    /// are guaranteed to **not** be accumulated. This means that at that point,
    /// [`collect()`](Collector::collect), [`collect_many()`](Collector::collect_many)
    /// and [`break_hint()`](CollectorBase::break_hint) are
    /// guaranteed to return [`Break(())`].
    ///
    /// # Examples
    ///
    /// Without `fuse()`:
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// // `take_while()` is one of a few collectors that do NOT fuse internally.
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .take_while(|&x| x != 3);
    ///
    /// assert!(collector.collect(1).is_continue());
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(3).is_break());
    ///
    /// // Use after `Break` ⚠️
    /// let _ = collector.collect(4);
    ///
    /// // What do you think what `collector.finish()` would yield? You can try it yourself.
    /// // (Spoiler: by the current implementation, it may NOT be `[1, 2]`!)
    /// # // Not shown to the doc. We only confirm our claim here.
    /// # assert_ne!(collector.finish(), [1, 2]);
    /// ```
    ///
    /// With `fuse()`:
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .take_while(|&x| x != 3)
    ///     .fuse();
    ///
    /// assert!(collector.collect(1).is_continue());
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(3).is_break());
    ///
    /// // From now on, there's only `Break`. No further items are accumulated.
    /// assert!(collector.collect(4).is_break());
    /// assert!(collector.collect(5).is_break());
    /// assert!(collector.collect_many([6, 7, 8, 9]).is_break());
    ///
    /// // The output is consistent again.
    /// assert_eq!(collector.finish(), [1, 2]);
    /// ```
    ///
    /// [`Continue(())`]: ControlFlow::Continue
    /// [`Break(())`]: ControlFlow::Break
    #[inline]
    fn fuse(self) -> Fuse<Self>
    where
        Self: Sized,
    {
        assert_collector_base(Fuse::new(self))
    }

    /// Creates a collector that lets both collectors collect the same item.
    ///
    /// For each item collected, the first collector collects the item
    /// copied with the [`Copy`] trait before the second collector collects it.
    ///
    /// `tee()` only stops when **both** collectors have stopped.
    ///
    /// If the item type of this adapter is `T`, both collectors must implement
    /// [`Collector<T>`](super::Collector), and `T` must implement [`Copy`].
    ///
    /// The [`Output`](CollectorBase::Output) is a tuple containing the outputs of
    /// both underlying collectors, in order.
    ///
    /// See the [module-level documentation](crate::collector) for
    /// when this adapter is used and other variants of `tee` adapters.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{prelude::*, cmp::Max};
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .tee(Max::new());
    ///
    /// assert!(collector.collect(4).is_continue());
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(6).is_continue());
    /// assert!(collector.collect(3).is_continue());
    ///
    /// assert_eq!(collector.finish(), (vec![4, 2, 6, 3], Some(6)));
    /// ```
    #[inline]
    fn tee<C>(self, other: C) -> Tee<Self, C::IntoCollector>
    where
        Self: Sized,
        C: IntoCollectorBase,
    {
        assert_collector_base(Tee::new(self, other.into_collector()))
    }

    /// Creates a collector that lets both collectors collect the same item.
    ///
    /// For each item collected, the first collector collects the item
    /// cloned with the [`Clone`] trait before the second collector collects it.
    /// If one of them has stopped, the implementation will **not** clone
    /// the item, and will instead feed it into the other for optimization.
    ///
    /// `tee_clone()` only stops when **both** collectors have stopped.
    ///
    /// If the item type of this adapter is `T`, both collectors must implement
    /// [`Collector<T>`](super::Collector), and `T` must implement [`Clone`].
    ///
    /// The [`Output`](CollectorBase::Output) is a tuple containing the outputs of
    /// both underlying collectors, in order.
    ///
    /// See the [module-level documentation](crate::collector) for
    /// when this adapter is used and other variants of `tee` adapters.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    /// use std::rc::Rc;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .take(2)
    ///     .tee_clone(vec![]);
    ///
    /// assert!(collector.collect(Rc::new(1)).is_continue());
    /// assert!(collector.collect(Rc::new(2)).is_continue());
    /// // From here, the `Rc` will NOT be cloned.
    /// assert!(collector.collect(Rc::new(3)).is_continue());
    ///
    /// let (nums1, nums2) = collector.finish();
    ///
    /// assert!(nums1.iter().map(|num| **num).eq([1, 2]));
    /// assert!(nums2.iter().map(|num| **num).eq([1, 2, 3]));
    /// assert!(nums2.iter().map(Rc::strong_count).eq([2, 2, 1]));
    /// ```
    #[inline]
    fn tee_clone<C>(self, other: C) -> TeeClone<Self, C::IntoCollector>
    where
        Self: Sized,
        C: IntoCollectorBase,
    {
        assert_collector_base(TeeClone::new(self, other.into_collector()))
    }

    /// Creates a collector that lets both collectors collect the same item.
    ///
    /// For each item collected, the first collector collects
    /// the mutable reference of the item before the second collector collects it.
    ///
    /// `tee_funnel()` only stops when **both** collectors have stopped.
    ///
    /// If the item type of this adapter is `T`,
    /// the first collector must implement [`for<'a> Collector<&'a mut T>`](super::Collector)
    /// (a collector that can collect a mutable reference with any lifetime),
    /// and the second collector must implement [`Collector<T>`](super::Collector).
    ///
    /// The [`Output`](CollectorBase::Output) is a tuple containing the outputs of
    /// both underlying collectors, in order.
    ///
    /// See the [module-level documentation](crate::collector) for
    /// when this adapter is used and other variants of `tee` adapters.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{prelude::*, clb_mut};
    ///
    /// let mut collector = String::new()
    ///     .into_concat()
    ///     .map(clb_mut!(|s: &mut String| -> &str { &s[..] }))
    ///     .tee_funnel(vec![]);
    ///
    /// let strings = ["noble", "and", "singer"].map(String::from);
    /// assert!(collector.collect_many(strings).is_continue());
    ///
    /// let (concat, string_vec) = collector.finish();
    ///
    /// assert_eq!(concat, "nobleandsinger");
    /// assert_eq!(string_vec, ["noble", "and", "singer"]);
    /// ```
    #[inline]
    fn tee_funnel<C>(self, other: C) -> TeeFunnel<Self, C::IntoCollector>
    where
        Self: Sized,
        C: IntoCollectorBase,
    {
        assert_collector_base(TeeFunnel::new(self, other.into_collector()))
    }

    /// Creates a collector that lets both collectors collect the same item.
    ///
    /// For each item collected, the first collector collects
    /// the mutable reference of the item before the second collector also
    /// collects the mutable reference of it.
    ///
    /// `tee_mut()` only stops when **both** collectors have stopped.
    ///
    /// If the item type of this adapter is `&'i mut T`,
    /// the first collector must implement [`for<'a> Collector<&'a mut T>`](super::Collector)
    /// (a collector that can collect a mutable reference with any lifetime),
    /// and the second collector must implement [`Collector<&'i mut T>`](super::Collector).
    ///
    /// The [`Output`](CollectorBase::Output) is a tuple containing the outputs of
    /// both underlying collectors, in order.
    ///
    /// See the [module-level documentation](crate::collector) for
    /// when this adapter is used and other variants of `tee` adapters.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{cmp::Max, prelude::*, clb_mut};
    ///
    /// let mut collector = String::new()
    ///     .into_concat()
    ///     .map(clb_mut!(|s: &mut String| -> &str { &s[..] }))
    ///     .tee_mut(Max::new().map(
    ///         clb_mut!(|s: &mut String| -> usize { s.len() })
    ///     ))
    ///     .tee_funnel(vec![]);
    ///
    /// let strings = ["noble", "and", "singer"].map(String::from);
    /// assert!(collector.collect_many(strings).is_continue());
    ///
    /// let ((concat, max_len), string_vec) = collector.finish();
    ///
    /// assert_eq!(concat, "nobleandsinger");
    /// assert_eq!(max_len, Some(6));
    /// assert_eq!(string_vec, ["noble", "and", "singer"]);
    /// ```
    #[inline]
    fn tee_mut<C>(self, other: C) -> TeeMut<Self, C::IntoCollector>
    where
        Self: Sized,
        C: IntoCollectorBase,
    {
        assert_collector_base(TeeMut::new(self, other.into_collector()))
    }

    /// Creates a collector that [`clone`](Clone::clone)s every collected item.
    ///
    /// This is useful when you have a [`Collector<T>`](super::Collector), but you
    /// need a [`for<'a> Collector<&'a mut T>`](super::Collector)
    /// or [`for<'a> Collector<&'a T>`](super::Collector).
    ///
    /// Many collectors may have implementations for references, such as collections.
    /// In this case, you do not need this adapter.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let collector = vec![]
    ///     .into_concat()
    ///     .cloning() // Try putting `cloning` before every other collector
    ///     .filter(|num: &&Vec<_>| num.len() > 1);
    ///
    /// let concat = [vec![0, 1, 2], vec![3], vec![4, 5]]
    ///     .iter()
    ///     .feed_into(collector);
    ///
    /// assert_eq!(concat, [0, 1, 2, 4, 5]);
    /// ```
    #[inline]
    fn cloning(self) -> Cloning<Self>
    where
        Self: Sized,
    {
        assert_collector_base(Cloning::new(self))
    }

    /// Creates a collector that copies every collected item.
    ///
    /// This is useful when you have a [`Collector<T>`](super::Collector), but you
    /// need a [`for<'a> Collector<&'a mut T>`](super::Collector)
    /// or [`for<'a> Collector<&'a T>`](super::Collector).
    ///
    /// Many collectors may have implementations for references, such as collections.
    /// In this case, you do not need this adapter.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let collector = vec![]
    ///     .into_collector()
    ///     .copying();
    ///
    /// let concat = [0, 1, 2, 3, 4]
    ///     .iter()
    ///     .feed_into(collector);
    ///
    /// assert_eq!(concat, [0, 1, 2, 3, 4]);
    /// ```
    #[inline]
    fn copying(self) -> Copying<Self>
    where
        Self: Sized,
    {
        assert_collector_base(Copying::new(self))
    }

    /// Creates a collector that stops accumulating after collecting the first `n` items,
    /// or fewer if the underlying collector stops sooner.
    ///
    /// `take(n)` collects items until either `n` items have been collected
    /// or the underlying collector stops, whichever happens first.
    /// For collections, the [`Output`](CollectorBase::Output) will contain
    /// at most `n` more items than it had before construction.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .take(3);
    ///
    /// assert!(collector.collect(1).is_continue());
    /// assert!(collector.collect(2).is_continue());
    ///
    /// // Immediately stops after the third item.
    /// assert!(collector.collect(3).is_break());
    /// # // Internal assertion.
    /// # assert!(collector.collect(4).is_break());
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3]);
    /// ```
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = String::new()
    ///     .into_collector()
    ///     .take(0);
    ///
    /// // This collector stops accumulating from construction.
    /// assert!(collector.break_hint().is_break());
    /// assert_eq!(collector.finish(), "");
    /// ```
    #[inline]
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        assert_collector_base(Take::new(self, n))
    }

    /// Creates a collector that skips the first `n` collected items
    /// before it begins accumulating them.
    ///
    /// `skip(n)` ignores collected items until `n` items have been collected.
    /// After that, subsequent items are accumulated normally.
    ///
    /// Note that in the current implementation,
    /// if the underlying collector has stopped accumulating during skipping,
    /// its [`collect()`], [`break_hint()`] and similar methods may return [`Break(())`],
    /// regardless of whether the adaptor has skipped enough items or not.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .skip(3);
    ///
    /// assert!(collector.collect(1).is_continue());
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(3).is_continue());
    ///
    /// // It has skipped enough items.
    /// assert!(collector.collect(4).is_continue());
    /// assert!(collector.collect(5).is_continue());
    ///
    /// assert_eq!(collector.finish(), [4, 5]);
    /// ```
    ///
    /// [`Break(())`]: ControlFlow::Break
    /// [`collect()`]: super::Collector::collect
    /// [`break_hint()`]: CollectorBase::break_hint
    fn skip(self, n: usize) -> Skip<Self>
    where
        Self: Sized,
    {
        assert_collector_base(Skip::new(self, n))
    }

    /// Creates a collector that destructures each 2-tuple `(A, B)` item and distributes its fields:
    /// `A` goes to the first collector, and `B` goes to the second collector.
    ///
    /// `unzip()` is useful when you want to split an [`Iterator`]
    /// producing tuples or structs into multiple collections.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// struct User {
    ///     id: u32,
    ///     name: String,
    ///     email: String,
    /// }
    ///
    /// let users = [
    ///     User {
    ///         id: 1,
    ///         name: "Alice".to_owned(),
    ///         email: "alice@mail.com".to_owned(),
    ///     },
    ///     User {
    ///         id: 2,
    ///         name: "Bob".to_owned(),
    ///         email: "bob@mail.com".to_owned(),
    ///     },
    /// ];
    ///
    /// let ((ids, names), emails) = users
    ///     .into_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_collector()
    ///             .unzip(vec![])
    ///             .unzip(vec![])
    ///             .map(|user: User| ((user.id, user.name), user.email)),
    ///     );
    ///
    /// assert_eq!(ids, [1, 2]);
    /// assert_eq!(names, vec!["Alice", "Bob"]);
    /// assert_eq!(emails, vec!["alice@mail.com", "bob@mail.com"]);
    /// ```
    #[inline]
    fn unzip<C>(self, other: C) -> Unzip<Self, C::IntoCollector>
    where
        Self: Sized,
        C: IntoCollectorBase,
    {
        assert_collector_base(Unzip::new(self, other.into_collector()))
    }

    /// Creates a collector that feeds every item in the first collector until it stops accumulating,
    /// then continues feeding items into the second one.
    ///
    /// The first collector should be finite (typically achieved with
    /// [`take`](CollectorBase::take) or [`take_while`](super::CollectorBase::take_while)),
    /// otherwise it will hoard all incoming items and never pass any to the second.
    ///
    /// The [`Output`](CollectorBase::Output) is a tuple containing the outputs of
    /// both underlying collectors, in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .take(2)
    ///     .chain(vec![]);
    ///
    /// assert!(collector.collect(1).is_continue());
    ///
    /// // Now the first collector stops accumulating, but the second one is still active.
    /// assert!(collector.collect(2).is_continue());
    ///
    /// // Now the second one takes the spotlight.
    /// assert!(collector.collect(3).is_continue());
    /// assert!(collector.collect(4).is_continue());
    /// assert!(collector.collect(5).is_continue());
    ///
    /// assert_eq!(collector.finish(), (vec![1, 2], vec![3, 4, 5]));
    /// ```
    #[inline]
    fn chain<C>(self, other: C) -> Chain<Self, C::IntoCollector>
    where
        Self: Sized,
        C: IntoCollectorBase,
    {
        assert_collector_base(Chain::new(self, other.into_collector()))
    }

    /// Creates a collector that transforms the final accumulated result.
    ///
    /// This is used when your output gets "ugly" after a chain of adaptors,
    /// or when you do not want to break your API by (accidentally) rearranging adaptors,
    /// or when you just want a different output type for your collector.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{prelude::*, iter::Count};
    ///
    /// let mut average = i32::adding()
    ///     .tee(Count::new())
    ///     .map_output(|(sum, count)| {
    ///         (count != 0).then(|| sum as f64 / count as f64)
    ///     });
    ///
    /// assert!(average.collect(1).is_continue());
    /// assert!(average.collect(6).is_continue());
    /// assert!(average.collect(4).is_continue());
    /// assert!(average.collect(2).is_continue());
    ///
    /// assert_eq!(average.finish(), Some(3.25));
    /// ```
    fn map_output<F, T>(self, f: F) -> MapOutput<Self, F>
    where
        Self: Sized,
        F: FnOnce(Self::Output) -> T,
    {
        assert_collector_base(MapOutput::new(self, f))
    }

    /// Creates a collector that feeds the underlying collector with
    /// the mutable reference to the item, "pretending" the collector
    /// accepts owned items.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .funnel();
    ///
    /// assert!(collector.collect_many([1, 2, 3]).is_continue());
    /// assert_eq!(collector.finish(), [1, 2, 3]);
    /// ```
    #[cfg(feature = "unstable")]
    #[inline]
    fn funnel(self) -> Funnel<Self>
    where
        Self: Sized,
    {
        assert_collector_base(Funnel::new(self))
    }

    /// Creates a collector that calls a closure on each item before collecting.
    ///
    /// This is used when you need a collector that collects `U`,
    /// but you have a collector that collects `T`. In that case,
    /// you can use `map()` to transform `U` into `T` before passing it along.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![].into_collector().map(|num| num * num);
    ///
    /// assert!(collector.collect_many(1..=5).is_continue());
    ///
    /// assert_eq!(collector.finish(), [1, 4, 9, 16, 25]);
    /// ```
    ///
    /// If you have multiple collectors with different item types, this adaptor bridges them.
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let (_strings, lens) = ["a", "bcd", "ef"]
    ///     .into_iter()
    ///     .feed_into(
    ///         "".to_owned()
    ///             .into_concat()
    ///             // Limitation: type annotation may be needed.
    ///             .tee(vec![].into_collector().map(|s: &str| s.len()))
    ///     );
    ///
    /// assert_eq!(lens, [1, 3, 2]);
    /// ```
    #[inline]
    fn map<F, T, U>(self, f: F) -> Map<Self, F>
    where
        Self: Collector<T> + Sized,
        F: FnMut(U) -> T,
    {
        assert_collector::<_, U>(Map::new(self, f))
    }

    /// Creates a collector that uses a closure to determine whether an item should be accumulated.
    ///
    /// The underlying collector only collects items for which the given predicate returns `true`.
    ///
    /// Note that even if an item is not accumulated, this adaptor will still return
    /// [`Continue(())`] as long as the underlying collector does. If you want the collector to stop
    /// after the first `false`, consider using [`take_while()`](CollectorBase::take_while) instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .filter(|&x| x % 2 == 0);
    ///
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(4).is_continue());
    /// assert!(collector.collect(0).is_continue());
    ///
    /// // Still `Continue` even if an item doesn’t satisfy the predicate.
    /// assert!(collector.collect(1).is_continue());
    ///
    /// assert_eq!(collector.finish(), [2, 4, 0]);
    /// ```
    ///
    /// [`Continue(())`]: ControlFlow::Continue
    #[inline]
    fn filter<F, T>(self, pred: F) -> Filter<Self, F>
    where
        Self: Collector<T> + Sized,
        F: FnMut(&T) -> bool,
    {
        assert_collector::<_, T>(Filter::new(self, pred))
    }

    /// Creates a collector that accumulates items as long as a predicate returns `true`.
    ///
    /// `take_while()` accumulates items until it encounters one for which the predicate returns `false`.
    /// Conceptually, that item and all subsequent ones will **not** be accumulated.
    /// However, you should ensure that you do not feed more items after it has signaled
    /// a stop.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = "".to_owned()
    ///     .into_concat()
    ///     .take_while(|&s| s != "stop");
    ///
    /// assert!(collector.collect("abc").is_continue());
    /// assert!(collector.collect("def").is_continue());
    ///
    /// // Immediately stops after "stop".
    /// assert!(collector.collect("stop").is_break());
    ///
    /// assert_eq!(collector.finish(), "abcdef");
    /// ```
    fn take_while<F, T>(self, pred: F) -> TakeWhile<Self, F>
    where
        Self: Collector<T> + Sized,
        F: FnMut(&T) -> bool,
    {
        assert_collector::<_, T>(TakeWhile::new(self, pred))
    }

    // fn step_by()

    /// Creates a collector that distributes items between two collectors based on a predicate.
    ///
    /// Items for which the predicate returns `true` are sent to the first collector,
    /// and those for which it returns `false` go to the second collector.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let collector = vec![]
    ///     .into_collector()
    ///     .partition(|&mut x| x % 2 == 0, vec![]);
    /// let (evens, odds) = collector.collect_then_finish(-5..5);
    ///
    /// assert_eq!(evens, [-4, -2, 0, 2, 4]);
    /// assert_eq!(odds, [-5, -3, -1, 1, 3]);
    /// ```
    #[inline]
    fn partition<C, F, T>(self, pred: F, other_if_false: C) -> Partition<Self, C::IntoCollector, F>
    where
        Self: Collector<T> + Sized,
        C: IntoCollector<T>,
        F: FnMut(&mut T) -> bool,
    {
        assert_collector::<_, T>(Partition::new(self, other_if_false.into_collector(), pred))
    }

    /// Creates a collector that lets both collectors collect the same item.
    ///
    /// For each item collected, the first collector collects the item
    /// mapped by a given closure before the second collector collects it.
    /// If the second collector stops accumulating, the item will **not**
    /// be mapped, and instead is fed directly into the first collector.
    ///
    /// `tee_with()` only stops when **both** collectors have stopped.
    ///
    /// If the item type of this adapter is `T`, the first collector must implement
    /// [`Collector<T>`](super::Collector) and [`Collector<U>`](super::Collector),
    /// and the second collector must implement [`Collector<T>`](super::Collector).
    /// Since many collectors do not collect two or more types of items,
    /// `U` is effectively also `T` in this case.
    ///
    /// The [`Output`](CollectorBase::Output) is a tuple containing the outputs of
    /// both underlying collectors, in order.
    ///
    /// See the [module-level documentation](crate::collector) for
    /// when this adapter is used and other variants of `tee` adapters.
    #[inline]
    #[cfg(feature = "unstable")]
    fn tee_with<C, F, T, U>(self, f: F, other: C) -> TeeWith<Self, C::IntoCollector, F>
    where
        Self: Collector<T> + Collector<U> + Sized,
        C: IntoCollector<T>,
        F: FnMut(&mut T) -> U,
    {
        assert_collector::<_, T>(TeeWith::new(self, other.into_collector(), f))
    }

    /// Creates a collector with a custom collection logic.
    ///
    /// This adaptor is useful for behaviors that cannot be expressed
    /// through existing adaptors without cloning or intermediate allocations.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = Vec::<i32>::new()
    ///     .into_collector()
    ///     .unbatching(|v, arr: &[_]| v.collect_many(arr));
    ///
    /// assert!(collector.collect(&[1, 2, 3]).is_continue());
    /// assert!(collector.collect(&[4, 5]).is_continue());
    /// assert!(collector.collect(&[6, 7, 8, 9]).is_continue());
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3, 4, 5, 6, 7, 8, 9]);
    /// ```
    fn unbatching<F, T>(self, f: F) -> Unbatching<Self, F>
    where
        Self: Sized,
        F: FnMut(&mut Self, T) -> ControlFlow<()>,
    {
        assert_collector_base(Unbatching::new(self, f))
    }

    // ///
    // #[inline]
    // fn map_ref_ref<F, T, U>(self, f: F) -> Map<Self, F>
    // where
    //     Self: for<'a> Collector<&'a T> + Sized,
    //     F: FnMut(&U) -> &T,
    //     T: ?Sized,
    //     U: ?Sized,
    // {
    //     assert_collector::<_, &U>(Map::new(self, f))
    // }

    // ///
    // #[inline]
    // fn map_mut_ref<F, T, U>(self, f: F) -> Map<Self, F>
    // where
    //     Self: for<'a> Collector<&'a T> + Sized,
    //     F: FnMut(&mut U) -> &T,
    //     T: ?Sized,
    //     U: ?Sized,
    // {
    //     assert_collector::<_, &mut U>(Map::new(self, f))
    // }

    // ///
    // #[inline]
    // fn map_mut_mut<F, T, U>(self, f: F) -> Map<Self, F>
    // where
    //     Self: for<'a> Collector<&'a mut T> + Sized,
    //     F: FnMut(&mut U) -> &mut T,
    //     T: ?Sized,
    //     U: ?Sized,
    // {
    //     assert_collector::<_, &mut U>(Map::new(self, f))
    // }

    /// A collector that flattens items by one level of nesting before collecting.
    ///
    /// Each item will be converted into an iterator, then the underlying collector
    /// collects every element in that iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .flatten();
    ///
    /// assert!(collector.collect([1, 2]).is_continue());
    /// assert!(collector.collect(&[] as &[i32]).is_continue());
    /// assert!(collector.collect(vec![3, 4, 5]).is_continue());
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3, 4, 5]);
    /// ```
    #[inline]
    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
    {
        assert_collector_base(Flatten::new(self))
    }

    /// A collector that collects elements in each iterator item provided by a closure.
    ///
    /// Each item will be mapped into an iterator by a closure,
    /// then the underlying collector collects every element in that iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = String::new()
    ///     .into_collector()
    ///     .flat_map(str::chars);
    ///
    /// assert!(collector.collect("elegance ").is_continue());
    /// assert!(collector.collect("and ").is_continue());
    /// assert!(collector.collect("radiance").is_continue());
    ///
    /// assert_eq!(collector.finish(), "elegance and radiance");
    /// ```
    #[inline]
    fn flat_map<F, T, I>(self, f: F) -> FlatMap<Self, F>
    where
        Self: Collector<I::Item> + Sized,
        F: FnMut(T) -> I,
        I: IntoIterator,
    {
        assert_collector::<_, T>(FlatMap::new(self, f))
    }

    /// Creates a "by reference" adapter for this collector.
    ///
    /// Used when you do not want, yet, consume the collector
    /// and reuse it further.
    ///
    /// It is possible since `&mut C` implements [`Collector<T>`]
    /// when `C` implements [`Collector<T>`].
    ///
    /// Due to this, function signatures and structs (using generics)
    /// should only either expect an [`impl Collector<T>`](Collector)
    /// or [`impl IntoCollector<T>`](super::IntoCollector)
    /// for more flexibility, allowing callers to opt for
    /// either ownership or borrowing.
    ///
    /// Also, if you do not chain adapters (before and after `by_ref()`),
    /// consider passing a `&mut collector` instead to express the intent better.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// fn fill_one_and_two(collector: impl IntoCollector<i32>) {
    ///     collector
    ///         .into_collector()
    ///         .collect_many([1, 2]);
    /// }
    ///
    /// let mut collector = vec![].into_collector();
    /// // `by_ref()` works, but this is more readable.
    /// fill_one_and_two(&mut collector);
    /// assert!(collector.collect(3).is_continue());
    /// assert_eq!(collector.finish(), [1, 2, 3]);
    ///
    /// let mut collector = vec![].into_collector();
    /// fill_one_and_two(collector.by_ref().filter(|&num| num % 2 == 0));
    /// assert!(collector.collect(3).is_continue());
    /// assert_eq!(collector.finish(), [2, 3]);
    /// ```
    #[inline]
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    /// Creates a collector that "views" each item first before collecting.
    ///
    /// It is used when you want to debug/log what happens between transformations.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .inspect(|&num| println!("After the filter: {num}"))
    ///     .filter(|&num| num % 2 != 0)
    ///     .inspect(|&num| println!("Before the filter: {num}"));
    ///
    /// assert!(collector.collect(1).is_continue());
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(3).is_continue());
    ///
    /// assert_eq!(collector.finish(), [1, 3]);
    /// ```
    #[inline]
    fn inspect<F, T>(self, f: F) -> Inspect<Self, F>
    where
        Self: Collector<T> + Sized,
        F: FnMut(&T),
    {
        assert_collector::<_, T>(Inspect::new(self, f))
    }

    /// Creates a collector that feeds the underlying collector with the current index
    /// alongside with the item.
    ///
    /// The underlying collector must implement [`Collector<(usize, T)>`],
    /// where the first element of the tuple is the current index, starting at `0`
    /// for the first item and incremented for each subsequent item.
    ///
    /// # Overflow behavior
    ///
    /// The method does no guarding against overflows, so collecting more than
    /// [`usize::MAX`] items either produces the wrong result or panics.
    /// If overflow checks are enabled, a panic is guaranteed.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .enumerate();
    ///
    /// assert!(collector.collect('a').is_continue());
    /// assert!(collector.collect('b').is_continue());
    /// assert!(collector.collect('c').is_continue());
    ///
    /// assert_eq!(collector.finish(), [(0, 'a'), (1, 'b'), (2, 'c')]);
    /// ```
    #[inline]
    fn enumerate(self) -> Enumerate<Self>
    where
        Self: Sized,
    {
        Enumerate::new(self)
    }

    /// Creates a collector that both filters and maps each item before collecting.
    ///
    /// The underlying collector only collects `value`s that the closure returns `Some(value)`.
    ///
    /// If you find yourself using `map()` and `filter()` consecutively, consider using
    /// `filter_map()` to be more concise.
    ///
    /// Note that even if an item is not accumulated, this adaptor will still return
    /// [`Continue(())`] as long as the underlying collector does.
    /// If you want the collector to stop after the first `false`,
    /// consider using [`map_while()`](CollectorBase::map_while) instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .filter_map(|s: &str| s.parse::<i32>().ok());
    ///
    /// assert!(collector.collect("1").is_continue());
    /// assert!(collector.collect("2").is_continue());
    /// assert!(collector.collect("three").is_continue());
    /// assert!(collector.collect("4").is_continue());
    ///
    /// assert_eq!(collector.finish(), [1, 2, 4]);
    /// ```
    ///
    /// [`Continue(())`]: ControlFlow::Continue
    #[inline]
    fn filter_map<P, T, R>(self, pred: P) -> FilterMap<Self, P>
    where
        Self: Collector<R> + Sized,
        P: FnMut(T) -> Option<R>,
    {
        assert_collector::<_, T>(FilterMap::new(self, pred))
    }

    /// Creates a collector that accumulates items as long as a predicate returns [`Some`].
    ///
    /// `map_while()` accumulates `value`s when the closure returns [`Some(value)`](Some),
    /// until it encounters one for which the predicate returns [`None`].
    /// Conceptually, that item and all subsequent ones will **not** be accumulated.
    /// However, you should ensure that you do not feed more items after it has signaled
    /// a stop.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .map_while(|s: &str| s.parse::<i32>().ok());
    ///
    /// assert!(collector.collect("1").is_continue());
    /// assert!(collector.collect("2").is_continue());
    ///
    /// // Immediately stops after a string that cannot be parsed into an integer.
    /// assert!(collector.collect("three").is_break());
    ///
    /// assert_eq!(collector.finish(), [1, 2]);
    /// ```
    #[inline]
    fn map_while<P, T, R>(self, pred: P) -> MapWhile<Self, P>
    where
        Self: Collector<R> + Sized,
        P: FnMut(T) -> Option<R>,
    {
        assert_collector::<_, T>(MapWhile::new(self, pred))
    }

    /// Creates a collector that skips the first collected items that satisfy a predicate
    /// before accumulating.
    ///
    /// `skip_while()` ignores collected items until the first item that
    /// does not satisfy the predicate.
    /// After that, this item and subsequent items are accumulated normally.
    ///
    /// Note that in the current implementation,
    /// if the underlying collector has stopped accumulating during skipping,
    /// its [`collect()`], [`break_hint()`] and similar methods may return [`Break(())`],
    /// regardless of whether the adapter has met an item that does not satisfy
    /// the predicate or not.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = String::new()
    ///     .into_collector()
    ///     .skip_while(|&ch| ch != '\n');
    ///
    /// assert!(collector.collect_many("noble\nand\nsinger".chars()).is_continue());
    /// assert_eq!(collector.finish(), "\nand\nsinger");
    /// ```
    ///
    /// [`Break(())`]: ControlFlow::Break
    /// [`collect()`]: super::Collector::collect
    /// [`break_hint()`]: CollectorBase::break_hint
    #[inline]
    fn skip_while<P, T>(self, pred: P) -> SkipWhile<Self, P>
    where
        Self: Collector<T> + Sized,
        P: FnMut(&T) -> bool,
    {
        SkipWhile::new(self, pred)
    }

    /// Creates a collector that alternates the behavior of [`break_hint()`](Self::break_hint).
    ///
    /// This is useful for [`unbatching()`](Self::unbatching) and
    /// [`TryFold`](crate::iter::TryFold), when you want to configure
    /// whether those stop accumulating on construction.
    ///
    /// You can leverage the fact that "after any of
    /// [`Collector::collect()`], [`Collector::collect_many()`], or
    /// [`CollectorBase::break_hint()`] have returned [`Break(())`] once,
    /// behaviors of subsequent calls to any method other than
    /// [`finish()`](CollectorBase::finish) are unspecified"
    /// to calculate the hint before collecting only.
    ///
    /// [`Break(())`]: ControlFlow::Break
    //    ///
    //    /// # Examples
    //    ///
    //    /// ```
    //    /// use std::ops::ControlFlow;
    //    /// use komadori::prelude::*;
    //    ///
    //    /// fn vec_zip(nums: impl IntoIterator<Item = i32>) -> impl Collector<i32, Output = Vec<i32>> {
    //    ///     let mut nums = nums.into_iter();
    //    ///     let sh = nums.size_hint();
    //    ///
    //    ///     vec![]
    //    ///         .into_collector()
    //    ///         .unbatching(move |collector, item| {
    //    ///             if let Some(num) = nums.next() {
    //    ///                 collector.collect(item)
    //    ///             } else {
    //    ///                 ControlFlow::Break(())
    //    ///             }
    //    ///         })
    //    ///         .alt_break_hint(move |_| {
    //    ///             if let (0, Some(0)) = sh {
    //    ///                 ControlFlow::Break(())
    //    ///             } else {
    //    ///                 ControlFlow::Continue(())
    //    ///             }
    //    ///         })
    //    /// }
    //    /// ```
    //    ///
    #[cfg(feature = "unstable")]
    #[inline]
    fn alt_break_hint<F>(self, f: F) -> AltBreakHint<Self, F>
    where
        Self: Sized,
        F: Fn(&Self) -> ControlFlow<()>,
    {
        assert_collector_base(AltBreakHint::new(self, f))
    }

    /// Creates a collector that distributes items between two collectors based on a predicate.
    ///
    /// Items for which the predicate returns [`Either::Left`] go to the first collector,
    /// and those for which it returns [`Either::Right`] go to the second collector.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .partition_map(From::from, vec![]);
    ///
    /// assert!(collector.collect(Ok(1)).is_continue());
    /// assert!(collector.collect(Err("Error")).is_continue());
    /// assert!(collector.collect(Ok(2)).is_continue());
    ///
    /// let (errs, oks) = collector.finish();
    ///
    /// assert_eq!(oks, [1, 2]);
    /// assert_eq!(errs, ["Error"]);
    /// ```
    #[cfg(feature = "itertools")]
    #[inline]
    fn partition_map<C, F, T, L, R>(
        self,
        pred: F,
        collector_right: C,
    ) -> PartitionMap<Self, C::IntoCollector, F>
    where
        Self: Collector<L> + Sized,
        C: IntoCollector<R>,
        F: FnMut(T) -> Either<L, R>,
    {
        PartitionMap::new(self, collector_right.into_collector(), pred)
    }

    /// Creates a collector that mutates each item first before collecting.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .update(|num| *num += 1);
    ///
    /// assert!(collector.collect(1).is_continue());
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(3).is_continue());
    ///
    /// assert_eq!(collector.finish(), [2, 3, 4]);
    /// ```
    #[cfg(feature = "itertools")]
    #[inline]
    fn update<F, T>(self, f: F) -> Update<Self, F>
    where
        Self: Collector<T> + Sized,
        F: FnMut(&mut T),
    {
        Update::new(self, f)
    }

    /// Creates a collector that collects all outputs produced by an inner collector.
    ///
    /// The inner collector collects items first until it stops accumulating,
    /// then, the outer collector collects the output produced by the inner collector,
    /// then repeat.
    ///
    /// The inner collector must implement [`Clone`]. Also, it should be finite
    /// so that the outer can collect more, or else the outer will be stuck with
    /// one output forever.
    ///
    /// This version collects the unfinished inner (the remainder), if any,
    /// after calling [`finish()`] or [`collect_then_finish()`].
    /// Hence, this adaptor is not "exact," similar to [`[_]::chunks()`](slice::chunks).
    /// Use [`nest_exact()`](CollectorBase::nest_exact) if you do not care about the remainder,
    /// since the exact verion is generally faster.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .nest(vec![].into_collector().take(3));
    ///
    /// assert!(collector.collect_many(1..=11).is_continue());
    ///
    /// assert_eq!(
    ///     collector.finish(),
    ///     [
    ///         vec![1, 2, 3],
    ///         vec![4, 5, 6],
    ///         vec![7, 8, 9],
    ///         vec![10, 11],
    ///     ],
    /// );
    /// ```
    ///
    /// [`finish()`]: CollectorBase::finish
    /// [`collect_then_finish()`]: Collector::collect_then_finish
    #[cfg(feature = "unstable")]
    fn nest<C>(self, inner: C) -> Nest<Self, C::IntoCollector>
    where
        Self: Collector<C::Output> + Sized,
        C: IntoCollectorBase<IntoCollector: Clone>,
    {
        assert_collector_base(Nest::new(self, inner.into_collector()))
    }

    /// Creates a collector that collects all outputs produced by an inner collector.
    ///
    /// The inner collector collects items first until it stops accumulating,
    /// then, the outer collector collects the output produced by the inner collector,
    /// then repeat.
    ///
    /// The inner collector must implement [`Clone`]. Also, it should be finite
    /// so that the outer can collect more, or else the outer will be stuck with
    /// one output forever.
    ///
    /// This version will only collect all the inners that has stopped accumulating.
    /// Any unfinished inner (the remainder) is discarded after calling
    /// [`finish()`] or [`collect_then_finish()`].
    /// Hence, this adaptor is "exact," similar to [`[_]::chunks_exact()`](slice::chunks_exact).
    /// Since the implementation is simpler, this adaptor is generally faster.
    /// Use [`nest()`](CollectorBase::nest) if you care about the remainder.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .nest_exact(vec![].into_collector().take(3));
    ///
    /// assert!(collector.collect_many(1..=11).is_continue());
    ///
    /// assert_eq!(
    ///     collector.finish(),
    ///     [
    ///         [1, 2, 3],
    ///         [4, 5, 6],
    ///         [7, 8, 9],
    ///     ],
    /// );
    /// ```
    ///
    /// [`finish()`]: CollectorBase::finish
    /// [`collect_then_finish()`]: Collector::collect_then_finish
    #[cfg(feature = "unstable")]
    fn nest_exact<C>(self, inner: C) -> NestExact<Self, C::IntoCollector>
    where
        Self: Collector<C::Output> + Sized,
        C: IntoCollectorBase<IntoCollector: Clone>,
    {
        assert_collector_base(NestExact::new(self, inner.into_collector()))
    }
}

impl<C> CollectorBase for &mut C
where
    C: CollectorBase,
{
    type Output = ();

    fn finish(self) -> Self::Output {}

    fn break_hint(&self) -> ControlFlow<()> {
        C::break_hint(self)
    }
}

macro_rules! dyn_impl {
    ($($traits:ident)*) => {
        impl<'a> CollectorBase for &mut (dyn CollectorBase $(+ $traits)* + 'a) {
            type Output = ();

            #[inline]
            fn finish(self) -> Self::Output {}

            #[inline]
            fn break_hint(&self) -> ControlFlow<()> {
                <dyn CollectorBase>::break_hint(self)
            }
        }

        impl<'a, T> CollectorBase for &mut (dyn super::Collector<T> $(+ $traits)* + 'a) {
            type Output = ();

            #[inline]
            fn finish(self) -> Self::Output {}

            #[inline]
            fn break_hint(&self) -> ControlFlow<()> {
                <dyn super::Collector<T>>::break_hint(self)
            }
        }
    };
}

dyn_impl!();
dyn_impl!(Send);
dyn_impl!(Sync);
dyn_impl!(Send Sync);

// `Output` shouldn't be required to be specified.
fn _dyn_compatible(_: &mut dyn CollectorBase) {}

// You actually read this? So here's a workaround for issues
// when you can't even name the type (e.g. closures, async blocks).
#[cfg(feature = "std")]
fn _unnamed_type_workaround() {
    use crate::{cmp::Max, prelude::*};

    [|| ""].into_iter().feed_into(
        Max::new()
            .map({
                fn f(s: &mut impl FnMut() -> &'static str) -> &'static str {
                    s()
                }
                f
            })
            .take_while({
                fn f(_: &&mut impl FnMut() -> &'static str) -> bool {
                    true
                }
                f
                // |_| true
            })
            .tee_funnel(vec![]),
    );
}
