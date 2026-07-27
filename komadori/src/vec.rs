//! [`Collector`]s for [`Vec`].
//!
//! This module corresponds to [`mod@std::vec`].

use crate::{
    collector::{Collector, CollectorBase},
    slice::{Concat, ConcatItem, ConcatItemSealed, ConcatSealed},
};

use std::{borrow::Borrow, ops::ControlFlow};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;

/// A collector that pushes collected items into a [`Vec`].
/// Its [`Output`] is [`Vec`].
///
/// This struct is created by `Vec::into_collector()`.
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoCollector<T>(Vec<T>);

/// A collector that pushes collected items into a [`&mut Vec`](Vec).
/// Its [`Output`] is [`&mut Vec`](Vec).
///
/// This struct is created by `Vec::collector_mut()`.
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a, T>(&'a mut Vec<T>);

impl<T> crate::collector::IntoCollectorBase for Vec<T> {
    type Output = Self;

    type IntoCollector = IntoCollector<T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        IntoCollector(self)
    }
}

impl<'a, T> crate::collector::IntoCollectorBase for &'a mut Vec<T> {
    type Output = Self;

    type IntoCollector = CollectorMut<'a, T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        CollectorMut(self)
    }
}

impl<T> CollectorBase for IntoCollector<T> {
    type Output = Vec<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }
}

impl<T> Collector<T> for IntoCollector<T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.0.extend(items);
        self.0
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: the caller has reserved for at least one element.
            push_unchecked(&mut self.0, item);
        }

        ControlFlow::Continue(())
    }
}

impl<'i, T> Collector<&'i T> for IntoCollector<T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i T>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = &'i T>) -> Self::Output {
        self.0.extend(items);
        self.0
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: the caller has reserved for at least one element.
            push_unchecked(&mut self.0, item);
        }

        ControlFlow::Continue(())
    }
}

impl<'i, T> Collector<&'i mut T> for IntoCollector<T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut T>) -> ControlFlow<()> {
        self.0.extend(items.into_iter().map(|&mut item| item));
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = &'i mut T>) -> Self::Output {
        self.0.extend(items.into_iter().map(|&mut item| item));
        self.0
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: the caller has reserved for at least one element.
            push_unchecked(&mut self.0, item);
        }

        ControlFlow::Continue(())
    }
}

impl<'a, T> CollectorBase for CollectorMut<'a, T> {
    type Output = &'a mut Vec<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }
}

impl<'a, T> Collector<T> for CollectorMut<'a, T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.0.extend(items);
        self.0
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: the caller has reserved for at least one element.
            push_unchecked(self.0, item);
        }

        ControlFlow::Continue(())
    }
}

impl<'a, 'i, T> Collector<&'i T> for CollectorMut<'a, T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i T>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = &'i T>) -> Self::Output {
        self.0.extend(items);
        self.0
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: the caller has reserved for at least one element.
            push_unchecked(self.0, item);
        }

        ControlFlow::Continue(())
    }
}

impl<'a, 'i, T> Collector<&'i mut T> for CollectorMut<'a, T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut T>) -> ControlFlow<()> {
        self.0.extend(items.into_iter().map(|&mut item| item));
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = &'i mut T>) -> Self::Output {
        self.0.extend(items.into_iter().map(|&mut item| item));
        self.0
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: the caller has reserved for at least one element.
            push_unchecked(self.0, item);
        }

        ControlFlow::Continue(())
    }
}

impl<T> Default for IntoCollector<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// # Safety
///
/// Must have reserved for at least one element via [`Vec::reserve()`] or similar methods.
unsafe fn push_unchecked<T>(v: &mut Vec<T>, item: T) {
    let len = v.len();

    unsafe {
        v.as_mut_ptr()
            // SAFETY: the allocated object is `sizeof(T) * len` big.
            .add(len)
            // SAFETY: We've reserved for at least one element.
            .write(item);

        // SAFETY: We've reserved for at least one element,
        // and the element at index `len` is initialized.
        v.set_len(len + 1);
    }
}

