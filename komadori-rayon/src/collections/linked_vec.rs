#![allow(missing_debug_implementations)]

use std::{collections::LinkedList, marker::PhantomData, ops::ControlFlow};

use komadori::prelude::*;

use crate::{cell::CellOptRefMut, collector::plumbing};

// The entire idea is that we keep a mutable reference to the original collection
// in the "left most" consumer.
// This way, we can be heavily optimized in `par_collector.into_collector()`,
// but reallocations may be triggered more often for general parallel uses.
// Based on the benchmark, it performs just as good as `rayon`'s approach in average,
// which simply (can't call it "naively," tho) just uses linked lists of vecs.

pub trait Collection<T> {
    fn push_back(&mut self, elem: T);

    fn reserve(&mut self, additional: usize) {
        let _ = additional;
    }

    unsafe fn assume_reserved_push_back(&mut self, elem: T) {
        self.push_back(elem);
    }

    #[inline]
    fn push_back_iter(&mut self, elems: impl IntoIterator<Item = T>) {
        elems.into_iter().for_each(|elem| self.push_back(elem));
    }

    #[inline]
    fn push_back_iter_ref<'a>(&mut self, elems: impl IntoIterator<Item = &'a T>)
    where
        T: Copy + 'a,
    {
        elems.into_iter().for_each(|&elem| self.push_back(elem));
    }

    // No for `&'a mut T` because collections in the standard library don't have one.

    /// For the case like `Vec` where it can use existing chunks to optimize,
    /// and generally for the case when reserving is needed.
    #[inline]
    fn push_back_linked_vec(&mut self, chunks: LinkedList<Vec<T>>, len: usize) {
        let _ = len;
        chunks
            .into_iter()
            .for_each(|chunk| self.push_back_iter(chunk));
    }
}

pub struct Consumer<'a, C, T> {
    collection: CellOptRefMut<'a, C>,
    _marker: PhantomData<fn(T)>,
}

pub enum Serial<'a, C, T> {
    LeftMost(&'a mut C),
    Right(Vec<T>),
}

pub struct Output<'a, C, T> {
    collection: Option<&'a mut C>,
    // We won't be pushing chunks into the collections yet
    // since it may incur so much reallocations.
    // We save this job in the committer.
    chunks: Chunks<T>,
}

enum Chunks<T> {
    Single(Vec<T>),
    Plural {
        chunks: LinkedList<Vec<T>>,
        len: usize,
    },
}

impl<T> Chunks<T> {
    fn extend(&mut self, right: Self) {
        match (&mut *self, right) {
            (Self::Single(left_chunk), Self::Single(right_chunk)) if left_chunk.is_empty() => {
                *left_chunk = right_chunk;
            }
            (Self::Single(left_chunk), right) => {
                let left_chunk = std::mem::take(left_chunk);
                *self = match right {
                    Self::Single(right_chunk) => Self::Plural {
                        len: left_chunk.len() + right_chunk.len(),
                        chunks: [left_chunk, right_chunk].into(),
                    },
                    Self::Plural { chunks, len } => Self::Plural {
                        len: left_chunk.len() + len,
                        chunks: if left_chunk.is_empty() {
                            chunks
                        } else {
                            let mut chunk = LinkedList::from([left_chunk]);
                            chunk.extend(chunks);
                            chunk
                        },
                    },
                };
            }
            (Self::Plural { chunks, len }, Self::Single(chunk)) => {
                *len += chunk.len();
                chunks.push_back(chunk);
            }
            (
                Self::Plural {
                    chunks: left_chunks,
                    len: left_len,
                },
                Self::Plural {
                    chunks: mut right_chunks,
                    len: right_len,
                },
            ) => {
                *left_len += right_len;
                left_chunks.append(&mut right_chunks);
            }
        }
    }
}

impl<C, T> Output<'_, C, T>
where
    C: Collection<T>,
{
    pub(crate) fn finalize(self) {
        let Some(collection) = self.collection else {
            // Which means that the parallel collector will be disposed soon
            // (possibly under `trying_options()` or similar),
            // or just being combined incorrectly.
            return;
        };

        match self.chunks {
            // `Vec` already has specialization for `extend`ing a `Vec`.
            // No need for a method like `push_back_vec`.
            Chunks::Single(items) => collection.push_back_iter(items),
            Chunks::Plural { chunks, len } => collection.push_back_linked_vec(chunks, len),
        }
    }
}

pub struct Combiner(());

impl<'a, C, T> Consumer<'a, C, T> {
    #[inline]
    pub(crate) fn new(collection: &'a mut C) -> Self {
        Self {
            collection: CellOptRefMut::from(Some(collection)),
            _marker: PhantomData,
        }
    }
}

impl<'a, C, T> IntoCollectorBase for Consumer<'a, C, T>
where
    C: Collection<T>,
{
    type Output = Output<'a, C, T>;

    type IntoCollector = Serial<'a, C, T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        match self.collection.into_inner() {
            Some(collection) => Serial::LeftMost(collection),
            None => Serial::Right(vec![]),
        }
    }
}

