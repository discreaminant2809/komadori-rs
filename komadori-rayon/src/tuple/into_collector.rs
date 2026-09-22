use std::{fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{
        Fuse, IntoParallelCollectorBase, ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that tees items to multiple underlying parallel collectors.
///
/// Every underlying parallel collector collects the mutable reference of each item
/// (in order from left to right),
/// except the last one which collects the item directly lastly.
///
/// This is similar to
///
/// ```text
/// collector1
///     .tee_mut(collector2)
///     .tee_mut(collector3)
///     ...
///     .tee_funnel(collector_n);
/// ```
///
/// except it is more readable and its output is a tuple of the underlying collectors'
/// of the same arity instead of a horribly nested tuple.
/// Most of the time you should prefer this over deeply chained `tee_*()`.
///
/// Unit type is not supported here. Use [`crate::unit::ParCollector`] instead.
///
/// You can refer to this struct by `IntoParCollector<(C0, C1, ..., Cn)>`,
/// where `C0`, `C1`, ..., `Cn` are parallel collectors.
///
/// This struct is created by `<(T0, T1, ..., Tn)>::into_par_collector()`,
/// where [`T0, T1, ..., Tn: IntoParallelCollectorBase`](IntoParallelCollectorBase),
/// and the created struct is
/// `IntoParCollector<(T0::IntoParCollector, T1::IntoParCollector, ..., Tn::IntoParCollector)>`.
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMax};
///
/// let mut nums = vec![1];
/// let (sum, _, max) = [4, 2, 6, 3]
///     .into_par_iter()
///     .feed_into((
///         0.into_par_sum(),
///         &mut nums,
///         ParMax::new(),
///     ));
///
/// assert_eq!(sum, 15);
/// assert_eq!(max, Some(6));
/// assert_eq!(nums, [1, 4, 2, 6, 3]);
/// ```
#[expect(private_bounds)]
pub struct IntoParCollector<Cs: Tuple>(Cs::IntoParCollectorRepr);

impl<Cs> Debug for IntoParCollector<Cs>
where
    Cs: Tuple<IntoParCollectorRepr: Debug>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("IntoCollector").field(&self.0).finish()
    }
}

impl<Cs> Clone for IntoParCollector<Cs>
where
    Cs: Tuple<IntoParCollectorRepr: Clone>,
{
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
    }
}

trait Tuple {
    type IntoParCollectorRepr;
}

impl<C> Tuple for (C,) {
    // So that we debug, the callers can see a 1-ary tuple
    // and infer it's converted from a 1-ary tuple.
    type IntoParCollectorRepr = (C,);
}

impl<C> IntoParallelCollectorBase for (C,)
where
    C: IntoParallelCollectorBase,
{
    type Output = (C::Output,);

    type IntoParCollector = IntoParCollector<(C::IntoParCollector,)>;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        IntoParCollector((self.0.into_par_collector(),))
    }
}

impl<'a, C> DefineSerial<'a> for IntoParCollector<(C,)>
where
    C: DefineSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, C::Serial>;
}

impl<'a, C> DefineUnindexedSerial<'a> for IntoParCollector<(C,)>
where
    C: DefineUnindexedSerial<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, C::UnindexedSerial>;
}

impl<C> ParallelCollectorBase for IntoParCollector<(C,)>
where
    C: ParallelCollectorBase,
{
    type Output = (C::Output,);

    #[inline]
    fn finish(self) -> Self::Output {
        (self.0.0.finish(),)
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.0.0.max_afford(request)
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify(self.0.0.parts(len))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        unique::take_uniquify(self.0.0.take_parts(len))
    }
}

impl<C> UnindexedParallelCollectorBase for IntoParCollector<(C,)>
where
    C: UnindexedParallelCollectorBase,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify(self.0.0.unindexed_parts())
    }

    fn take_unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        unique_unindexed::take_uniquify(self.0.0.take_unindexed_parts())
    }
}

