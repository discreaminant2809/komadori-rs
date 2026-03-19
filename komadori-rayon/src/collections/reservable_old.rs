use std::{cell::RefCell, collections::LinkedList, ops::ControlFlow};

use crate::collector::plumbing::{self, UnindexedConsumerBase};

#[inline]
pub fn unindexed_linked_vec_consumer<E, T>(
    extender: E,
) -> impl plumbing::UnindexedConsumer<T, Output = Output<E, T>>
where
    E: Extender<T> + Send,
    T: Send,
{
    Consumer {
        state: ConsumerState::LeftMost(extender).into(),
    }
}

pub struct Output<E, T> {
    state: OutputState<E, T>,
}

impl<E, T> Output<E, T> {
    #[inline]
    pub fn unwrap(self) -> E {
        match self.state {
            OutputState::LeftMost(extend) => extend,
            OutputState::Right(_) => panic!("output is not fully reduced"),
        }
    }
}

enum OutputState<E, T> {
    LeftMost(E),
    Right(LinkedList<Vec<T>>),
}

/// Just [`Extend`], but implemented automatically for `&mut E`.
pub trait Extender<T> {
    fn extend_one(&mut self, item: T);
    fn extend_many(&mut self, items: impl IntoIterator<Item = T>);
}

struct Consumer<E, T> {
    state: RefCell<ConsumerState<E, T>>,
}

struct Combiner;

enum ConsumerState<E, T> {
    LeftMost(E),
    Right(Vec<T>),
}

impl<E, T> Extender<T> for &mut E
where
    E: Extender<T>,
{
    #[inline]
    fn extend_one(&mut self, item: T) {
        E::extend_one(self, item);
    }

    #[inline]
    fn extend_many(&mut self, items: impl IntoIterator<Item = T>) {
        E::extend_many(self, items);
    }
}

impl<E, T> komadori::collector::CollectorBase for Consumer<E, T> {
    type Output = Output<E, T>;

    fn finish(self) -> Self::Output {
        Output {
            state: match self.state.into_inner() {
                ConsumerState::LeftMost(extender) => OutputState::LeftMost(extender),
                ConsumerState::Right(chunk) => OutputState::Right([chunk].into()),
            },
        }
    }
}

impl<E, T> komadori::collector::Collector<T> for Consumer<E, T>
where
    E: Extender<T>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match self.state.get_mut() {
            ConsumerState::LeftMost(extender) => extender.extend_one(item),
            ConsumerState::Right(chunk) => chunk.push(item),
        }

        ControlFlow::Continue(())
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        match self.state.get_mut() {
            ConsumerState::LeftMost(extender) => extender.extend_many(items),
            ConsumerState::Right(chunk) => chunk.extend(items),
        }

        ControlFlow::Continue(())
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        match self.state.into_inner() {
            ConsumerState::LeftMost(mut extender) => {
                extender.extend_many(items);
                Output {
                    state: OutputState::LeftMost(extender),
                }
            }
            ConsumerState::Right(mut chunk) => {
                chunk.extend(items);
                Output {
                    state: OutputState::Right([chunk].into()),
                }
            }
        }
    }
}

impl<E, T> plumbing::ConsumerBase for Consumer<E, T>
where
    E: Extender<T> + Send,
    T: Send,
{
    type Combiner = Combiner;

    #[inline]
    fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
        (self.split_off_left(), Combiner)
    }
}

impl<E, T> plumbing::UnindexedConsumerBase for Consumer<E, T>
where
    E: Extender<T> + Send,
    T: Send,
{
    #[inline]
    fn split_off_left(&self) -> Self {
        let mut state = self.state.borrow_mut();
        Self {
            state: std::mem::replace(&mut *state, ConsumerState::Right(vec![])).into(),
        }
    }

    #[inline]
    fn to_combiner(&self) -> Self::Combiner {
        Combiner
    }
}

impl<E, T> plumbing::Combiner<Output<E, T>> for Combiner
where
    E: Extender<T>,
{
    fn combine(self, left: &mut Output<E, T>, right: Output<E, T>) {
        let OutputState::Right(mut chunks) = right.state else {
            panic!("outputs were combined in an incorrect order");
        };

        match &mut left.state {
            OutputState::LeftMost(extender) => {
                for chunk in chunks {
                    // Specialization for extending a Vec, hopefully.
                    extender.extend_many(chunk);
                }
            }
            OutputState::Right(left_chunks) => left_chunks.append(&mut chunks),
        }
    }
}