impl<'a, C, T> plumbing::Consumer for Consumer<'a, C, T>
where
    // For most collections in the standard library,
    // the collection being Send only needs their elements to be Send.
    // It is slightly problematic for something like HashSet
    // because the build hasher needs to be Send as well,
    // but in practice build hashers are mostly Send,
    // and the user can build a custom one fairly easily anyway.
    C: Collection<T> + Send,
    T: Send,
{
    type Combiner = Combiner;

    #[inline]
    fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
        use plumbing::UnindexedConsumer;
        (self.split_off_left(), self.to_combiner())
    }
}

impl<C, T> plumbing::UnindexedConsumer for Consumer<'_, C, T>
where
    C: Collection<T> + Send,
    T: Send,
{
    #[inline]
    fn split_off_left(&self) -> Self {
        Consumer {
            collection: self.collection.take().into(),
            _marker: PhantomData,
        }
    }

    #[inline]
    fn to_combiner(&self) -> Self::Combiner {
        Combiner(())
    }
}

impl<'a, C, T> plumbing::Combiner<Output<'a, C, T>> for Combiner
where
    C: Collection<T>,
{
    #[inline]
    fn combine(self, left: &mut Output<'a, C, T>, right: Output<'a, C, T>) {
        debug_assert!(
            right.collection.is_none(),
            "only the left-most output can have a mutable reference to the collection",
        );

        left.chunks.extend(right.chunks);
    }
}

impl<'a, C, T> CollectorBase for Serial<'a, C, T>
where
    C: Collection<T>,
{
    type Output = Output<'a, C, T>;

    #[inline]
    fn finish(self) -> Self::Output {
        match self {
            Self::LeftMost(collection) => Output {
                collection: Some(collection),
                chunks: Chunks::Single(vec![]),
            },
            Self::Right(chunk) => Output {
                collection: None,
                chunks: Chunks::Single(chunk),
            },
        }
    }

    plumbing::finish_boxed_impl! {}

    #[inline]
    fn reserve(&mut self, additional: usize) {
        match self {
            Self::LeftMost(collection) => collection.reserve(additional),
            Self::Right(chunk) => chunk.reserve(additional),
        }
    }
}

impl<C, T> Collector<T> for Serial<'_, C, T>
where
    C: Collection<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self {
            Self::LeftMost(collection) => collection.push_back(item),
            Self::Right(chunk) => chunk.push(item),
        }

        ControlFlow::Continue(())
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            match self {
                Self::LeftMost(collection) => collection.assume_reserved_push_back(item),
                Self::Right(chunk) => crate::vec::push_unchecked(chunk, item),
            }
        }

        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match self {
            Self::LeftMost(collection) => collection.push_back_iter(items),
            Self::Right(chunk) => chunk.extend(items),
        }

        ControlFlow::Continue(())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self {
            Self::LeftMost(collection) => {
                collection.push_back_iter(items);
                Output {
                    collection: Some(collection),
                    chunks: Chunks::Single(vec![]),
                }
            }
            Self::Right(mut chunk) => {
                chunk.extend(items);
                Output {
                    collection: None,
                    chunks: Chunks::Single(chunk),
                }
            }
        }
    }
}

impl<'a, 'i, C, T> Collector<&'i T> for Serial<'a, C, T>
where
    C: Collection<T>,
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        Collector::<T>::collect(self, item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        unsafe { Collector::<T>::assume_reserved_collect(self, item) }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i T>) -> ControlFlow<()> {
        match self {
            Self::LeftMost(collection) => collection.push_back_iter_ref(items),
            Self::Right(chunk) => chunk.extend(items),
        }

        ControlFlow::Continue(())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = &'i T>) -> Self::Output {
        match self {
            Self::LeftMost(collection) => {
                collection.push_back_iter_ref(items);
                Output {
                    collection: Some(collection),
                    chunks: Chunks::Single(vec![]),
                }
            }
            Self::Right(mut chunk) => {
                chunk.extend(items);
                Output {
                    collection: None,
                    chunks: Chunks::Single(chunk),
                }
            }
        }
    }
}

impl<'a, 'i, C, T> Collector<&'i mut T> for Serial<'a, C, T>
where
    C: Collection<T>,
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        Collector::<T>::collect(self, item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        unsafe { Collector::<T>::assume_reserved_collect(self, item) }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut T>) -> ControlFlow<()> {
        let items = items.into_iter().map(|&mut item| item);

        match self {
            Self::LeftMost(collection) => collection.push_back_iter(items),
            Self::Right(chunk) => chunk.extend(items),
        }

        ControlFlow::Continue(())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = &'i mut T>) -> Self::Output {
        let items = items.into_iter().map(|&mut item| item);

        match self {
            Self::LeftMost(collection) => {
                collection.push_back_iter(items);
                Output {
                    collection: Some(collection),
                    chunks: Chunks::Single(vec![]),
                }
            }
            Self::Right(mut chunk) => {
                chunk.extend(items);
                Output {
                    collection: None,
                    chunks: Chunks::Single(chunk),
                }
            }
        }
    }
}