macro_rules! impl_par_collector {
    ($($C:ident $o:ident)*) => {
        impl <$($C,)*> Tuple for ($($C,)*) {
            type IntoParCollectorRepr = ($(Fuse<$C>,)*);
        }

        #[expect(non_snake_case)]
        impl<$($C,)*> IntoParallelCollectorBase for ($($C,)*)
        where
            $($C: IntoParallelCollectorBase,)*
        {
            type Output = ($($C::Output,)*);

            type IntoParCollector = IntoParCollector<($($C::IntoParCollector,)*)>;

            #[inline]
            fn into_par_collector(self) -> Self::IntoParCollector {
                let ($($C,)*) = self;
                IntoParCollector((
                    $($C.into_par_collector().fuse(),)*
                ))
            }
        }

        impl<'a, $($C,)*> DefineSerial<'a> for IntoParCollector<($($C,)*)>
        where
            $($C: DefineSerial<'a>,)*
        {
            type Serial = unique::Serial<
                'a, Self,
                consumer::Serial<($(<Fuse<$C> as DefineSerial<'a>>::Serial,)*)>
            >;
        }

        impl<'a, $($C,)*> DefineUnindexedSerial<'a> for IntoParCollector<($($C,)*)>
        where
            $($C: DefineUnindexedSerial<'a>,)*
        {
            type UnindexedSerial = unique_unindexed::Serial<
                'a, Self,
                consumer::Serial<($(<Fuse<$C> as DefineUnindexedSerial<'a>>::UnindexedSerial,)*)>
            >;
        }

        #[expect(non_snake_case)]
        impl<$($C,)*> ParallelCollectorBase for IntoParCollector<($($C,)*)>
        where
            $($C: ParallelCollectorBase,)*
        {
            type Output = ($($C::Output,)*);

            #[inline]
            fn finish(self) -> Self::Output {
                let ($($C,)*) = self.0;
                ($($C.finish(),)*)
            }

            #[inline]
            fn max_afford(&self, request: usize) -> usize {
                let ($($C,)*) = &self.0;
                let mut max_afford = usize::MIN;
                $(max_afford = $C.max_afford(request).max(max_afford);)*
                max_afford
            }

            fn parts<'a>(
                &'a mut self,
                len: usize,
            ) -> (
                impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
                impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
            ) {
                let ($($C,)*) = &mut self.0;
                $(let $C = $C.parts(len);)*

                unique::uniquify((
                    consumer::Consumer::new(($($C.0,)*)),
                    |($($o,)*)| {
                        let mut is_break = true;
                        $(is_break &= ($C.1)($o).is_break();)*

                        if is_break {
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    },
                ))
            }

            fn take_parts<'a>(
                &'a mut self,
                len: usize,
            ) -> (
                impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
                impl FnOnce(SerialOutputOf<'a, Self>),
            ) {
                let ($($C,)*) = &mut self.0;
                $(let $C = $C.take_parts(len);)*

                unique::take_uniquify((
                    consumer::Consumer::new(($($C.0,)*)),
                    |($($o,)*)| {
                        $(($C.1)($o);)*
                    },
                ))
            }
        }

        #[expect(non_snake_case)]
        impl<$($C,)*> UnindexedParallelCollectorBase for IntoParCollector<($($C,)*)>
        where
            $($C: UnindexedParallelCollectorBase,)*
        {
            fn unindexed_parts<'a>(
                &'a mut self,
            ) -> (
                impl UnindexedConsumer<
                    IntoCollector = UnindexedSerialOf<'a, Self>,
                    Output = UnindexedSerialOutputOf<'a, Self>,
                >,
                impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
            ) {
                let ($($C,)*) = &mut self.0;
                $(let $C = $C.unindexed_parts();)*

                unique_unindexed::uniquify((
                    consumer::Consumer::new(($($C.0,)*)),
                    |($($o,)*)| {
                        let mut is_break = true;
                        $(is_break &= ($C.1)($o).is_break();)*

                        if is_break {
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    },
                ))
            }

            fn take_unindexed_parts<'a>(
                &'a mut self,
            ) -> (
                impl UnindexedConsumer<
                    IntoCollector = UnindexedSerialOf<'a, Self>,
                    Output = UnindexedSerialOutputOf<'a, Self>,
                >,
                impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
            ) {
                let ($($C,)*) = &mut self.0;
                $(let $C = $C.take_unindexed_parts();)*

                unique_unindexed::take_uniquify((
                    consumer::Consumer::new(($($C.0,)*)),
                    |($($o,)*)| {
                        $(($C.1)($o);)*
                    },
                ))
            }
        }
    };
}
impl_par_collector!(C0 o0 C1 o1);
impl_par_collector!(C0 o0 C1 o1 C2 o2);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3 C4 o4);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3 C4 o4 C5 o5);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3 C4 o4 C5 o5 C6 o6);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3 C4 o4 C5 o5 C6 o6 C7 o7);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3 C4 o4 C5 o5 C6 o6 C7 o7 C8 o8);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3 C4 o4 C5 o5 C6 o6 C7 o7 C8 o8 C9 o9);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3 C4 o4 C5 o5 C6 o6 C7 o7 C8 o8 C9 o9 C10 o10);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3 C4 o4 C5 o5 C6 o6 C7 o7 C8 o8 C9 o9 C10 o10 C11 o11);
impl_par_collector!(C0 o0 C1 o1 C2 o2 C3 o3 C4 o4 C5 o5 C6 o6 C7 o7 C8 o8 C9 o9 C10 o10 C11 o11 C12 o12);

