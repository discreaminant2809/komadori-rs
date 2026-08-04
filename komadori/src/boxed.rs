use core::ops::ControlFlow;

use alloc::boxed::Box;

use crate::collector::{Collector, CollectorBase, advanced_collect_many_default_impl};

impl<C> CollectorBase for Box<C>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        (*self).finish()
    }

    #[inline]
    fn finish_boxed(self: Box<Self>) -> Self::Output {
        (*self).finish_boxed()
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        (**self).reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        (**self).max_afford(request)
    }
}

impl<C, T> Collector<T> for Box<C>
where
    C: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        C::collect(self, item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: The caller has reserved for one item.
            C::assume_reserved_collect(self, item)
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // FIXED: specialization for unsized type.
        // We can't add `?Sized` to the bound of `C` because this method requires `Sized`.
        C::collect_many(self, items)
    }

    // The default implementation for `collect_then_finish()` is sufficient.
}

macro_rules! dyn_impl {
    ($($traits:ident)*) => {
        impl<O> CollectorBase for Box<dyn CollectorBase<Output = O> $(+ $traits)* + '_> {
            type Output = O;

            // Yes. This is the entire purpose of `finish_boxed`!
            #[inline]
            fn finish(self) -> Self::Output {
                self.finish_boxed()
            }

            #[inline]
            fn finish_boxed(self: Box<Self>) -> Self::Output {
                (*self).finish_boxed()
            }

            #[inline]
            fn reserve(&mut self, additional: usize) {
                (**self).reserve(additional)
            }

            #[inline]
            fn max_afford(&self, request: usize) -> usize {
                (**self).max_afford(request)
            }
        }

        impl<T, O> CollectorBase for Box<dyn Collector<T, Output = O> $(+ $traits)* + '_> {
            type Output = O;

            // Yes. This is the entire purpose of `finish_boxed`!
            #[inline]
            fn finish(self) -> Self::Output {
                self.finish_boxed()
            }

            #[inline]
            fn finish_boxed(self: Box<Self>) -> Self::Output {
                (*self).finish_boxed()
            }

            #[inline]
            fn reserve(&mut self, additional: usize) {
                (**self).reserve(additional)
            }

            #[inline]
            fn max_afford(&self, request: usize) -> usize {
                (**self).max_afford(request)
            }
        }


        impl<T, O> Collector<T> for Box<dyn Collector<T, Output = O> $(+ $traits)* + '_> {
            #[inline]
            fn collect(&mut self, item: T) -> ControlFlow<()> {
                (**self).collect(item)
            }

            #[inline]
            fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
                advanced_collect_many_default_impl(self, items)
            }

            #[inline]
            unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
                unsafe {
                    // SAFETY: The caller has reserved for one item.
                    (**self).assume_reserved_collect(item)
                }
            }
        }
    };
}

dyn_impl!();
dyn_impl!(Send);
dyn_impl!(Sync);
dyn_impl!(Send Sync);

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::{collector::take_collector_model, test_utils::prelude::*};

    collector_test!(box_sized {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: Box::new(vec![].into_collector().take(n)),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });

    collector_test!(box_dyn {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: Box::new(vec![].into_collector().take(n)) as Box<dyn Collector<_, Output = _>>,
        expected_f: |iter, count| {
            let res: Vec<_> = iter.take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
