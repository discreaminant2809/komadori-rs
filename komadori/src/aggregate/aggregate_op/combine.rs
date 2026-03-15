use std::marker::PhantomData;

use crate::aggregate::{AggregateOp, RefAggregateOp, assert_op};

/// An [`AggregateOp`] that combines multiple aggregate ops into a single one.
///
/// In a tuple of [`AggregateOp`]s, every op except the last must also implement
/// [`RefAggregateOp`].
///
/// Alongside the tuple of aggregate ops, `Combine` requires two additional closures:
///
/// - **`new_fn`** - Defines how a new group's *grand value* is initialized from the values
///   produced by each individual op when the group is first created.
///   This receives a reference to the group's key and a tuple of new values,
///   and returns the "grand" value.
///   
///   Signature: `FnMut(&Key, (Value0, Value1, …, ValueN)) -> GrandValue`.
///
/// - **`get_mut_fn`** - Takes mutable references to the values of every op
///   stored inside the grand value.
///   The returned references should correspond to
///   the new values that were passed into `new_fn`.
///   
///   Signature: `FnMut(&mut GrandValue) -> (&mut Value0, &mut Value1, …, &mut ValueN)`.
///
/// Currently, 1-ary to 12-ary tuples are supported.
///
/// # Examples
///
/// ```
/// struct Stats {
///     sum: i32,
///     max: i32,
/// }
///
/// // TODO: more later.
/// ```
pub struct Combine<V, F, G, Ops> {
    ops: Ops,
    new_fn: F,
    get_mut_fn: G,
    _marker: PhantomData<fn(&mut V) -> V>,
}

impl<V, F, G, Ops> Combine<V, F, G, Ops>
where
    Ops: Tuple<V, F, G>,
    F: FnMut(&Ops::Key, Ops::Values) -> V,
    G: FnMut(&mut V) -> Ops::ValuesMut<'_>,
{
    /// Creates a new instance of this aggregate op.
    pub fn new(ops: Ops, new_fn: F, get_mut_fn: G) -> Self {
        assert_op(Self {
            ops,
            new_fn,
            get_mut_fn,
            _marker: PhantomData,
        })
    }
}

impl<V, F, G, Ops> AggregateOp for Combine<V, F, G, Ops>
where
    Ops: Tuple<V, F, G>,
    F: FnMut(&Ops::Key, Ops::Values) -> V,
    G: FnMut(&mut V) -> Ops::ValuesMut<'_>,
{
    type Key = Ops::Key;

    type Value = V;

    type Item = Ops::Item;

    #[inline]
    fn new_value(&mut self, key: &Self::Key, item: Self::Item) -> Self::Value {
        let values = self.ops.new_value(key, item);
        (self.new_fn)(key, values)
    }

    #[inline]
    fn modify(&mut self, value: &mut Self::Value, item: Self::Item) {
        let values = (self.get_mut_fn)(value);
        self.ops.modify(values, item);
    }
}

trait Sealed {}

#[doc(hidden)] // Needed (plus the trait being `pub`) due to E0446.
#[allow(private_bounds)]
pub trait Tuple<V, F, G>: Sealed + Sized {
    type Key;

    type Item;

    type Values;

    type ValuesMut<'a>
    where
        Self: 'a;

    fn new_value(&mut self, key: &Self::Key, item: Self::Item) -> Self::Values;

    fn modify<'a>(&mut self, values: Self::ValuesMut<'a>, item: Self::Item)
    where
        Self: 'a;
}

