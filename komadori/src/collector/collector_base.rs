use core::ops::ControlFlow;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::collector::{Intersperse, IntersperseWith};

#[cfg(feature = "itertools")]
use super::Update;
use super::{
    Chain, Cloning, Collector, Copying, Enumerate, Filter, FilterMap, FlatMap, Flatten, Fuse,
    Inspect, IntoCollectorBase, Map, MapOutput, MapWhile, Partition, Skip, SkipWhile, StepBy, Take,
    TakeWhile, Tee, TeeClone, TeeFunnel, TeeMut, TryingOptions, TryingResults, Unbatching, Unzip,
    assert_collector, assert_collector_base,
};
#[cfg(feature = "unstable")]
use super::{Funnel, Nest, NestExact, Then};

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
pub trait CollectorBase {
    /// The result this collector yields, via the [`finish()`](CollectorBase::finish) method.
    type Output;

    /// Consumes the collector and returns the accumulated result.
    ///
    /// This method cannot be implemented on unsized types.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let collector = vec![1, 2, 3]
    ///     .into_collector()
    ///     .take(999)
    ///     .fuse()
    ///     .filter(|&x: &i32| x > 0);
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3]);
    /// ```
    fn finish(self) -> Self::Output
    where
        Self: Sized;

    /// Same as [`finish()`], except it can be implemented
    /// on unsized types via [`Box`].
    ///
    /// This exists mainly for `Box<dyn _>` support.
    /// You rarely need to call this method.
    ///
    /// This method does not have a default implementation due to
    /// [`finish()`] requiring [`Sized`]. Therefore, you have to
    /// implement it manually for sized types with this
    /// (and should only be this):
    ///
    /// ```ignore
    /// fn finish_boxed(self: Box<Self>) -> Self::Output {
    ///     (*self).finish()
    /// }
    /// ```
    ///
    /// Hopefully this limitation may be lifted soon when
    /// a proper support for unsized types comes.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let collector = vec![1, 2, 3]
    ///     .into_collector()
    ///     .take(999)
    ///     .fuse()
    ///     .filter(|&x: &i32| x > 0);
    /// let collector = Box::new(collector) as Box<dyn Collector<_, Output = _>>;
    ///
    /// assert_eq!(collector.finish_boxed(), [1, 2, 3]);
    /// ```
    ///
    /// [`finish()`]: Self::finish
    #[cfg(feature = "alloc")]
    fn finish_boxed(self: Box<Self>) -> Self::Output;

    /// Reserves for `additional` items before collecting.
    ///
    /// It does nothing for the default implementation and most collectors,
    /// but it calls [`reserve()`](alloc::vec::Vec::reserve) for [`Vec`](alloc::vec::Vec).
    ///
    /// This method has an interaction with [`assume_reserved_collect()`](Collector::assume_reserved_collect).
    /// See the ["Safety" section of the module-level documentation][safety-section] for more.
    ///
    /// [safety-section]: super#safety
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .take(2)
    ///     .tee(vec![]);
    ///
    /// // The first collector only reserves for 2 items,
    /// // while the second collector reserves for 4 items as usual.
    /// collector.reserve(4);
    ///
    /// for num in 1..=4 {
    ///     assert!(collector.collect(num).is_continue(), "can't collect {num}");
    /// }
    ///
    /// let (first, second) = collector.finish();
    /// assert_eq!(first, [1, 2]);
    /// assert_eq!(second, [1, 2, 3, 4]);
    /// ```
    #[inline]
    fn reserve(&mut self, additional: usize) {
        let _additional = additional;
        // Does nothing.
    }

    /// Queries the maximum amount of items this collector can afford
    /// given the requested amount of items.
    ///
    /// Be aware that the returned `usize` is just the maximum.
    /// The collector is permitted to stop earlier than the maximum amount.
    /// Also, the returned `usize` must be less than or equal to `request`,
    /// or else the behavior is unspecified.
    ///
    /// `max_afford(1) == 0` can serve as a test whether this collector
    /// has stopped or not. For infinite collectors, this method
    /// always returns `request`.
    ///
    /// The default implementation returns `request`.
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
    /// // This collector can afford 2 items!
    /// assert_eq!(collector.max_afford(2), 2);
    ///
    /// assert!(collector.collect_many([1, 2]).is_continue());
    /// // This collector can only afford 1 more now.
    /// assert_eq!(collector.max_afford(2), 1);
    ///
    /// assert!(collector.collect(3).is_break());
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3]);
    /// ```
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// assert_eq!(0_i32.into_sum().max_afford(999_999_999), 999_999_999);
    /// ```
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .take_while(|&num| num != 4);
    ///
    /// // This collector can afford 1 million items (at best)!
    /// assert_eq!(collector.max_afford(1_000_000), 1_000_000);
    ///
    /// // Well, it is just "at best"...
    /// assert!(collector.collect_many([1, 2, 3]).is_continue());
    /// assert!(collector.collect(4).is_break());
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3]);
    /// ```
    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        request
    }

    /// Creates a collector that can "safely" collect items even after
    /// the underlying collector has stopped accumulating,
    /// without triggering undesired behaviors.
    ///
    /// Normally, a collector having stopped may behave unpredictably,
    /// including accumulating again.
    /// `fuse()` ensures that once a collector has stopped, subsequent items
    /// are guaranteed to **not** be accumulated. This means that at that point,
    /// [`collect()`](Collector::collect) and [`collect_many()`](Collector::collect_many)
    /// are guaranteed to return [`Break(())`].
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
    /// use komadori::{prelude::*, cmp::Max, iter::Fold};
    ///
    /// let mut max_subarray_sum = Fold::new(0, {
    ///     // The compiler is stupid so we get to write like this
    ///     let f = |curr_sum: &mut i32, num: &mut _| {
    ///         *curr_sum += *num;
    ///         *num = *curr_sum;
    ///         *curr_sum = (*curr_sum).max(0);
    ///     };
    ///     f
    /// })
    /// .tee_funnel(Max::new())
    /// .map_output(|(_, max)| max);
    ///
    /// assert!(
    ///     max_subarray_sum
    ///         .collect_many([2, -3, 4, -1, 2, 1, -5, 4])
    ///         .is_continue()
    /// );
    ///
    /// assert_eq!(max_subarray_sum.finish(), Some(6));
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
    /// In a collector converted from a tuple, you may need this adapters
    /// for collectors that are not the last in the chain.
    ///
    /// Many collectors may have implementations for references, such as collections.
    /// In this case, you do not need this adapter.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{prelude::*, cmp::{Max, Min}};
    ///
    /// let mut collector = (
    ///     Max::new().cloning(),
    ///     Min::new(),
    /// ).into_collector();
    ///
    /// assert!(collector.collect("a".to_owned()).is_continue());
    /// assert!(collector.collect("c".to_owned()).is_continue());
    /// assert!(collector.collect("b".to_owned()).is_continue());
    ///
    /// assert_eq!(collector.finish(), (Some("c".to_owned()), Some("a".to_owned())));
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
    /// In a collector converted from a tuple, you may need this adapters
    /// for collectors that are not the last in the chain.
    ///
    /// Many collectors may have implementations for references, such as collections.
    /// In this case, you do not need this adapter.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{prelude::*, cmp::{Max, Min}};
    ///
    /// let mut collector = (
    ///     Max::new().copying(),
    ///     Min::new(),
    /// ).into_collector();
    ///
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(3).is_continue());
    /// assert!(collector.collect(1).is_continue());
    ///
    /// assert_eq!(collector.finish(), (Some(3), Some(1)));
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
    /// // Can only afford 3 items!
    /// assert_eq!(collector.max_afford(5), 3);
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
    /// its [`collect()`] and [`collect_many()`] may return [`Break(())`],
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
    /// [`collect()`]: Collector::collect
    /// [`collect_many()`]: Collector::collect_many
    #[inline]
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
    /// #[derive(Debug, PartialEq)]
    /// struct Vec3D {
    ///     x: i32,
    ///     y: i32,
    ///     z: i32,
    /// }
    ///
    /// let vectors = [
    ///     Vec3D { x: 1, y: 2, z: 3 },
    ///     Vec3D { x: -1, y: 0, z: 1 },
    ///     Vec3D { x: 2, y: 3, z: -5 },
    /// ];
    ///
    /// let sum = vectors
    ///     .into_iter()
    ///     .feed_into(
    ///         0i32.into_sum()
    ///             .unzip(0.into_sum())
    ///             .unzip(0.into_sum())
    ///             .map(|vector: Vec3D| ((vector.x, vector.y), vector.z))
    ///             .map_output(|((x, y), z)| Vec3D { x, y, z }),
    ///     );
    ///
    /// assert_eq!(sum, Vec3D { x: 2, y: 5, z: -1 });
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
    /// let mut average = 0_i32
    ///     .into_sum()
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
    #[inline]
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
    #[inline]
    fn take_while<F, T>(self, pred: F) -> TakeWhile<Self, F>
    where
        Self: Collector<T> + Sized,
        F: FnMut(&T) -> bool,
    {
        assert_collector::<_, T>(TakeWhile::new(self, pred))
    }

    /// Creates a collector that enters a fixed "cooldown" after accumulating one item.
    /// Items that are collected during the cooldown will be ignored.
    ///
    /// The first item is guaranteed to be accumulated. Then, `step_by(n)`
    /// will be skipping the next `n - 1` items before it can accumulate again.
    ///
    /// # Panics
    ///
    /// Panics if `n` is `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .step_by(3);
    ///
    /// assert!(collector.collect_many(0..=10).is_continue());
    ///
    /// assert_eq!(collector.finish(), [0, 3, 6, 9]);
    /// ```
    #[inline]
    fn step_by(self, step: usize) -> StepBy<Self>
    where
        Self: Sized,
    {
        assert_collector_base(StepBy::new(self, step))
    }

    /// Creates a collector that distributes items between two collectors based on
    /// whether an item is "left" or "right."
    ///
    /// Items in [`Either::Left`] go to the first collector,
    /// and items in [`Either::Right`] go to the second collector.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{
    ///     prelude::*,
    ///     cmp::Max,
    ///     iter::Count,
    /// };
    /// use either::IntoEither;
    ///
    /// let nums = [1, 4, 2, 5];
    ///
    /// let (max_even, odd_count) = nums
    ///     .into_iter()
    ///     .map(|x| x.into_either(x % 2 == 0))
    ///     .feed_into(Max::new().partition(Count::new()));
    ///
    /// assert_eq!(max_even, Some(4));
    /// assert_eq!(odd_count, 2);
    /// ```
    ///
    /// It may be more readable to use [`crate::collector::partition()`]:
    ///
    /// ```
    /// use komadori::{
    ///     prelude::*,
    ///     collector::partition,
    /// };
    /// use either::IntoEither;
    ///
    /// let (evens, odds) = (-5..5)
    ///     .map(|x| x.into_either(x % 2 == 0))
    ///     // More readable than `vec![].into_collector().partition(vec![])`!
    ///     .feed_into(partition(vec![], vec![]));
    ///
    /// assert_eq!(evens, [-4, -2, 0, 2, 4]);
    /// assert_eq!(odds, [-5, -3, -1, 1, 3]);
    /// ```
    ///
    /// [`Either::Left`]: crate::either::Either::Left
    /// [`Either::Right`]: crate::either::Either::Right
    #[inline]
    fn partition<R>(self, right: R) -> Partition<Self, R::IntoCollector>
    where
        Self: Sized,
        R: IntoCollectorBase,
    {
        assert_collector_base(Partition::new(self, right.into_collector()))
    }

    /// Creates a collector with a custom collection logic.
    ///
    /// This adaptor is useful for behaviors that cannot be expressed
    /// through existing adaptors without cloning or intermediate allocations.
    ///
    /// # Notes
    ///
    /// The closure should compose the stop signal of the underlying collector
    /// (either from [`ControlFlow`]-returning methods or `max_afford(1) == 0`,
    /// even if the underlying collector does not collect anything at all,
    /// to signal a stop as soon as possible.
    /// In fact, [`max_afford()`](Self::max_afford) implementation
    /// of this collector returns `0` the underlying collector returns `0`,
    /// skipping the closure.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{prelude::*, cmp::Max};
    /// use std::ops::ControlFlow;
    ///
    /// let mut curr_sum = 0;
    /// let mut max_subarray_sum = Max::new()
    ///     .unbatching(move |max_sum, num| {
    ///         curr_sum += num;
    ///         max_sum.collect(curr_sum)?;
    ///         curr_sum = curr_sum.max(0);
    ///         ControlFlow::Continue(())
    ///     });
    ///
    /// assert!(
    ///     max_subarray_sum
    ///         .collect_many([2, -3, 4, -1, 2, 1, -5, 4])
    ///         .is_continue()
    /// );
    ///
    /// assert_eq!(max_subarray_sum.finish(), Some(6));
    /// ```
    #[inline]
    fn unbatching<F, T>(self, f: F) -> Unbatching<Self, F>
    where
        Self: Sized,
        F: FnMut(&mut Self, T) -> ControlFlow<()>,
    {
        assert_collector::<_, T>(Unbatching::new(self, f))
    }

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
        assert_collector_base(self)
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
        assert_collector_base(Enumerate::new(self))
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
    /// its [`collect()`] and [`collect_many()`] may return [`Break(())`],
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
    /// [`collect()`]: Collector::collect
    /// [`collect_many()`]: Collector::collect_many
    #[inline]
    fn skip_while<P, T>(self, pred: P) -> SkipWhile<Self, P>
    where
        Self: Collector<T> + Sized,
        P: FnMut(&T) -> bool,
    {
        assert_collector::<_, T>(SkipWhile::new(self, pred))
    }

    /// Creates a collector that sets the [`Output`] to [`None`] when
    /// a [`None`] item is encountered for the first time,
    /// else the underlying collector collects the `item` inside
    /// [`Some(item)`](Some).
    ///
    /// If the item type of the underlying collector is `T`, the item type of
    /// `trying_options()` is `Option<T>`.
    ///
    /// This is analogous to when you collect an iterator of [`Option<T>`]
    /// to an `Option<Collection<T>>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .trying_options();
    ///
    /// assert!(collector.collect(Some(1)).is_continue());
    /// assert!(collector.collect(Some(2)).is_continue());
    /// assert!(collector.collect(Some(3)).is_continue());
    ///
    /// assert_eq!(collector.finish(), Some(vec![1, 2, 3]));
    /// ```
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .trying_options();
    ///
    /// assert!(collector.collect(Some(1)).is_continue());
    /// assert!(collector.collect(Some(2)).is_continue());
    /// assert!(collector.collect(None::<i32>).is_break());
    ///
    /// assert_eq!(collector.finish(), None);
    /// ```
    ///
    /// [`Output`]: CollectorBase::Output
    #[inline]
    fn trying_options(self) -> TryingOptions<Self>
    where
        Self: Sized,
    {
        assert_collector_base(TryingOptions::new(self))
    }

    /// Creates a collector that sets the [`Output`] to [`Err(e)`](Err) when
    /// an [`Err(e)`](Err) item is encountered for the first time,
    /// else the underlying collector collects the `item` inside
    /// [`Ok(item)`](Ok).
    ///
    /// This is analogous to when you collect an iterator of [`Result<T, E>`]
    /// to a `Result<Collection<T>, E>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .trying_results::<&str>();
    ///
    /// assert!(collector.collect(Ok(1)).is_continue());
    /// assert!(collector.collect(Ok(2)).is_continue());
    /// assert!(collector.collect(Ok(3)).is_continue());
    ///
    /// assert_eq!(collector.finish(), Ok(vec![1, 2, 3]));
    /// ```
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .trying_results();
    ///
    /// assert!(collector.collect(Ok(1)).is_continue());
    /// assert!(collector.collect(Ok(2)).is_continue());
    /// assert!(collector.collect(Err::<i32, _>("can't collect anymore")).is_break());
    ///
    /// assert_eq!(collector.finish(), Err("can't collect anymore"));
    /// ```
    ///
    /// [`Output`]: CollectorBase::Output
    #[inline]
    fn trying_results<E>(self) -> TryingResults<Self, E>
    where
        Self: Sized,
    {
        assert_collector_base(TryingResults::new(self))
    }

    /// Creates a collector that separates collected items with a separator.
    ///
    /// No separator is collected when the first item is collected. Afterwards,
    /// the underlying collector collects a separator first before an item.
    ///
    /// The separator must implement [`Clone`].
    ///
    /// # Truncation
    ///
    /// If you call [`max_afford()`](Self::max_afford) with `request` more than
    /// [`isize::MAX`], it may yield less than expected, even though
    /// the underlying collector is infinite.
    ///
    /// # Panics
    ///
    /// [`reserve()`](Self::reserve) of this adapter may panic if you reserve
    /// for more than [`isize::MAX`] items.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = String::new()
    ///     .into_concat()
    ///     .intersperse(" ");
    ///
    /// assert!(collector.collect("noble").is_continue());
    ///
    /// assert_eq!(collector.finish(), "noble");
    /// ```
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut collector = String::new()
    ///     .into_concat()
    ///     .intersperse(" ");
    ///
    /// assert!(collector.collect("noble").is_continue());
    /// assert!(collector.collect("and").is_continue());
    /// assert!(collector.collect("singer").is_continue());
    ///
    /// assert_eq!(collector.finish(), "noble and singer");
    /// ```
    #[inline]
    fn intersperse<S>(self, sep: S) -> Intersperse<Self, S>
    where
        Self: Sized,
        S: Clone,
    {
        assert_collector_base(Intersperse::new(self, sep))
    }

    /// Creates a collector that separates collected items with a separator
    /// from a function.
    ///
    /// `intersperse_with()` works the same as [`intersperse()`](Self::intersperse),
    /// except each separator is generated from a function instead.
    /// This is useful when the separator type is not [`Clone`]
    /// or you want to compute the separators every time.
    ///
    /// # Truncation
    ///
    /// If you call [`max_afford()`](Self::max_afford) with `request` more than
    /// [`isize::MAX`], it may yield less than expected, even though
    /// the underlying collector is infinite.
    ///
    /// # Panics
    ///
    /// [`reserve()`](Self::reserve) of this adapter may panic if you reserve
    /// for more than [`isize::MAX`] items.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut sep = 0;
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .intersperse_with(move || {
    ///         sep -= 1;
    ///         sep
    ///     });
    ///
    /// assert!(collector.collect(1).is_continue());
    ///
    /// assert_eq!(collector.finish(), [1]);
    /// ```
    ///
    /// ```
    /// use komadori::prelude::*;
    ///
    /// let mut sep = 0;
    /// let mut collector = vec![]
    ///     .into_collector()
    ///     .intersperse_with(move || {
    ///         sep -= 1;
    ///         sep
    ///     });
    ///
    /// assert!(collector.collect(1).is_continue());
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(3).is_continue());
    ///
    /// assert_eq!(collector.finish(), [1, -1, 2, -2, 3]);
    /// ```
    #[inline]
    fn intersperse_with<FS, S>(self, sep_f: FS) -> IntersperseWith<Self, FS>
    where
        Self: Sized,
        FS: FnMut() -> S,
    {
        assert_collector_base(IntersperseWith::new(self, sep_f))
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
        assert_collector::<_, T>(Update::new(self, f))
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
    #[inline]
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
    #[inline]
    fn nest_exact<C>(self, inner: C) -> NestExact<Self, C::IntoCollector>
    where
        Self: Collector<C::Output> + Sized,
        C: IntoCollectorBase<IntoCollector: Clone>,
    {
        assert_collector_base(NestExact::new(self, inner.into_collector()))
    }

    /// Creates a collector that feeds every item in the first collector until it stops accumulating,
    /// then creates a second collector from the output of the first collector
    /// and continues feeding the rest of the items into the second one.
    ///
    /// The output is the output of the first collector if it is not full,
    /// otherwise the output of the second collector.
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori::{prelude::*, iter::Fold};
    /// use std::{collections::BinaryHeap, cmp::Reverse};
    ///
    /// fn top_k<C: IntoCollector<i32>>(
    ///     k: usize,
    ///     other: C,
    /// ) -> impl Collector<i32, Output = (Vec<i32>, Option<C::Output>)> {
    ///     Vec::with_capacity(k + 1) // One extra element for `Fold`
    ///         .into_collector()
    ///         .map(Reverse)
    ///         .take(k)
    ///         .map_output(|tops| (BinaryHeap::from(tops), None))
    ///         .then(|(tops, _)| {
    ///             Fold::new(tops, {
    ///                 // The compiler is stupid so we get to write like this
    ///                 let f = |tops: &mut BinaryHeap<_>, num: &mut _| {
    ///                     tops.push(Reverse(*num));
    ///                     *num = tops.pop().unwrap().0;
    ///                 };
    ///                 f
    ///             })
    ///             .tee_funnel(other.into_collector().map_output(Some))
    ///         })
    ///         .map_output(move |(mut tops, other)| (
    ///             std::iter::from_fn(|| tops.pop())
    ///                 .map(|top| top.0)
    ///                 .feed_into(Vec::with_capacity(k)),
    ///             other,
    ///         ))
    /// }
    ///
    /// let mut top3_and_sum_other = top_k(3, vec![]);
    /// assert!(top3_and_sum_other.collect_many([2, 7]).is_continue());
    /// assert_eq!(top3_and_sum_other.finish(), (vec![2, 7], None));
    ///
    /// let mut top3_and_sum_other = top_k(3, 0.into_sum());
    /// assert!(top3_and_sum_other.collect_many([2, 7, 3, 4, 8]).is_continue());
    /// assert_eq!(top3_and_sum_other.finish(), (vec![4, 7, 8], Some(5)));
    /// ```
    #[cfg(feature = "unstable")]
    #[inline]
    fn then<F, C>(self, f: F) -> Then<Self, C::IntoCollector, F>
    where
        Self: Sized,
        C: IntoCollectorBase<Output = Self::Output>,
        F: FnOnce(Self::Output) -> C,
    {
        assert_collector_base(Then::new(self, f))
    }
}

// `Output` shouldn't be required to be specified.
fn _dyn_compatible<O>(_: &mut dyn CollectorBase<Output = O>) {}

// You actually read this? So here's a workaround for issues
// when you can't even name the type (e.g. closures, async blocks).
#[cfg(feature = "std")]
fn _unnamed_type_workaround() {
    use alloc::vec;

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
