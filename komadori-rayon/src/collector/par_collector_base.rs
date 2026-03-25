use std::ops::ControlFlow;

use komadori::prelude::*;

use super::plumbing::{Consumer, DefineConsumer};
use super::{Fuse, IntoParallelCollectorBase, Tee, assert_par_collector_base};

/// An indexed parallel collector.
///
/// This trait also defines the output and the way to finish and return that output.
pub trait ParallelCollectorBase: for<'this> DefineConsumer<'this> {
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
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
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
    /// [`filter()`]: Self::filter
    /// [`take_any_while()`]: Self::take_any_while
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
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(<<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output),
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
    /// [`Continue(())`]: ControlFlow::Continue
    /// [`Break(())`]: ControlFlow::Break
    #[inline]
    fn fuse(self) -> Fuse<Self>
    where
        Self: Sized,
    {
        assert_par_collector_base(Fuse::new(self))
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
    #[inline]
    fn tee<C>(self, other: C) -> Tee<Self, C::IntoParCollector>
    where
        Self: Sized,
        C: IntoParallelCollectorBase,
    {
        assert_par_collector_base(Tee::new(self, other.into_par_collector()))
    }
}

/// Defines what item types are collected in an indexed parallel collector.
///
/// You cannot implement this trait directly. You should instead define the item type
/// of serial collectors produced by consumers of this parallel collector.
pub trait ParallelCollector<T>: ParallelCollectorBase<Consumer: Consumer<T>> {}
impl<C, T> ParallelCollector<T> for C where C: ParallelCollectorBase<Consumer: Consumer<T>> {}

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