macro_rules! tuple_impl {
    (
        $($tys:ident)*,
        $($ops:ident)*,
        $($values:ident)*,
    ) => {
        impl<K, It, $($tys,)* OpLast> Sealed for ($($tys,)* OpLast,)
        where
            $($tys: RefAggregateOp<Key = K, Item = It>,)*
            OpLast: AggregateOp<Key = K, Item = It>,
        {
        }

        impl<$($tys,)* OpLast, K, It, V, F, G> Tuple<V, F, G> for ($($tys,)* OpLast,)
        where
            $($tys: RefAggregateOp<Key = K, Item = It>,)*
            OpLast: AggregateOp<Key = K, Item = It>,
        {
            type Key = K;

            type Item = It;

            type Values = ($($tys::Value,)* OpLast::Value,);

            type ValuesMut<'a>
                = ($(&'a mut $tys::Value,)* &'a mut OpLast::Value,)
            where
                Self: 'a;

            #[allow(unused_mut)]
            fn new_value(&mut self, key: &Self::Key, mut item: Self::Item) -> Self::Values {
                let ($($ops,)* last_op,) = self;
                // (op0.new_value_ref(key, &mut item), op1.new_value(key, item))
                (
                    $($ops.new_value_ref(key, &mut item),)*
                    last_op.new_value(key, item),
                )
            }

            #[allow(unused_mut)]
            fn modify<'a>(&mut self, values: Self::ValuesMut<'a>, mut item: Self::Item)
            where
                Self: 'a,
            {
                let ($($ops,)* last_op,) = self;
                let ($($values,)* last_value,) = values;

                $($ops.modify_ref($values, &mut item);)*
                last_op.modify(last_value, item);
            }
        }
    };
}

tuple_impl!(
    ,
    ,
    ,
);

#[rustfmt::skip]
tuple_impl!(
    Op0,
    op0,
    value0,
);

tuple_impl!(
    Op0 Op1,
    op0 op1,
    value0 value1,
);

tuple_impl!(
    Op0 Op1 Op2,
    op0 op1 op2,
    value0 value1 value2,
);

tuple_impl!(
    Op0 Op1 Op2 Op3,
    op0 op1 op2 op3,
    value0 value1 value2 value3,
);

tuple_impl!(
    Op0 Op1 Op2 Op3 Op4,
    op0 op1 op2 op3 op4,
    value0 value1 value2 value3 value4,
);

tuple_impl!(
    Op0 Op1 Op2 Op3 Op4 Op5,
    op0 op1 op2 op3 op4 op5,
    value0 value1 value2 value3 value4 value5,
);

tuple_impl!(
    Op0 Op1 Op2 Op3 Op4 Op5 Op6,
    op0 op1 op2 op3 op4 op5 op6,
    value0 value1 value2 value3 value4 value5 value6,
);

tuple_impl!(
    Op0 Op1 Op2 Op3 Op4 Op5 Op6 Op7,
    op0 op1 op2 op3 op4 op5 op6 op7,
    value0 value1 value2 value3 value4 value5 value6 value7,
);

tuple_impl!(
    Op0 Op1 Op2 Op3 Op4 Op5 Op6 Op7 Op8,
    op0 op1 op2 op3 op4 op5 op6 op7 op8,
    value0 value1 value2 value3 value4 value5 value6 value7 value8,
);

tuple_impl!(
    Op0 Op1 Op2 Op3 Op4 Op5 Op6 Op7 Op8 Op9,
    op0 op1 op2 op3 op4 op5 op6 op7 op8 op9,
    value0 value1 value2 value3 value4 value5 value6 value7 value8 value9,
);

tuple_impl!(
    Op0 Op1 Op2 Op3 Op4 Op5 Op6 Op7 Op8 Op9 Op10,
    op0 op1 op2 op3 op4 op5 op6 op7 op8 op9 op10,
    value0 value1 value2 value3 value4 value5 value6 value7 value8 value9 value10,
);

tuple_impl!(
    Op0 Op1 Op2 Op3 Op4 Op5 Op6 Op7 Op8 Op9 Op10 Op11,
    op0 op1 op2 op3 op4 op5 op6 op7 op8 op9 op10 op11,
    value0 value1 value2 value3 value4 value5 value6 value7 value8 value9 value10 value11,
);
