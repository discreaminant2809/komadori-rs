use std::ops::ControlFlow;

use komadori::prelude::*;

use super::plumbing::{Consumer, DefineSerial};
use super::{
    Fuse, IntoCollector, IntoParallelCollectorBase, Map, MapOutput, MapWith, Take, Tee, TeeClone, TeeFunnel,
    TeeMut, assert_par_collector, assert_par_collector_base, tee, tee_clone, tee_funnel, tee_mut,
};

/// An (indexed) parallel collector.
///
/// This trait also defines the output and the way to finish and return that output.
pub trait ParallelCollectorBase: for<'this> DefineSerial<'this> {
    /// The result this collector yields, via the
    /// [`finish()`](Self::finish) method.
    ///
    /// This assosciated type does not appear in trait objects.
    type Output;

    /// Consumes the collector and returns the accumulated result.
    fn finish(self) -> Self::Output;

    /// Reserves for `len` items and returns "parts" needed
    /// to drive this parallel collector.
    #[allow(clippy::type_complexity)]
    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    );

    /// Returns a hint whether this parallel collector has stopped accumulating.
    ///
    /// Returns [`Break(())`] if it is guaranteed that the collector
    /// has stopped accumulating, or returns [`Continue(())`] otherwise.
    ///
    /// As specified in the [module-level documentation](crate::collector),
    /// after the stop is signaled somewhere else (e.g. from committers),
    /// the behavior of this method is unspecified.
    /// This may include returning [`Continue(())`] even if the collector has conceptually stopped.
    ///
    /// This method should be called once and only once before using.
    /// It is not intended for repeatedly checking whether the
    /// parallel collector has stopped. Use [`fuse()`](Self::fuse)
    /// if you find yourself needing such behavior.
    ///
    /// If the collector is uncertain, like "maybe I won’t accumulate… uh, fine, I will,"
    /// it is recommended to just return [`Continue(())`].
    /// For example, [`filter()`] might skip some items it collects,
    /// but still returns [`Continue(())`] as long as the underlying collector can still accumulate.
    /// The filter just denies "undesirable" items and does not signal termination
    /// (this is the job of [`take_any_while()`] instead).
    ///
    /// The default implementation always returns [`Continue(())`].
    ///
    /// [`Break(())`]: ControlFlow::Break
    /// [`Continue(())`]: ControlFlow::Continue
    /// [`filter()`]: super::UnindexedParallelCollectorBase::filter
    /// [`take_any_while()`]: super::UnindexedParallelCollectorBase::take_any_while
    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    /// Reserves for `len` items and returns "parts" needed
    /// to drive this parallel collector.
    ///
    /// This method effectively "consumes" the collector.
    /// After calling this method, the collector is counted
    /// to have returned [`Break(())`](ControlFlow::Break)
    /// and the only valid method to call is [`finish()`](Self::finish).
    /// The behavior is unspecified if you call other methods than that method,
    /// including panicking or incorrect results.
    /// You can leverage it by "consuming" some states instead of cloning them
    /// for more efficiency.
    ///
    /// Most parallel collectors do not care whether they can
    /// optimize anything by consuming some states
    /// (and hence this method is not required to override),
    /// but if it is the case or you are implementing an adapter,
    /// you should override this method.
    ///
    /// The signature is similar to [`parts()`](Self::parts),
    /// except the returning function which does not return
    /// a [`ControlFlow`].
    #[allow(clippy::type_complexity)]
    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        let (actual_len, consumer, commit) = self.parts(len);
        (actual_len, consumer, |output| {
            let _ = commit(output);
        })
    }

    /// Create a parallel collector that can "safely" collect even after
    /// the underlying collector has stopped accumulating,
    /// without triggering undesired behaviors.
    ///
    /// Normally, a collector having stopped may behave unpredictably,
    /// including accumulating again.
    /// `fuse()` ensures that once a collector has stopped, subsequent items
    /// are guaranteed to **not** be accumulated. This means that at that point:
    ///
    /// - [`break_hint()`](Self::break_hint) is guaranteed to return [`Break(())`].
    ///
    /// - Collectors obtained from consumers are fused.
    ///   (See [`CollectorBase::fuse()`] for more information)
    ///
    /// However, `fuse()` does **not** protect you from behaviors happens
    /// after calling [`take_parts()`](Self::take_parts) and
    /// [`take_parts_unindexed()`](super::UnindexedParallelCollectorBase::take_parts_unindexed).
    /// You still have to consider such collectors as being "taken" and only
    /// call [`finish()`](Self::finish) on them.
    ///
    /// # Examples
    ///
    /// Coming soon!
    ///
    /// [`Continue(())`]: ControlFlow::Continue
    /// [`Break(())`]: ControlFlow::Break
    #[inline]
    fn fuse(self) -> Fuse<Self>
    where
        Self: Sized,
    {
        assert_par_collector_base(Fuse::new(self))
    }

    /// Creates a parallel collector that stops accumulating after collecting `n` items,
    /// or fewer if the underlying collector stops sooner.
    ///
    /// When being fed a batch of `m` items and `m < n`
    /// (where `n` is the remaining amount of items),
    /// `take(n)` takes all `m` them and subtracts `m` to `n`.
    /// Otherwise:
    ///
    /// - In the indexed path, `take(n)` only accumulates `n` first items
    ///   in the batch (counted from the start of the batch sequentially),
    ///   and drops the rest.
    ///
    /// - In the unindexed path, `take(n)` only accumulates `n` items
    ///   *anywhere* in the batch, and drops the rest.
    ///
    ///   **Note**: By the current implementations, in this path,
    ///   `take(n)` *may* collect slightly more items than it needs,
    ///   but it still guarantees to only let the underlying collector
    ///   collects only `n` items. The overreached items are currently
    ///   simply dropped.
    ///
    /// Conceptually, `take(n)` behaves like both of `rayon`'s
    /// [`take(n)`](rayon::iter::IndexedParallelIterator::take) and
    /// [`take_any(n)`](rayon::iter::ParallelIterator::take_any),
    /// depending on whether this is used in the indexed path
    /// or the unindexed path, respectively. However, in the indexed path,
    /// `take(n)` may (but not always) cause uneven split because parallel collectors
    /// in this crate is nearly unaware of what feeds them.
    /// It could be less a problem when the adapter is used alone without
    /// teeing with other parallel collectors, however.
    /// If uneven splitting causes problems in your case, consider
    /// constraining your batches of items
    /// using the returned `usize` in [`parts()`](Self::parts),
    /// or actively constraining `n` items in the upstream
    /// and avoiding this adapter if not being teed with anything else.
    /// For `rayon` specifically,
    /// [`feed_into_indexed()`](crate::iter::RayonParallelIteratorExt::feed_into_indexed)
    /// helps migrate this problem also.
    ///
    /// # Examples
    ///
    /// Indexed path:
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    ///
    /// let nums = [1, 2, 3, 4, 5]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .take(3)
    ///     );
    ///
    /// assert_eq!(nums.len(), 3);
    /// assert_eq!(nums, [1, 2, 3]);
    /// ```
    ///
    /// Unindexed path:
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    /// use std::collections::HashSet;
    ///
    /// let nums = (1..=10)
    ///     .into_par_iter()
    ///     .filter(|&x| x % 2 == 0)
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .take(3)
    ///     );
    ///
    /// assert_eq!(nums.len(), 3);
    /// assert!(
    ///     HashSet::from_iter(nums)
    ///         .is_subset(&HashSet::from([2, 4, 6, 8, 10]))
    /// );
    /// ```
    #[inline]
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        assert_par_collector_base(Take::new(self, n))
    }

    /// Creates a parallel collector that lets both collectors collect the same item.
    ///
    /// For each item collected, the first collector collects the item
    /// copied with the [`Copy`] trait before the second collector collects it.
    ///
    /// `tee()` only stops when **both** collectors have stopped.
    ///
    /// If the item type of this adapter is `T`, both collectors must be able to
    /// collect `T`, and `T` must implement [`Copy`].
    ///
    /// The [`Output`](Self::Output) is a tuple containing the outputs of
    /// both underlying collectors, in order.
    ///
    /// See the [module-level documentation](crate::collector) for
    /// when this adapter is used and other variants of `tee` adapters.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::{prelude::*, cmp::ParMax};
    ///
    /// let (sum, max) = [1, 2, 4, 5, 3]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         0i32.into_par_sum()
    ///             .tee(ParMax::new())
    ///     );
    ///
    /// assert_eq!(sum, 15);
    /// assert_eq!(max, Some(5));
    /// ```
    #[inline]
    fn tee<C>(self, other: C) -> Tee<Self, C::IntoParCollector>
    where
        Self: Sized,
        C: IntoParallelCollectorBase,
    {
        assert_par_collector_base(tee(self, other.into_par_collector()))
    }

    /// Creates a parallel collector that lets both collectors collect the same item.
    ///
    /// For each item collected, the first collector collects the item
    /// cloned with the [`Clone`] trait before the second collector collects it.
    /// If one of them has stopped, the implementation will **not** clone
    /// the item, and will instead feed it into the other for optimization.
    ///
    /// `tee_clone()` only stops when **both** collectors have stopped.
    ///
    /// If the item type of this adapter is `T`, both collectors must implement
    /// [`ParallelCollector<T>`](super::ParallelCollector), and `T` must implement [`Clone`].
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
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    /// use std::sync::Arc;
    ///
    /// let (nums1, nums2) = [1, 2, 3]
    ///     .into_par_iter()
    ///     .map(Arc::new)
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .take(2)
    ///             .tee_clone(vec![])
    ///     );
    ///
    /// assert!(nums1.iter().map(|num| **num).eq([1, 2]));
    /// assert!(nums2.iter().map(|num| **num).eq([1, 2, 3]));
    /// assert!(nums2.iter().map(Arc::strong_count).eq([2, 2, 1]));
    /// ```
    #[inline]
    fn tee_clone<C>(self, other: C) -> TeeClone<Self, C::IntoParCollector>
    where
        Self: Sized,
        C: IntoParallelCollectorBase,
    {
        assert_par_collector_base(tee_clone(self, other.into_par_collector()))
    }

    /// Creates a parallel collector that lets both collectors collect the same item.
    ///
    /// For each item collected, the first collector collects
    /// the mutable reference of the item before the second collector also
    /// collects the mutable reference of it.
    ///
    /// `tee_mut()` only stops when **both** collectors have stopped.
    ///
    /// If the item type of this adapter is `&'i mut T`,
    /// the first collector must implement
    /// [`for<'a> ParallelCollector<&'a mut T>`](super::ParallelCollector)
    /// (a collector that can collect a mutable reference with any lifetime),
    /// and the second collector must implement
    /// [`ParallelCollector<&'i mut T>`](super::ParallelCollector).
    ///
    /// The [`Output`](CollectorBase::Output) is a tuple containing the outputs of
    /// both underlying collectors, in order.
    ///
    /// See the [module-level documentation](crate::collector) for
    /// when this adapter is used and other variants of `tee` adapters.
    ///
    /// # Examples
    ///
    /// Coming soon!
    // FIXED: Blocker: Parallel concatenation.
    // /// ```
    // /// use rayon::*;
    // /// use komadori_rayon::{prelude::*, cmp::Max, clb_mut};
    // ///
    // /// let ((concat, max_len), string_vec) = ["noble", "and", "singer"]
    // ///     .map(String::from)
    // ///     .into_par_iter()
    // ///     .feed_into(
    // ///         String::new()
    // ///             .into_concat()
    // ///             .map(clb_mut!(|s: &mut String| -> &str { &s[..] }))
    // ///             .tee_funnel(vec![])
    // ///     );
    // ///
    // /// assert_eq!(concat, "nobleandsinger");
    // /// assert_eq!(string_vec, ["noble", "and", "singer"]);
    // /// ```
    #[inline]
    fn tee_mut<C>(self, other: C) -> TeeMut<Self, C::IntoParCollector>
    where
        Self: Sized,
        C: IntoParallelCollectorBase,
    {
        assert_par_collector_base(tee_mut(self, other.into_par_collector()))
    }

    /// Creates a parallel collector that lets both collectors collect the same item.
    ///
    /// For each item collected, the first collector collects
    /// the mutable reference of the item before the second collector collects it.
    ///
    /// `tee_funnel()` only stops when **both** collectors have stopped.
    ///
    /// If the item type of this adapter is `T`,
    /// the first collector must implement
    /// [`for<'a> ParallelCollector<&'a mut T>`](super::ParallelCollector)
    /// (a collector that can collect a mutable reference with any lifetime),
    /// and the second collector must implement [`ParallelCollector<T>`](super::ParallelCollector).
    ///
    /// The [`Output`](CollectorBase::Output) is a tuple containing the outputs of
    /// both underlying collectors, in order.
    ///
    /// See the [module-level documentation](crate::collector) for
    /// when this adapter is used and other variants of `tee` adapters.
    ///
    /// # Examples
    ///
    /// Coming soon!
    // FIXED: Blocker: Parallel concatenation.
    // /// ```
    // /// use rayon::*;
    // /// use komadori_rayon::{prelude::*, cmp::Max, clb_mut};
    // ///
    // /// let ((concat, max_len), string_vec) = ["noble", "and", "singer"]
    // ///     .map(String::from)
    // ///     .into_par_iter()
    // ///     .feed_into(
    // ///         String::new()
    // ///             .into_concat()
    // ///             .map(clb_mut!(|s: &mut String| -> &str { &s[..] }))
    // ///             .tee_funnel(vec![])
    // ///     );
    // ///
    // /// assert_eq!(concat, "nobleandsinger");
    // /// assert_eq!(string_vec, ["noble", "and", "singer"]);
    // /// ```
    #[inline]
    fn tee_funnel<C>(self, other: C) -> TeeFunnel<Self, C::IntoParCollector>
    where
        Self: Sized,
        C: IntoParallelCollectorBase,
    {
        assert_par_collector_base(tee_funnel(self, other.into_par_collector()))
    }

    /// Creates a parallel collector that calls a closure on each item before collecting.
    ///
    /// This is used when you need a parallel collector that collects `U`,
    /// but you have a parallel collector that collects `T`. In that case,
    /// you can use `map()` to transform `U` into `T` before passing it along.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    ///
    /// let squares = (1..=5)
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .map(|num| num * num)
    ///     );
    ///
    /// assert_eq!(squares, [1, 4, 9, 16, 25]);
    /// ```
    #[inline]
    fn map<F, T, U>(self, f: F) -> Map<Self, F>
    where
        Self: ParallelCollector<T> + Sized,
        F: Fn(U) -> T + Sync,
    {
        assert_par_collector::<_, U>(Map::new(self, f))
    }

    /// Same as [`map()`](Self::map), but with a state that will either be cloned
    /// or created from a factory (or both) to each serial execution.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    /// use komadori::prelude::*;
    /// use std::sync::mpsc::channel;
    ///
    /// let (sender, receiver) = channel();
    ///
    /// let digit_counts = [1_u32, 2_000_000_000, 3_000, 4_000_000]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .map_with(
    ///                 sender, String::new,
    ///                 |sender, buf, num| {
    ///                     // I know, this is not an efficient way to
    ///                     // count the number of digits.
    ///                     // This is just an example.
    ///                     buf.clear();
    ///                     use std::fmt::Write;
    ///                     write!(buf, "{num}");
    ///
    ///                     sender.send(num).unwrap();
    ///                     buf.len()
    ///                 },
    ///             ),
    ///     );
    ///
    /// let mut sum = receiver.iter().feed_into(0_u32.into_sum());
    ///
    /// assert_eq!(digit_counts, [1, 10, 4, 7]);
    /// assert_eq!(sum, 2_004_003_001);
    /// ```
    #[inline]
    fn map_with<L1, FL2, L2, F, T, U>(self, local1: L1, local2_f: FL2, f: F) -> MapWith<Self, L1, FL2, F>
    where
        Self: ParallelCollector<T> + Sized,
        L1: Clone + Send,
        FL2: Fn() -> L2 + Sync,
        F: Fn(&mut L1, &mut L2, U) -> T + Sync,
    {
        assert_par_collector::<_, U>(MapWith::new(self, local1, local2_f, f))
    }

    /// Creates a parallel collector that transforms the final accumulated result.
    ///
    /// This is used when your output gets "ugly" after a chain of adaptors,
    /// or when you do not want to break your API by (accidentally) rearranging adaptors,
    /// or when you just want a different output type for your collector.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::{prelude::*, iter::ParCount};
    ///
    /// let avg = [1, 6, 4, 2]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         0_i32.into_par_sum()
    ///             .tee(ParCount::new())
    ///             .map_output(|(sum, count)| {
    ///                 (count != 0).then(|| sum as f64 / count as f64)
    ///             })
    ///     );
    ///
    /// assert_eq!(avg, Some(3.25));
    /// ```
    #[inline]
    fn map_output<F, R>(self, f: F) -> MapOutput<Self, F>
    where
        Self: Sized,
        F: FnOnce(Self::Output) -> R,
    {
        assert_par_collector_base(MapOutput::new(self, f))
    }

    /// Creates a (serial) collector from a parallel collector.
    ///
    /// It is a method of this trait instead of implementing
    /// [`IntoCollector`] because of the orphan rule,
    /// and the danger of implicit conversion
    /// (may accidentally downgrade to serial execution without knowing).
    ///
    /// A type should not be **both** a serial and parallel collectors,
    /// since it would be a clash between this method and tbe same method
    /// in [`IntoCollector`].
    ///
    /// # Examples
    ///
    /// ```
    /// use komadori_rayon::prelude::*;
    /// use komadori::prelude::*;
    ///
    /// let mut collector = vec![]
    ///     .into_par_collector()
    ///     .take(3)
    ///     .into_collector();
    ///
    /// // Use as a normal (serial) collector!
    /// assert!(collector.break_hint().is_continue());
    /// assert!(collector.collect(1).is_continue());
    /// assert!(collector.collect(2).is_continue());
    /// assert!(collector.collect(3).is_break());
    ///
    /// assert_eq!(collector.finish(), [1, 2, 3]);
    /// ```
    ///
    /// [`IntoCollector`]: komadori::collector::IntoCollector
    #[inline]
    fn into_collector(self) -> IntoCollector<Self>
    where
        Self: Sized,
    {
        IntoCollector::new(self)
    }
}

