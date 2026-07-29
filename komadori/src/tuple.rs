//! Collectors for tuples.

use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{
    Collector, CollectorBase, Fuse, IntoCollectorBase, advanced_collect_many_default_impl,
    finish_boxed_impl,
};

/// A collector that tees items to multiple underlying collectors.
///
/// Every underlying collector collects the mutable reference of each item
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
/// Unit type is not supported here. Use [`crate::unit::Collector`] instead.
///
/// You can refer to this struct by `IntoCollector<(C0, C1, ..., Cn)>`,
/// where `C0`, `C1`, ..., `Cn` are collectors.
///
/// This struct is created by `<(T0, T1, ..., Tn)>::into_collector()`,
/// where [`T0, T1, ..., Tn: IntoCollectorBase`](IntoCollectorBase),
/// and the created struct is
/// `IntoCollector<(T0::IntoCollector, T1::IntoCollector, ..., Tn::IntoCollector)>`.
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::Max};
///
/// let (nums, sum, max) = [4, 2, 6, 3]
///     .into_iter()
///     .feed_into((
///         vec![],
///         0.into_sum(),
///         Max::new(),
///     ));
///
/// assert_eq!(nums, [4, 2, 6, 3]);
/// assert_eq!(sum, 15);
/// assert_eq!(max, Some(6));
/// ```
#[allow(private_bounds)]
pub struct IntoCollector<Cs: Tuple>(Cs::IntoCollectorRepr);

trait Tuple {
    type IntoCollectorRepr;
}

impl<Cs> Debug for IntoCollector<Cs>
where
    Cs: Tuple<IntoCollectorRepr: Debug>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("IntoCollector").field(&self.0).finish()
    }
}

impl<Cs> Clone for IntoCollector<Cs>
where
    Cs: Tuple<IntoCollectorRepr: Clone>,
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

impl<C> Tuple for (C,) {
    type IntoCollectorRepr = C;
}

impl<C> IntoCollectorBase for (C,)
where
    C: IntoCollectorBase,
{
    type Output = (C::Output,);

    type IntoCollector = IntoCollector<(C::IntoCollector,)>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        let (collector,) = self;
        IntoCollector(collector.into_collector())
    }
}

impl<C> CollectorBase for IntoCollector<(C,)>
where
    C: CollectorBase,
{
    type Output = (C::Output,);

    fn finish(self) -> Self::Output {
        (self.0.finish(),)
    }

    finish_boxed_impl! {}

    fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    fn max_afford(&self, request: usize) -> usize {
        self.0.max_afford(request)
    }
}

// In this special case, we can just forward everything.
// We can't do that for tuple more than 1-ary, however.
impl<C, T> Collector<T> for IntoCollector<(C,)>
where
    C: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.0.collect(item)
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.0.collect_many(items)
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        (self.0.collect_then_finish(items),)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: The caller has reserved for one item.
            self.0.assume_reserved_collect(item)
        }
    }
}

macro_rules! tuple_impl {
    ($last_ty_name:ident $($ty_name:ident)*) => {
        impl <$($ty_name,)* $last_ty_name> Tuple for ($($ty_name,)* $last_ty_name) {
            type IntoCollectorRepr = ($(Fuse<$ty_name>,)* Fuse<$last_ty_name>);
        }

        impl <$($ty_name,)* $last_ty_name> IntoCollectorBase for ($($ty_name,)* $last_ty_name)
        where
            $($ty_name: IntoCollectorBase,)*
            $last_ty_name: IntoCollectorBase,
        {
            type Output = ($($ty_name::Output,)* $last_ty_name::Output);

            type IntoCollector = IntoCollector<($($ty_name::IntoCollector,)* $last_ty_name::IntoCollector)>;

            #[inline]
            fn into_collector(self) -> Self::IntoCollector {
                #[allow(non_snake_case)]
                let ($($ty_name,)* $last_ty_name) = self;
                IntoCollector((
                    $($ty_name.into_collector().fuse(),)*
                    $last_ty_name.into_collector().fuse(),
                ))
            }
        }

        #[allow(non_snake_case)]
        impl <$($ty_name,)* $last_ty_name> CollectorBase for IntoCollector<($($ty_name,)* $last_ty_name)>
        where
            $($ty_name: CollectorBase,)*
            $last_ty_name: CollectorBase,
        {
            type Output = ($($ty_name::Output,)* $last_ty_name::Output);

            fn finish(self) -> Self::Output {
                let ($($ty_name,)* $last_ty_name) = self.0;
                ($($ty_name.finish(),)* $last_ty_name.finish())
            }

            finish_boxed_impl! {}

            fn reserve(&mut self, additional: usize) {
                let ($($ty_name,)* $last_ty_name) = &mut self.0;
                $($ty_name.reserve(additional);)*
                $last_ty_name.reserve(additional);
            }

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
        impl <$($ty_name,)* $last_ty_name, T> Collector<T> for IntoCollector<($($ty_name,)* $last_ty_name)>
        where
            $($ty_name: for<'a> Collector<&'a mut T>,)*
            $last_ty_name: Collector<T>,
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
                advanced_collect_many_default_impl(self, items)
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

tuple_impl!(CLast C0);
tuple_impl!(CLast C0 C1);
tuple_impl!(CLast C0 C1 C2);
tuple_impl!(CLast C0 C1 C2 C3);
tuple_impl!(CLast C0 C1 C2 C3 C4);
tuple_impl!(CLast C0 C1 C2 C3 C4 C5);
tuple_impl!(CLast C0 C1 C2 C3 C4 C5 C6);
tuple_impl!(CLast C0 C1 C2 C3 C4 C5 C6 C7);
tuple_impl!(CLast C0 C1 C2 C3 C4 C5 C6 C7 C8);
tuple_impl!(CLast C0 C1 C2 C3 C4 C5 C6 C7 C8 C9);
tuple_impl!(CLast C0 C1 C2 C3 C4 C5 C6 C7 C8 C9 C10);
tuple_impl!(CLast C0 C1 C2 C3 C4 C5 C6 C7 C8 C9 C10 C11);

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::{collector::take_collector_model, test_utils::prelude::*};

    collector_test!(tuple_3_ary {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let first_n = ..=5_usize;
            let second_n = ..=5_usize;
            let third_n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: (
            vec![].into_collector().take(first_n),
            vec![].into_collector().take(second_n),
            vec![].into_collector().take(third_n),
        )
            .into_collector(),
        expected_f: |iter, count| {
            let max_n = first_n.max(second_n).max(third_n);
            let nums: Vec<i32> = iter.take(max_n).collect();

            (
                (
                    nums.iter().copied().take(first_n).collect(),
                    nums.iter().copied().take(second_n).collect(),
                    nums.iter().copied().take(third_n).collect(),
                ),
                count >= max_n,
            )
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(first_n.max(second_n).max(third_n)),
    });

    collector_test!(tuple_1_ary {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: (vec![].into_collector().take(n),).into_collector(),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.take(n).collect();
            ((res,), count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
