use crate::tuple::Tuple;

/// [`FnOnce`], but can be a concrete type.
pub trait CallOnce<Args: Tuple> {
    type Output;

    fn call_once(self, args: Args) -> Self::Output;
}

/// [`FnMut`], but can be a concrete type.
pub trait CallMut<Args: Tuple>: CallOnce<Args> {
    fn call_mut(&mut self, args: Args) -> Self::Output;
}

/// [`Fn`], but can be a concrete type.
pub trait Call<Args: Tuple>: CallMut<Args> {
    fn call(&self, args: Args) -> Self::Output;
}

// We can't do blanket implementations like `&Call: CallOnce`
// because of conflicting implementation.

macro_rules! tuple_impl {
    ($($T:ident)*) => {
        impl<F, R, $($T,)*> CallOnce<($($T,)*)> for F
        where
            F: FnOnce($($T,)*) -> R
        {
            type Output = R;

            #[allow(non_snake_case)]
            fn call_once(self, ($($T,)*): ($($T,)*)) -> Self::Output {
                self($($T,)*)
            }
        }

        impl<F, R, $($T,)*> CallMut<($($T,)*)> for F
        where
            F: FnMut($($T,)*) -> R
        {
            #[allow(non_snake_case)]
            fn call_mut(&mut self, ($($T,)*): ($($T,)*)) -> Self::Output {
                self($($T,)*)
            }
        }

        impl<F, R, $($T,)*> Call<($($T,)*)> for F
        where
            F: Fn($($T,)*) -> R
        {
            #[allow(non_snake_case)]
            fn call(&self, ($($T,)*): ($($T,)*)) -> Self::Output {
                self($($T,)*)
            }
        }
    };
}

tuple_impl!();
tuple_impl!(T0);
tuple_impl!(T0 T1);
tuple_impl!(T0 T1 T2);
tuple_impl!(T0 T1 T2 T3);
// Add more if we need more

// I got the below bug so I have to go with such workaround (that I used Fn instead of Call):
// ```
// error[E0119]: conflicting implementations of trait `ops::par_fn::DefineCallable<'_, _>` for type `ops::with_state_fn::ParClosureWithStateFn<_, _>`
//   --> komadori-rayon/src/ops/with_state_fn.rs:20:1
//    |
// 20 | / impl<'a, FS, F, Args> DefineCallable<'a, Args> for ParClosureWithStateFn<FS, F>
// 21 | | where
// 22 | |     FS: Call<()> + Sync,
// 23 | |     F: Call<(FS::Output,)> + Sync,
//    | |__________________________________^ conflicting implementation for `ops::with_state_fn::ParClosureWithStateFn<_, _>`
//    |
//   ::: komadori-rayon/src/ops/call.rs:70:1
//    |
// 70 | / impl<'a, F, Args> DefineCallable<'a, Args> for F
// 71 | | where
// 72 | |     Args: Tuple,
// 73 | |     F: Call<Args> + Sync,
// 74 | |     &'a F: CallOnce<Args, Output = F::Output> + Send,
//    | |_____________________________________________________- first implementation here
//    |
//    = note: downstream crates may implement trait `ops::call::CallOnce<_>` for type `ops::with_state_fn::ParClosureWithStateFn<_, _>`
//    = note: downstream crates may implement trait `ops::call::CallMut<_>` for type `ops::with_state_fn::ParClosureWithStateFn<_, _>`
//    = note: downstream crates may implement trait `ops::call::Call<_>` for type `ops::with_state_fn::ParClosureWithStateFn<_, _>`
//    = note: downstream crates may implement trait `ops::call::CallOnce<_>` for type `&ops::with_state_fn::ParClosureWithStateFn<_, _>`
// ```

// impl<'a, F, R> DefineCallOnce<'a, ()> for F
// where
//     F: Fn() -> R + Sync,
// {
//     type Output = R;

//     type CallOnce = &'a F;
// }