/// # Examples
///
/// ```
/// use komadori::prelude::*;
///
/// let matrix = [vec![1, 2], vec![3, 4, 5], vec![6]];
///
/// let array = matrix
///     .into_iter()
///     .feed_into(Vec::new().into_concat());
///
/// assert_eq!(array, [1, 2, 3, 4, 5, 6]);
/// ```
impl<T> Concat for Vec<T> {}

/// See [`std::slice::Concat`] for why this trait bound is used.
impl<S, T> ConcatItem<Vec<T>> for S
where
    S: Borrow<[T]>,
    T: Clone,
{
}

impl<T> ConcatSealed for Vec<T> {}

impl<S, T> ConcatItemSealed<Vec<T>> for S
where
    S: Borrow<[T]>,
    T: Clone,
{
    #[inline]
    fn push_to(&mut self, owned_slice: &mut Vec<T>) {
        owned_slice.extend_from_slice((*self).borrow());
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {

    use crate::test_utils::prelude::*;

    collector_test!(into_collector {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let starting_nums = propvec(any::<i32>(), ..=2);
        },
        iter: nums.iter().copied(),
        collector: starting_nums.into_collector(),
        expected_f: |iter, _| {
            let mut res = starting_nums.clone();
            res.extend(iter);
            (res, false)
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    collector_test!(into_collector_ref {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let starting_nums = propvec(any::<i32>(), ..=2);
        },
        iter: nums.iter(),
        collector: starting_nums.into_collector(),
        expected_f: |iter, _| {
            let mut res = starting_nums.clone();
            res.extend(iter.copied());
            (res, false)
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    collector_test!(into_collector_mut {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let starting_nums = propvec(any::<i32>(), ..=2);
        },
        iters: {
            let mut other_nums = nums.repeat(2);
            let (for_output, for_model) = other_nums.split_at_mut(nums.len());
            (nums.iter_mut(), for_output.iter_mut(), for_model.iter_mut())
        },
        collector: starting_nums.into_collector(),
        expected_f: |iter, _| {
            let mut res = starting_nums.clone();
            res.extend(iter.map(|&mut num| num));
            (res, false)
        },
        output_pred: PartialEq::eq,
        model: theo_inf_collector_model(),
    });

    collector_test!(collector_mut {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let mut starting_nums = propvec(any::<i32>(), ..=2);
        },
        iter: nums.iter().copied(),
        collector: starting_nums.collector_mut(),
        expected_f: |iter, _| {
            let mut res = starting_nums.clone();
            res.extend(iter);
            (res, false)
        },
        output_pred: |expected, actual| expected == *actual,
        model: theo_inf_collector_model(),
    });

    collector_test!(collector_mut_ref {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let mut starting_nums = propvec(any::<i32>(), ..=2);
        },
        iter: nums.iter(),
        collector: starting_nums.collector_mut(),
        expected_f: |iter, _| {
            let mut res = starting_nums.clone();
            res.extend(iter.copied());
            (res, false)
        },
        output_pred: |expected, actual| expected == *actual,
        model: theo_inf_collector_model(),
    });

    collector_test!(collector_mut_mut {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let mut starting_nums = propvec(any::<i32>(), ..=2);
        },
        iters: {
            let mut other_nums = nums.repeat(2);
            let (for_output, for_model) = other_nums.split_at_mut(nums.len());
            (nums.iter_mut(), for_output.iter_mut(), for_model.iter_mut())
        },
        collector: starting_nums.collector_mut(),
        expected_f: |iter, _| {
            let mut res = starting_nums.clone();
            res.extend(iter.map(|&mut num| num));
            (res, false)
        },
        output_pred: |expected, actual| expected == *actual,
        model: theo_inf_collector_model(),
    });
}
