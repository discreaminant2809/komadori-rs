use super::ParallelIterator;

#[derive(Clone)]
pub struct Filter<I, F> {
    iter: I,
    f: F,
}

impl<I, F> Filter<I, F> {
    pub(super) fn new(iter: I, f: F) -> Self {
        Self { iter, f }
    }
}

impl<I, F> ParallelIterator for Filter<I, F>
where
    I: ParallelIterator,
    F: Fn(&I::Item) -> bool,
{
    type Item = I::Item;

    fn take_producer(&mut self) -> impl super::Producer<Item = Self::Item> {
        Producer {
            producer: self.iter.take_producer(),
            f: &self.f,
        }
    }
}

struct Producer<'a, P, F> {
    producer: P,
    f: &'a F,
}

impl<P, F> super::Producer for Producer<'_, P, F>
where
    P: super::Producer,
    F: Fn(&P::Item) -> bool,
{
    type Item = P::Item;

    fn into_iter(self) -> impl Iterator<Item = Self::Item> {
        self.producer.into_iter().filter(self.f)
    }

    fn split_off_left(&mut self) -> Self {
        Self {
            producer: self.producer.split_off_left(),
            f: self.f,
        }
    }
}
