//! String-related [`Collector`]s.
//!
//! This module provides [`Collector`] implementations for [`String`] as well as
//! collectors for string concatenation.
//!
//! Collectors from [`String`] can collect `char`s. If you want to concat strings instead,
//! use [`into_concat()`](Concat::into_concat) or [`concat_mut()`](Concat::concat_mut)
//! method on a string.
//!
//! This module corresponds to [`std::string`].

use std::{borrow::Borrow, ops::ControlFlow};

#[cfg(not(feature = "std"))]
use alloc::string::String;

use crate::{
    collector::{Collector, CollectorBase},
    slice::{Concat, ConcatItem, ConcatItemSealed, ConcatSealed},
};

/// A collector that pushes `char`s into a [`String`].
/// Its [`Output`] is [`String`].
///
/// This struct is created by `String::into_collector()`.
///
/// [`Collector`]: crate::collector::Collector
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Clone, Default)]
pub struct IntoCollector(String);

/// A collector that pushes `char`s into a [`&mut String`](String).
/// Its [`Output`] is [`&mut String`](String).
///
/// This struct is created by `String::collector_mut()`.
///
/// [`Collector`]: crate::collector::Collector
/// [`Output`]: CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a>(&'a mut String);

impl crate::collector::IntoCollectorBase for String {
    type Output = Self;

    type IntoCollector = IntoCollector;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        IntoCollector(self)
    }
}

impl<'a> crate::collector::IntoCollectorBase for &'a mut String {
    type Output = Self;

    type IntoCollector = CollectorMut<'a>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        CollectorMut(self)
    }
}

impl CollectorBase for IntoCollector {
    type Output = String;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl Collector<char> for IntoCollector {
    #[inline]
    fn collect(&mut self, ch: char) -> ControlFlow<()> {
        self.0.push(ch);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = char>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = char>) -> Self::Output {
        self.0.extend(items);
        self.0
    }
}

impl<'a> Collector<&'a char> for IntoCollector {
    #[inline]
    fn collect(&mut self, &ch: &char) -> ControlFlow<()> {
        self.0.push(ch);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'a char>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = &'a char>) -> Self::Output {
        self.0.extend(items);
        self.0
    }
}

impl<'a> Collector<&'a mut char> for IntoCollector {
    #[inline]
    fn collect(&mut self, &mut ch: &'a mut char) -> ControlFlow<()> {
        self.0.push(ch);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'a mut char>) -> ControlFlow<()> {
        self.0.extend(items.into_iter().map(|&mut ch| ch));
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(
        mut self,
        items: impl IntoIterator<Item = &'a mut char>,
    ) -> Self::Output {
        self.0.extend(items.into_iter().map(|&mut ch| ch));
        self.0
    }
}

impl<'a> CollectorBase for CollectorMut<'a> {
    type Output = &'a mut String;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<'a> Collector<char> for CollectorMut<'a> {
    #[inline]
    fn collect(&mut self, ch: char) -> ControlFlow<()> {
        self.0.push(ch);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = char>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = char>) -> Self::Output {
        self.0.extend(items);
        self.0
    }
}

impl<'a, 'c> Collector<&'c char> for CollectorMut<'a> {
    #[inline]
    fn collect(&mut self, &ch: &'c char) -> ControlFlow<()> {
        self.0.push(ch);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'c char>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = &'c char>) -> Self::Output {
        self.0.extend(items);
        self.0
    }
}

impl<'a, 'c> Collector<&'c mut char> for CollectorMut<'a> {
    #[inline]
    fn collect(&mut self, &mut ch: &'c mut char) -> ControlFlow<()> {
        self.0.push(ch);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'c mut char>) -> ControlFlow<()> {
        self.0.extend(items.into_iter().map(|&mut ch| ch));
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = &'c mut char>) -> Self::Output {
        self.0.extend(items.into_iter().map(|&mut ch| ch));
        self.0
    }
}

/// # Examples
///
/// ```
/// use komadori::prelude::*;
///
/// let s = "abc de fghi j";
///
/// let s_no_whitespace = s
///     .split_whitespace()
///     .feed_into(String::new().into_concat());
///
/// assert_eq!(s_no_whitespace, "abcdefghij");
/// ```
impl Concat for String {}

/// See [`std::slice::Concat`] for why this trait bound is used.
impl<S> ConcatItem<String> for S where S: Borrow<str> {}

impl ConcatSealed for String {}

impl<S> ConcatItemSealed<String> for S
where
    S: Borrow<str>,
{
    #[inline]
    fn push_to(&mut self, owned_slice: &mut String) {
        owned_slice.push_str((*self).borrow());
    }
}