/// Defines what item types are collected in an indexed parallel collector.
///
/// You cannot implement this trait directly. You should instead define the item type
/// of serial collectors produced by consumers of this parallel collector.
pub trait ParallelCollector<T>: ParallelCollectorBase<Serial: Collector<T>> {}
impl<C, T> ParallelCollector<T> for C where C: ParallelCollectorBase<Serial: Collector<T>> {}

// For anyone wanna do this:
// ```
// fn with_consumer<R>(
//     self,
//     len: usize,
//     f: impl for<'a> FnOnce(
//         usize,
//         <Self as DefineConsumer<'a>>::Consumer,
//         PhantomData<&'a ()>,
//     ) -> (
//         R,
//         <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
//     ),
// ) -> (R, Self::Output) {
//     let mut this = self;
//     let (actual_len, consumer, committer) = this.parts(len);
//     let (ret, output) = f(actual_len, consumer, PhantomData);
//     committer(output);
//     (ret, this.finish())
// }
// ```
//
// It doesn't work. Here's why (paste it to `tee_base.rs`):
//
// ```
// fn for2_f<'c1, C1, C2, TF, R>(
//     actual_len1: usize,
//     consumer1: <Fuse<C1> as DefineConsumer<'c1>>::Consumer,
//     teer: TF,
//     f: impl for<'a> FnOnce(
//         usize,
//         __adapter_tee_internal::Consumer<
//             <Fuse<C1> as DefineConsumer<'a>>::Consumer,
//             <Fuse<C2> as DefineConsumer<'a>>::Consumer,
//             TF,
//         >,
//         PhantomData<&'a ()>,
//     ) -> (
//         R,
//         <__adapter_tee_internal::Consumer<
//             <Fuse<C1> as DefineConsumer<'a>>::Consumer,
//             <Fuse<C2> as DefineConsumer<'a>>::Consumer,
//             TF,
//         > as IntoCollectorBase>::Output,
//     ),
// ) -> impl for<'a> FnOnce(
//     usize,
//     <Fuse<C2> as DefineConsumer<'a>>::Consumer,
//     PhantomData<&'a ()>,
// ) -> (
//     (
//         R,
//         <<Fuse<C1> as DefineConsumer<'c1>>::Consumer as IntoCollectorBase>::Output,
//     ),
//     <<Fuse<C2> as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
// )
// where
//     C1: ParallelCollectorBase,
//     C2: ParallelCollectorBase,
//     TF: Send + Clone,
// {
//     for<'c2> move |actual_len2: usize,
//                    consumer2: <Fuse<C2> as DefineConsumer<'c2>>::Consumer,
//                    _: PhantomData<&'c2 ()>|
//                    -> (
//         (
//             R,
//             <<Fuse<C1> as DefineConsumer<'c1>>::Consumer as IntoCollectorBase>::Output,
//         ),
//         <<Fuse<C2> as DefineConsumer<'c2>>::Consumer as IntoCollectorBase>::Output,
//     ) {
//         let (ret, (output1, output2)) = f(
//             actual_len1.max(actual_len2),
//             __adapter_tee_internal::Consumer::new(consumer1, consumer2, teer.clone()),
//             PhantomData,
//         );
//
//         // `output1` now has a lifetime of `'c2`, not `'c1`.
//         // Any meaningful fix? No.[[[[[[[[[[]]]]]]]]]]
//         ((ret, output1), output2)
//     }
// }
// ```
