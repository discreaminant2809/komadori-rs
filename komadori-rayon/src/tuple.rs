/// Tuples
pub trait Tuple {}

#[allow(dead_code)] // FIXME: will be used in `nest_serial`
/// Tuples that can append one more type at the start.
pub trait PushFrontTuple: Tuple {
    type PushFront<T>: Tuple;

    fn push_front<T>(self, item: T) -> Self::PushFront<T>;
}

/// Tuples that can append two more types at the start.
pub trait PushFront2Tuple: Tuple {
    type PushFront2<T0, T1>: Tuple;

    fn push_front2<T0, T1>(self, item0: T0, item1: T1) -> Self::PushFront2<T0, T1>;
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

macro_rules! push_front2_tuple_impl {
    ($($T:ident)*) => {
        impl<$($T,)*> PushFront2Tuple for ($($T,)*) {
            type PushFront2<T, U> = (T, U, $($T,)*);

            #[allow(non_snake_case)]
            fn push_front2<T, U>(self, item0: T, item1: U) -> Self::PushFront2<T, U> {
                let ($($T,)*) = self;
                (item0, item1, $($T,)*)
            }
        }
    };
}
push_front2_tuple_impl!();
push_front2_tuple_impl!(T0);
push_front2_tuple_impl!(T0 T1);
// Add more if we need more