#[expect(missing_debug_implementations, private_bounds)]
mod consumer {
    use std::ops::ControlFlow;

    use crate::collector::plumbing;

    use super::Tuple;

    pub struct Consumer<Cs: Tuple>(Cs);

    pub struct Combiner<Cs: Tuple>(Cs);

    // We don't need to fuse. We've fused at the parallel collector level.
    pub struct Serial<Cs: Tuple>(Cs);

    impl<Cs: Tuple> Consumer<Cs> {
        #[inline]
        pub(super) fn new(tuple: Cs) -> Self {
            Self(tuple)
        }
    }

    macro_rules! impl_consumer_combiner {
        ($($C:ident $O:ident)*) => {
            #[expect(non_snake_case)]
            impl<$($C,)*> plumbing::IntoCollectorBase for Consumer<($($C,)*)>
            where
                $($C: plumbing::IntoCollectorBase,)*
            {
                type Output = ($($C::Output,)*);

                type IntoCollector = Serial<($($C::IntoCollector,)*)>;

                #[inline]
                fn into_collector(self) -> Self::IntoCollector {
                    let ($($C,)*) = self.0;
                    Serial(($($C.into_collector(),)*))
                }
            }

            #[expect(non_snake_case)]
            impl<$($C,)*> plumbing::Consumer for Consumer<($($C,)*)>
            where
                $($C: plumbing::Consumer,)*
            {
                type Combiner = Combiner<($($C::Combiner,)*)>;

                #[inline]
                fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
                    let ($($C,)*) = &mut self.0;
                    $(let $C = $C.split_off_left_at(index);)*

                    (
                        Self(($($C.0,)*)),
                        Combiner(($($C.1,)*)),
                    )
                }

                #[inline]
                fn max_afford(&self, request: usize) -> usize {
                    let ($($C,)*) = &self.0;
                    let mut max_afford = usize::MIN;
                    $(max_afford = $C.max_afford(request).max(max_afford);)*
                    max_afford
                }
            }

            #[expect(non_snake_case)]
            impl<$($C,)*> plumbing::UnindexedConsumer for Consumer<($($C,)*)>
            where
                $($C: plumbing::UnindexedConsumer,)*
            {
                #[inline]
                fn split_off_left(&self) -> Self {
                    let ($($C,)*) = &self.0;
                    Self(($($C.split_off_left(),)*))
                }

                #[inline]
                fn to_combiner(&self) -> Self::Combiner {
                    let ($($C,)*) = &self.0;
                    Combiner(($($C.to_combiner(),)*))
                }
            }

            #[expect(non_snake_case)]
            impl<$($C,)* $($O,)*> plumbing::Combiner<($($O,)*)> for Combiner<($($C,)*)>
            where
                $($C: plumbing::Combiner<$O>,)*
            {
                #[inline]
                fn combine(self, ($($C,)*): &mut ($($O,)*), ($($O,)*): ($($O,)*)) {
                    let ($($O,)*) = ($(($C, $O),)*);
                    let ($($C,)*) = self.0;
                    $($C.combine($O.0, $O.1);)*
                }
            }
        };
    }
    impl_consumer_combiner!(C0 O0 C1 O1);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3 C4 O4);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3 C4 O4 C5 O5);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3 C4 O4 C5 O5 C6 O6);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3 C4 O4 C5 O5 C6 O6 C7 O7);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3 C4 O4 C5 O5 C6 O6 C7 O7 C8 O8);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3 C4 O4 C5 O5 C6 O6 C7 O7 C8 O8 C9 O9);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3 C4 O4 C5 O5 C6 O6 C7 O7 C8 O8 C9 O9 C10 O10);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3 C4 O4 C5 O5 C6 O6 C7 O7 C8 O8 C9 O9 C10 O10 C11 O11);
    impl_consumer_combiner!(C0 O0 C1 O1 C2 O2 C3 O3 C4 O4 C5 O5 C6 O6 C7 O7 C8 O8 C9 O9 C10 O10 C11 O11 C12 O12);

    macro_rules! impl_collector {
        ($last_ty_name:ident $($ty_name:ident)*) => {
            #[allow(non_snake_case)]
            impl <$($ty_name,)* $last_ty_name> plumbing::CollectorBase for Serial<($($ty_name,)* $last_ty_name)>
            where
                $($ty_name: plumbing::CollectorBase,)*
                $last_ty_name: plumbing::CollectorBase,
            {
                type Output = ($($ty_name::Output,)* $last_ty_name::Output);

                #[inline]
                fn finish(self) -> Self::Output {
                    let ($($ty_name,)* $last_ty_name) = self.0;
                    ($($ty_name.finish(),)* $last_ty_name.finish())
                }

                plumbing::finish_boxed_impl! {}

                #[inline]
                fn reserve(&mut self, additional: usize) {
                    let ($($ty_name,)* $last_ty_name) = &mut self.0;
                    $($ty_name.reserve(additional);)*
                    $last_ty_name.reserve(additional);
                }

                #[inline]
                fn max_afford(&self, request: usize) -> usize {
                    let ($($ty_name,)* $last_ty_name) = &self.0;

                    let max = [$($ty_name.max_afford(request)),*]
                        .into_iter()
                        .max();

                    let last_max_afford = $last_ty_name.max_afford(request);
                    max.map_or(last_max_afford, move |max| max.max(last_max_afford))
                }
            }

            #[allow(non_snake_case)]
            impl <$($ty_name,)* $last_ty_name, T> plumbing::Collector<T> for Serial<($($ty_name,)* $last_ty_name)>
            where
                $($ty_name: for<'a> plumbing::Collector<&'a mut T>,)*
                $last_ty_name: plumbing::Collector<T>,
            {
                #[inline]
                fn collect(&mut self, mut item: T) -> ControlFlow<()> {
                    let ($($ty_name,)* $last_ty_name) = &mut self.0;

                    // Be careful not to use `&&` over `&`!
                    let all_break = $($ty_name.collect(&mut item).is_break() &)*
                        $last_ty_name.collect(item).is_break();

                    if all_break {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                }

                #[inline]
                fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
                    plumbing::advanced_collect_many_default_impl(self, items)
                }

                #[inline]
                unsafe fn assume_reserved_collect(&mut self, mut item: T) -> ControlFlow<()> {
                    let ($($ty_name,)* $last_ty_name) = &mut self.0;

                    let all_break = unsafe {
                        // SAFETY: The caller has reserved for one item.
                        $($ty_name.assume_reserved_collect(&mut item).is_break() &)*
                            $last_ty_name.assume_reserved_collect(item).is_break()
                    };

                    if all_break {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                }
            }
        };
    }
    impl_collector!(CLast C0);
    impl_collector!(CLast C0 C1);
    impl_collector!(CLast C0 C1 C2);
    impl_collector!(CLast C0 C1 C2 C3);
    impl_collector!(CLast C0 C1 C2 C3 C4);
    impl_collector!(CLast C0 C1 C2 C3 C4 C5);
    impl_collector!(CLast C0 C1 C2 C3 C4 C5 C6);
    impl_collector!(CLast C0 C1 C2 C3 C4 C5 C6 C7);
    impl_collector!(CLast C0 C1 C2 C3 C4 C5 C6 C7 C8);
    impl_collector!(CLast C0 C1 C2 C3 C4 C5 C6 C7 C8 C9);
    impl_collector!(CLast C0 C1 C2 C3 C4 C5 C6 C7 C8 C9 C10);
    impl_collector!(CLast C0 C1 C2 C3 C4 C5 C6 C7 C8 C9 C10 C11);
}
