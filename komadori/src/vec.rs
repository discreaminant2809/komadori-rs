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
    use std::{fmt::Debug, ops::ControlFlow};

    use proptest::collection::vec as propvec;
    use proptest::prelude::*;

    use crate::{
        prelude::*,
        test_utils::{
            BasicCollectorModel, CollectorFactoryBase, DefineCollector, TwoIterData,
            TwoIterFactory, TwoIterMutData, TwoIterMutFactory, TwoIterRefFactory, collector_test,
        },
    };

    use super::*;

    collector_test!(into_collector {
        iter_data: TwoIterData::strategy(),
        collector_data: propvec(any::<i32>(), ..5),
        iter_f: TwoIterFactory,
        collector_f: |starting_nums: &Vec<_>| starting_nums.clone(),
        model_f: |starting_nums| BasicCollectorModel {
            state: starting_nums.clone(),
            advance_f: |buf: &mut Vec<_>, item| buf.push(item),
            max_afford_f: |_, request| request,
            cf_f: |_| ControlFlow::Continue(()),
            output_and_pred_f: |buf| (buf, |expected: &_, actual: &_| expected == actual),
        },
    });

    collector_test!(into_collector_ref {
        iter_data: TwoIterData::strategy(),
        collector_data: propvec(any::<i32>(), ..5),
        iter_f: TwoIterRefFactory,
        collector_f: |starting_nums: &Vec<_>| starting_nums.clone(),
        model_f: |starting_nums| BasicCollectorModel {
            state: starting_nums.clone(),
            advance_f: |buf: &mut Vec<_>, &item: &_| buf.push(item),
            max_afford_f: |_, request| request,
            cf_f: |_| ControlFlow::Continue(()),
            output_and_pred_f: |buf| (buf, |expected: &_, actual: &_| expected == actual),
        },
    });

    collector_test!(into_collector_mut {
        iter_data: TwoIterMutData::strategy(),
        collector_data: propvec(any::<i32>(), ..5),
        iter_f: TwoIterMutFactory,
        collector_f: |starting_nums: &Vec<_>| starting_nums.clone(),
        model_f: |starting_nums| BasicCollectorModel {
            state: starting_nums.clone(),
            advance_f: |buf: &mut Vec<_>, &mut item: &mut _| buf.push(item),
            max_afford_f: |_, request| request,
            cf_f: |_| ControlFlow::Continue(()),
            output_and_pred_f: |buf| (buf, |expected: &_, actual: &_| expected == actual),
        },
    });

    collector_test!(collector_mut {
        iter_data: TwoIterData::strategy(),
        collector_data: CollectorMutData::strategy(),
        iter_f: TwoIterFactory,
        collector_f: CollectorMutFactory,
        model_f: |data| BasicCollectorModel {
            state: data.starting_nums.clone(),
            advance_f: |buf: &mut Vec<_>, item| buf.push(item),
            max_afford_f: |_, request| request,
            cf_f: |_| ControlFlow::Continue(()),
            output_and_pred_f: |buf| (buf, |expected: &_, actual: &_| expected == *actual),
        },
    });

    collector_test!(collector_mut_ref {
        iter_data: TwoIterData::strategy(),
        collector_data: CollectorMutData::strategy(),
        iter_f: TwoIterRefFactory,
        collector_f: CollectorMutFactory,
        model_f: |data| BasicCollectorModel {
            state: data.starting_nums.clone(),
            advance_f: |buf: &mut Vec<_>, &item: &_| buf.push(item),
            max_afford_f: |_, request| request,
            cf_f: |_| ControlFlow::Continue(()),
            output_and_pred_f: |buf| (buf, |expected: &_, actual: &_| expected == *actual),
        },
    });

    collector_test!(collector_mut_mut {
        iter_data: TwoIterMutData::strategy(),
        collector_data: CollectorMutData::strategy(),
        iter_f: TwoIterMutFactory,
        collector_f: CollectorMutFactory,
        model_f: |data| BasicCollectorModel {
            state: data.starting_nums.clone(),
            advance_f: |buf: &mut Vec<_>, &mut item: &mut _| buf.push(item),
            max_afford_f: |_, request| request,
            cf_f: |_| ControlFlow::Continue(()),
            output_and_pred_f: |buf| (buf, |expected: &_, actual: &_| expected == *actual),
        },
    });

    #[derive(Clone)]
    struct CollectorMutData {
        starting_nums: Vec<i32>,
        base: Vec<i32>,
    }

    impl Debug for CollectorMutData {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!("{:?}", self.starting_nums))
        }
    }

    impl CollectorMutData {
        fn new(starting_nums: Vec<i32>) -> Self {
            Self {
                starting_nums,
                base: vec![],
            }
        }

        fn strategy() -> impl Strategy<Value = Self> {
            propvec(any::<i32>(), ..=2).prop_map(Self::new)
        }
    }

    #[derive(Clone)]
    struct CollectorMutFactory;

    impl<'d> DefineCollector<'d, CollectorMutData> for CollectorMutFactory {
        type Collector = CollectorMut<'d, i32>;
        type Output = &'d mut Vec<i32>;
    }

    impl CollectorFactoryBase<CollectorMutData> for CollectorMutFactory {
        fn collector<'d>(
            &self,
            data: &'d mut CollectorMutData,
        ) -> <Self as DefineCollector<'d, CollectorMutData>>::Collector {
            // We deliberately clear the old buffer so that the collector's capacity
            // always starts new also!
            data.base = data.starting_nums.clone();
            data.base.collector_mut()
        }
    }
}
