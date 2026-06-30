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
        chunks.into_iter().for_each(|chunk| self.push_back_iter(chunk));
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

pub enum Output<'a, C, T> {
    LeftMost(&'a mut C),
    Right { chunks: LinkedList<Vec<T>>, len: usize },
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

impl<C, T> Output<'_, C, T> {
    #[inline]
    pub fn is_left_most(&self) -> bool {
        matches!(self, Self::LeftMost(_))
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
        let Output::Right {
            chunks: mut right_chunks,
            len: right_len,
        } = right
        else {
            panic!("the right-side output must be a linked list of vecs");
        };

        match left {
            Output::LeftMost(collection) => collection.push_back_linked_vec(right_chunks, right_len),
            Output::Right { chunks, len } => {
                chunks.append(&mut right_chunks);
                *len += right_len;
            }
        }
    }
}

impl<'a, C, T> CollectorBase for Serial<'a, C, T>
where
    C: Collection<T>,
{
    type Output = Output<'a, C, T>;

    fn finish(self) -> Self::Output {
        match self {
            Self::LeftMost(collection) => Output::LeftMost(collection),
            Self::Right(chunk) => Output::Right {
                len: chunk.len(),
                chunks: if chunk.is_empty() {
                    LinkedList::new()
                } else {
                    LinkedList::from([chunk])
                },
            },
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
                Output::LeftMost(collection)
            }
            Self::Right(mut chunk) => {
                chunk.extend(items);
                Output::Right {
                    len: chunk.len(),
                    chunks: if chunk.is_empty() {
                        LinkedList::new()
                    } else {
                        LinkedList::from([chunk])
                    },
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
                Output::LeftMost(collection)
            }
            Self::Right(mut chunk) => {
                chunk.extend(items);
                Output::Right {
                    len: chunk.len(),
                    chunks: if chunk.is_empty() {
                        LinkedList::new()
                    } else {
                        LinkedList::from([chunk])
                    },
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
                Output::LeftMost(collection)
            }
            Self::Right(mut chunk) => {
                chunk.extend(items);
                Output::Right {
                    len: chunk.len(),
                    chunks: if chunk.is_empty() {
                        LinkedList::new()
                    } else {
                        LinkedList::from([chunk])
                    },
                }
            }
        }
    }
}
