use super::{IndexedParallelIterator, ParallelIterator};

#[derive(Clone)]
pub struct Chain<I1, I2> {
    iter1: I1,
    iter2: I2,
}

impl<I1, I2> Chain<I1, I2> {
    pub(super) fn new(iter1: I1, iter2: I2) -> Self {
        Self { iter1, iter2 }
    }
}

impl<I1, I2> ParallelIterator for Chain<I1, I2>
where
    I1: ParallelIterator,
    I2: ParallelIterator<Item = I1::Item>,
{
    type Item = I1::Item;

    fn take_producer(&mut self) -> impl super::Producer<Item = Self::Item> {
        Producer::Both {
            producer1: self.iter1.take_producer(),
            producer2: self.iter2.take_producer(),
        }
    }

    fn count(self) -> usize {
        self.iter1.count() + self.iter2.count()
    }
}

impl<I1, I2> IndexedParallelIterator for Chain<I1, I2>
where
    I1: IndexedParallelIterator,
    I2: IndexedParallelIterator<Item = I1::Item>,
{
    fn indexed_producer(&mut self) -> impl super::IndexedProducer<Item = Self::Item> {
        Producer::Both {
            producer1: self.iter1.indexed_producer(),
            producer2: self.iter2.indexed_producer(),
        }
    }

    // It may overflow, but for our use cases we don't produce
    // billions of items anyway.
    fn len(&self) -> usize {
        self.iter1.len() + self.iter2.len()
    }
}

enum Producer<P1, P2> {
    Invalid,
    FirstOnly(P1),
    SecondOnly(P2),
    Both { producer1: P1, producer2: P2 },
}

impl<P1, P2> super::Producer for Producer<P1, P2>
where
    P1: super::Producer,
    P2: super::Producer<Item = P1::Item>,
{
    type Item = P1::Item;

    fn into_iter(self) -> impl Iterator<Item = Self::Item> {
        let (producer1, producer2) = match self {
            Self::Invalid => unreachable!("invalid state"),
            Self::FirstOnly(producer1) => (Some(producer1), None),
            Self::SecondOnly(producer2) => (None, Some(producer2)),
            Self::Both { producer1, producer2 } => (Some(producer1), Some(producer2)),
        };

        producer1
            .into_iter()
            .flat_map(super::Producer::into_iter)
            .chain(producer2.into_iter().flat_map(super::Producer::into_iter))
    }

    fn split_off_left(&mut self) -> Self {
        match self {
            Self::Invalid => unreachable!("invalid state"),
            Self::Both { .. } => {
                let Self::Both { producer1, producer2 } = std::mem::replace(self, Self::Invalid) else {
                    unreachable!("invalid state")
                };
                *self = Self::SecondOnly(producer2);
                Self::FirstOnly(producer1)
            }
            Self::FirstOnly(producer1) => Self::FirstOnly(producer1.split_off_left()),
            Self::SecondOnly(producer2) => Self::SecondOnly(producer2.split_off_left()),
        }
    }
}

impl<P1, P2> super::IndexedProducer for Producer<P1, P2>
where
    P1: super::IndexedProducer,
    P2: super::IndexedProducer<Item = P1::Item>,
{
    type Item = P1::Item;

    fn into_iter(self) -> impl Iterator<Item = Self::Item> {
        let (producer1, producer2) = match self {
            Self::Invalid => unreachable!("invalid state"),
            Self::FirstOnly(producer1) => (Some(producer1), None),
            Self::SecondOnly(producer2) => (None, Some(producer2)),
            Self::Both { producer1, producer2 } => (Some(producer1), Some(producer2)),
        };

        producer1
            .into_iter()
            .flat_map(super::IndexedProducer::into_iter)
            .chain(producer2.into_iter().flat_map(super::IndexedProducer::into_iter))
    }

    fn len(&self) -> usize {
        match self {
            Self::Invalid => unreachable!("invalid state"),
            Self::FirstOnly(producer1) => producer1.len(),
            Self::SecondOnly(producer2) => producer2.len(),
            // It may overflow, but for our use cases we don't produce
            // billions of items anyway.
            Self::Both { producer1, producer2 } => producer1.len() + producer2.len(),
        }
    }

    fn split_off_left_at(&mut self, index: usize) -> Self {
        match self {
            Self::Invalid => unreachable!("invalid state"),
            Self::Both { .. } => {
                let Self::Both {
                    mut producer1,
                    mut producer2,
                } = std::mem::replace(self, Self::Invalid)
                else {
                    unreachable!("invalid state")
                };

                if index < producer1.len() {
                    let left_producer1 = producer1.split_off_left_at(index);
                    *self = Self::Both { producer1, producer2 };
                    Self::FirstOnly(left_producer1)
                } else if index == producer1.len() {
                    *self = Self::SecondOnly(producer2);
                    Self::FirstOnly(producer1)
                } else {
                    let left_producer2 = producer2.split_off_left_at(index - producer1.len());
                    *self = Self::SecondOnly(producer2);
                    Self::Both {
                        producer1,
                        producer2: left_producer2,
                    }
                }
            }
            Self::FirstOnly(producer1) => Self::FirstOnly(producer1.split_off_left_at(index)),
            Self::SecondOnly(producer2) => Self::SecondOnly(producer2.split_off_left_at(index)),
        }
    }
}
