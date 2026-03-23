use std::marker::PhantomData;
use std::ops::ControlFlow;

use komadori::prelude::*;

use super::plumbing::{Consumer, DefineConsumer};
use super::{Fuse, IntoParallelCollectorBase, Tee};

///
pub trait ParallelCollectorBase: for<'this> DefineConsumer<'this> {
    ///
    type Output;

    ///
    fn finish(self) -> Self::Output;

    ///
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

    ///
    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    /// For the reason why [`PhantomData`] is needed in the closure,
    /// see [here](https://doc.rust-lang.org/error_codes/E0582.html).
    fn with_consumer<R>(
        self,
        len: usize,
        f: impl for<'a> FnOnce(
            usize,
            <Self as DefineConsumer<'a>>::Consumer,
            PhantomData<&'a ()>,
        ) -> (
            R,
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ),
    ) -> (R, Self::Output) {
        let mut this = self;
        let (actual_len, consumer, committer) = this.parts(len);
        let (ret, output) = f(actual_len, consumer, PhantomData);
        committer(output);
        (ret, this.finish())
    }

    ///
    #[inline]
    fn fuse(self) -> Fuse<Self>
    where
        Self: Sized,
    {
        Fuse::new(self)
    }

    ///
    #[inline]
    fn tee<C>(self, other: C) -> Tee<Self, C::IntoParCollector>
    where
        Self: Sized,
        C: IntoParallelCollectorBase,
    {
        Tee::new(self, other.into_par_collector())
    }
}

///
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
