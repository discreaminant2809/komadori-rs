//!

use std::ops::ControlFlow;

use komadori::prelude::*;

///
pub trait ConsumerBase: IntoCollectorBase<Output: Send> + Send + Sized {
    ///
    type Combiner: Combiner<Self::Output>;

    ///
    fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner);

    ///
    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}

///
pub trait UnindexedConsumerBase: ConsumerBase {
    ///
    fn split_off_left(&self) -> Self;

    ///
    fn to_combiner(&self) -> Self::Combiner;
}

///
pub trait Combiner<O> {
    ///
    fn combine(self, left: &mut O, right: O);
}

///
pub trait Consumer<T>: ConsumerBase<IntoCollector: Collector<T>> {}
impl<C, T> Consumer<T> for C where C: ConsumerBase<IntoCollector: Collector<T>> {}

///
pub trait UnindexedConsumer<T>: UnindexedConsumerBase<IntoCollector: Collector<T>> {}
impl<C, T> UnindexedConsumer<T> for C where C: UnindexedConsumerBase<IntoCollector: Collector<T>> {}

///
pub trait ConsumerFnOnce<T> {
    ///
    type Output;

    ///
    fn call_once<C>(self, actual_len: Option<usize>, consumer: C) -> (Self::Output, C::Output)
    where
        C: Consumer<T>;
}

///
pub trait UnindexedConsumerFnOnce<T> {
    ///
    type Output;

    ///
    fn call_once<C>(self, consumer: C) -> (Self::Output, C::Output)
    where
        C: UnindexedConsumer<T>;
}

// ///
// #[macro_export]
// macro_rules! consumer_fn_once {
//     (
//         for<
//             $($lts:lifetime$(: $lt_bounds:lifetime $(+ $more_lt_bounds:lifetime)* $(+)?)?),*
//             $(, $Ts:ident$(: $(($T_bounds:path) $(+ ($more_T_bounds:path))* $(+)?)?)?)*
//             $(,)?
//         >
//         |
//             $FnName:ident { $($fields:ident: ($field_inits:expr) as $field_tys:ty),* $(,)? },
//             $actual_len_ident:ident,
//             $consumer_ident:ident: impl Consumer<$ItemT:ident> $(,)?
//         | -> ($Output:ty, _) {
//             $($stmts:stmt)*
//         }
//     ) => {
//         struct $FnName<
//             $($lts$(: $lt_bounds $(+ $more_lt_bounds)*)?),*
//             $(, $Ts$(: $($T_bounds $(+ $more_T_bounds)*)?)?)*
//         > {
//             $($fields: $field_tys),*
//         }

//         impl<
//             $($lts$(: $lt_bounds $(+ $more_lt_bounds)*)?),*
//             $(, $Ts$(: $($T_bounds $(+ $more_T_bounds)*)?)?)*
//         > $crate::collector::plumbing::ConsumerFnOnce<$ItemT> for $FnName<
//             $($lts),*
//             $(, $Ts)*
//         > {
//             type Output = $Output;

//             // For some reason clippy warns every single semicolon
//             #[allow(redundant_semicolons)]
//             fn call_once<__Consumer__>(
//                 self, $actual_len_ident: Option<usize>, $consumer_ident: __Consumer__
//             ) -> (Self::Output, __Consumer__::Output)
//             where
//                 __Consumer__: $crate::collector::plumbing::Consumer<$ItemT>
//             {
//                 $($stmts)*
//             }
//         }
//     };

//     (
//         for<
//             $($lts:lifetime$(: $lt_bounds:lifetime $(+ $more_lt_bounds:lifetime)* $(+)?)?),*
//             $(, $Ts:ident$(: $(($T_bounds:path) $(+ ($more_T_bounds:path))* $(+)?)?)?)*
//             $(,)?
//         >
//         |
//             $FnName:ident { $($fields:ident: ($field_inits:expr) as $field_tys:ty),* $(,)? },
//             $actual_len_ident:ident,
//             $consumer_ident:ident $(,)?
//         | -> ($Output:ty, _) {
//             $($stmts:stmt)*
//         }
//     ) => {{
//         struct $FnName<
//             $($lts$(: $lt_bounds $(+ $more_lt_bounds)*)?),*
//             $(, $Ts$(: $($T_bounds $(+ $more_T_bounds)*)?)?)*
//         > {
//             $($fields: $field_tys),*
//         }

//         impl<
//             $($lts$(: $lt_bounds $(+ $more_lt_bounds)*)?,)*
//             __ItemType__
//             $(, $Ts$(: $($T_bounds $(+ $more_T_bounds)*)?)?)*
//         > $crate::collector::plumbing::ConsumerFnOnce<__ItemType__> for $FnName<
//             $($lts),*
//             $(, $Ts)*
//         > {
//             type Output = $Output;

//             // For some reason clippy warns every single semicolon
//             #[allow(redundant_semicolons)]
//             fn call_once<__Consumer__>(
//                 self, $actual_len_ident: Option<usize>, $consumer_ident: __Consumer__
//             ) -> (Self::Output, __Consumer__::Output)
//             where
//                 __Consumer__: $crate::collector::plumbing::Consumer<__ItemType__>
//             {
//                 $($stmts)*
//             }
//         }

//         $FnName {
//             $($fields: $field_inits),*
//         }
//     }};
// }

// consumer_fn_once!(
//     for<'c, 'a: 'b + 'c, 'b: 'c, T: (::core::Path<T, 2>) + (ASJ)> |MyFn { a: (2) as Foo<T> },
//                                                                    actual_len2,
//                                                                    consumer2|
//                                                                    -> i32 {
//         let _ = 2;
//         (3, consumer2.into_collector().finish())
//     }
// );

fn _unindexed_substitutable_to_indexed<T>(consumer: impl UnindexedConsumer<T>) {
    fn check<T>(_consumer: impl Consumer<T>) {}
    check::<T>(consumer);
}
