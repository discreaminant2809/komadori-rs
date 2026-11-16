#![allow(missing_debug_implementations)]

use std::{cell::Cell, collections::LinkedList, ops::ControlFlow};

use crate::collector::plumbing::{
    BasicUnindexedConsumer, Collector, CollectorBase, OpaqueUnindexedConsumer, finish_boxed_impl,
};

use super::{Collection, CollectionBase};

// The entire idea is that we keep a mutable reference to the original collection
// in the "left most" consumer.
// This way, we can be heavily optimized in `par_collector.into_collector()`.
// Based on the benchmark, it performs just as good as `rayon`'s approach in average,
// which simply (can't call it "naively," tho) just uses linked lists of vecs.

pub fn unindexed<PC, SC>(primary: PC) -> OpaqueUnindexedConsumer!(Serial<PC, SC>)
where
    PC: CollectionBase + Send,
    SC: CollectionBase + Default + Send,
{
    BasicUnindexedConsumer {
        state: Cell::new(Some(primary)),
        split_f: |primary| Cell::new(primary.take()),
        combiner_f: |_| Output::extend,
        ma_f: |_, request| request,
        collector_f: |primary| match primary.into_inner() {
            Some(primary) => Serial::LeftMost(primary),
            None => Serial::Right(SC::default()),
        },
    }
}

pub enum Serial<PC, SC> {
    LeftMost(PC),
    Right(SC),
}

pub struct Output<PC, SC> {
    primary: Option<PC>,
    chunks: Chunks<SC>,
}

enum Chunks<SC> {
    // This state exists so that we can avoid another indirection of `LinkedList`.
    // A win if we somehow only ever have 1 chunk (rare).
    Single(SC),
    Plural { chunks: LinkedList<SC>, len: usize },
}

impl<PC, SC> CollectorBase for Serial<PC, SC>
where
    PC: CollectionBase,
    SC: CollectionBase + Default,
{
    type Output = Output<PC, SC>;

    #[inline]
    fn finish(self) -> Self::Output {
        match self {
            Self::LeftMost(primary) => Output {
                primary: Some(primary),
                chunks: Chunks::Single(SC::default()),
            },
            Self::Right(secondary) => Output {
                primary: None,
                chunks: Chunks::Single(secondary),
            },
        }
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        match self {
            Self::LeftMost(prinary) => prinary.reserve(additional),
            Self::Right(secondary) => secondary.reserve(additional),
        }
    }

    finish_boxed_impl! {}
}

impl<PC, SC, T> Collector<T> for Serial<PC, SC>
where
    PC: Collection<T>,
    SC: Collection<T> + Default,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self {
            Self::LeftMost(primary) => primary.push(item),
            Self::Right(secondary) => secondary.push(item),
        }

        ControlFlow::Continue(())
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        // SAFETY: We've overriden `CollectorBase::reserve` to reserve correctly.
        match self {
            Self::LeftMost(primary) => unsafe { primary.assume_reserved_push(item) },
            Self::Right(secondary) => unsafe { secondary.assume_reserved_push(item) },
        }

        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match self {
            Self::LeftMost(primary) => primary.push_many(items),
            Self::Right(secondary) => secondary.push_many(items),
        }

        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self {
            Self::LeftMost(mut primary) => {
                primary.push_many(items);
                Output {
                    primary: Some(primary),
                    chunks: Chunks::Single(SC::default()),
                }
            }
            Self::Right(mut secondary) => {
                secondary.push_many(items);
                Output {
                    primary: None,
                    chunks: Chunks::Single(secondary),
                }
            }
        }
    }
}

impl<PC, SC> Output<PC, SC>
where
    PC: CollectionBase,
    SC: CollectionBase + Default,
{
    fn extend(&mut self, other: Self) {
        self.chunks.extend(other.chunks);
    }

    pub(crate) fn finalize(
        self,
        single: impl FnOnce(PC, SC),
        plural: impl FnOnce(PC, LinkedList<SC>, usize),
    ) {
        let Some(primary) = self.primary else {
            return;
        };

        match self.chunks {
            Chunks::Single(chunk) => single(primary, chunk),
            Chunks::Plural { chunks, len } => plural(primary, chunks, len),
        }
    }
}

impl<SC> Chunks<SC>
where
    SC: CollectionBase + Default,
{
    fn extend(&mut self, right: Self) {
        match (&mut *self, right) {
            (Self::Single(left_chunk), Self::Single(right_chunk)) if left_chunk.is_empty() => {
                // Use the one with more capacity to be more efficient.
                // If the right chunk has some elements, we must use it,
                // so that we don't spend time copying elements.
                if !right_chunk.is_empty() || left_chunk.capacity() < right_chunk.capacity() {
                    *left_chunk = right_chunk;
                }
            }
            (Self::Single(left_chunk), Self::Single(right_chunk)) if right_chunk.is_empty() => {
                // We won't waste a `LinkedList` just to combine with an empty chunk.
            }
            (Self::Single(left_chunk), Self::Single(right_chunk)) => {
                let left_chunk = std::mem::take(left_chunk);
                *self = Self::Plural {
                    len: left_chunk.len() + right_chunk.len(),
                    chunks: LinkedList::from([left_chunk, right_chunk]),
                };
            }
            (Self::Single(left_chunk), Self::Plural { mut chunks, len }) => {
                let left_chunk = std::mem::take(left_chunk);
                *self = Self::Plural {
                    len: left_chunk.len() + len,
                    chunks: {
                        chunks.push_front(left_chunk);
                        chunks
                    },
                };
            }

            (Self::Plural { chunks, len }, Self::Single(chunk)) => {
                if !chunk.is_empty() {
                    *len += chunk.len();
                    chunks.push_back(chunk);
                }

                // Otherwise that chunk is useless and not worth appending.
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
