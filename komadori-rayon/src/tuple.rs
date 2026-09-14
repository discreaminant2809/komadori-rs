//! Parallel collectors for tuples.

mod into_collector;

pub use into_collector::*;

/// Tuples
pub(crate) trait Tuple {}

#[allow(dead_code)] // FIXME: will be used in `nest_serial`
/// Tuples that can append one more type at the start.
pub(crate) trait PushFrontTuple: Tuple {
    type PushFront<T>: Tuple;

    fn push_front<T>(self, item: T) -> Self::PushFront<T>;
}

macro_rules! tuple_impl {
    ($($T:ident)*) => {
        impl<$($T,)*> Tuple for ($($T,)*) {}
    };
}
tuple_impl!();
tuple_impl!(T0);
tuple_impl!(T0 T1);
tuple_impl!(T0 T1 T2);
tuple_impl!(T0 T1 T2 T3);
// Add more if we need more

macro_rules! push_front_tuple_impl {
    ($($T:ident)*) => {
        impl<$($T,)*> PushFrontTuple for ($($T,)*) {
            type PushFront<T> = (T, $($T,)*);

            #[allow(non_snake_case)]
            fn push_front<T>(self, item: T) -> Self::PushFront<T> {
                let ($($T,)*) = self;
                (item, $($T,)*)
            }
        }
    };
}
push_front_tuple_impl!();
push_front_tuple_impl!(T0);
push_front_tuple_impl!(T0 T1);
push_front_tuple_impl!(T0 T1 T2);
// Add more if we need more
