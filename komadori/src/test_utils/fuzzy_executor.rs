use std::fmt::Debug;

use proptest::{
    strategy::{Just, Strategy},
    test_runner::TestCaseResult,
};

use crate::collector::{Collector, CollectorBase, IntoCollectorBase};

use super::{CollectorModel, FuzzyExecSeq, FuzzyExecSeqStrategy};

pub struct FuzzyExecutor<ID, CD, IF, CF>
where
    ID: Clone + Debug,
    CD: Clone + Debug,
    IF: TriIteratorFactory<ID>,
    CF: for<'a> CollectorFactory<CD, <IF as DefineItem<'a, ID>>::Item>,
{
    iter_data: ID,
    collector_data: CD,
    seq: FuzzyExecSeq,
    iter_f: IF,
    collector_f: CF,
}

pub trait DefineItem<'d, D: ?Sized, Binder = &'d mut D> {
    // `PartialEq + Debug` because we need to compare the remaining
    // of two iterators, and debug on mismatch.
    type Item: PartialEq + Debug;
}

// One iterator is for collector, and another is for the execution predicate.
pub trait TriIteratorFactory<D: ?Sized>: for<'d> DefineItem<'d, D> + Clone {
    // `&self` because `prop_*` adapter only accept `Fn`.
    // We don't want to `use` the lifetime of `self`.
    fn three_iters<'d>(
        &self,
        data: &'d mut D,
    ) -> [impl Iterator<Item = <Self as DefineItem<'d, D>>::Item> + use<'d, D, Self>; 3];

    /// This is used to create a [`FuzzyExecSeq`].
    // `&self` because `prop_*` adapter only accept `Fn`.
    // We don't want to `use` the lifetime of `self`.
    fn one_iter<'d>(
        &self,
        data: &'d mut D,
    ) -> impl Iterator<Item = <Self as DefineItem<'d, D>>::Item> + use<'d, D, Self>;
}

impl<'d, F, D, I> DefineItem<'d, D> for F
where
    D: ?Sized,
    // Deliberately `&D` to avoid mutation.
    // We can't return an iterator that use that reference anyway.
    F: Fn(&D) -> I + Clone,
    I: IntoIterator<Item: PartialEq + Debug, IntoIter: Clone>,
{
    type Item = I::Item;
}

impl<F, D, I> TriIteratorFactory<D> for F
where
    D: ?Sized,
    // Deliberately `&D` to avoid mutation.
    // We can't return an iterator that use that reference anyway.
    F: Fn(&D) -> I + Clone,
    I: IntoIterator<Item: PartialEq + Debug, IntoIter: Clone>,
{
    fn three_iters<'d>(
        // `&self` because `prop_*` adapter only accept `Fn`.
        &self,
        data: &'d mut D,
        // We don't want to `use` the lifetime of `self`.
    ) -> [impl Iterator<Item = <Self as DefineItem<'d, D>>::Item> + use<'d, F, D, I>; 3] {
        let iter = self(data).into_iter();
        [iter.clone(), iter.clone(), iter]
    }

    fn one_iter<'d>(
        // `&self` because `prop_*` adapter only accept `Fn`.
        &self,
        data: &'d mut D,
        // We don't want to `use` the lifetime of `self`.
    ) -> impl Iterator<Item = <Self as DefineItem<'d, D>>::Item> + use<'d, F, D, I> {
        self(data).into_iter()
    }
}

pub trait DefineCollector<'d, D: ?Sized, Binder = &'d mut D> {
    // Deliberately have the implementors name the collector type
    // because they can then be clear about what collector is being tested,
    // and we can use the trick that's similar to `IntoCollector: IntoCollectorBase`.
    type Collector: CollectorBase<Output = Self::Output>;
    type Output: Debug;
}

pub trait CollectorFactoryBase<D: ?Sized>: for<'d> DefineCollector<'d, D> + Clone {
    fn collector<'d>(&self, data: &'d mut D) -> <Self as DefineCollector<'d, D>>::Collector;
}

pub trait CollectorFactory<D: ?Sized, T>: CollectorFactoryBase<D, Collector: Collector<T>> {}
impl<CF, D, T> CollectorFactory<D, T> for CF where
    CF: CollectorFactoryBase<D, Collector: Collector<T>>
{
}

impl<'d, F, D, C> DefineCollector<'d, D> for F
where
    D: ?Sized,
    // Deliberately `&D` to avoid mutation.
    // We can't return a collector that use that reference anyway.
    F: Fn(&D) -> C + Clone,
    C: IntoCollectorBase<Output: Debug>,
{
    type Collector = C::IntoCollector;
    type Output = C::Output;
}

impl<F, D, C> CollectorFactoryBase<D> for F
where
    D: ?Sized,
    // Deliberately `&D` to avoid mutation.
    // We can't return a collector that use that reference anyway.
    F: Fn(&D) -> C + Clone,
    C: IntoCollectorBase<Output: Debug>,
{
    fn collector<'d>(&self, data: &'d mut D) -> <Self as DefineCollector<'d, D>>::Collector {
        self(data).into_collector()
    }
}

impl<ID, CD, IF, CF> FuzzyExecutor<ID, CD, IF, CF>
where
    ID: Clone + Debug,
    CD: Clone + Debug,
    IF: TriIteratorFactory<ID>,
    CF: for<'a> CollectorFactory<CD, <IF as DefineItem<'a, ID>>::Item>,
{
    /// Used when we need to isolate a test case.
    #[allow(dead_code)]
    pub fn new(
        iter_data: ID,
        collector_data: CD,
        seq: FuzzyExecSeq,
        iter_f: IF,
        collector_f: CF,
    ) -> Self {
        Self {
            iter_data,
            collector_data,
            seq,
            iter_f,
            collector_f,
        }
    }

    pub fn strategy(
        iter_data: impl Strategy<Value = ID>,
        collector_data: impl Strategy<Value = CD>,
        iter_f: IF,
        collector_f: CF,
    ) -> impl Strategy<Value = Self> {
        let iter_f1 = iter_f.clone();

        (iter_data, collector_data)
            .prop_flat_map(move |(mut iter_data, collector_data)| {
                let iter_f = &iter_f1;
                let iter = iter_f.one_iter(&mut iter_data);
                (
                    FuzzyExecSeqStrategy::new(iter.count()),
                    Just(iter_data),
                    Just(collector_data),
                )
            })
            .prop_map(move |(seq, iter_data, collector_data)| Self {
                iter_data,
                collector_data,
                seq,
                iter_f: iter_f.clone(),
                collector_f: collector_f.clone(),
            })
    }

    pub fn execute<
        'd,
        EO: Debug + PartialEq,
        P: CollectorModel<
                <IF as DefineItem<'d, ID>>::Item,
                EO,
                <CF as DefineCollector<'d, CD>>::Output,
            >,
    >(
        &'d mut self,
        expected_output_f: impl FnOnce(
            &mut dyn Iterator<Item = <IF as DefineItem<'d, ID>>::Item>,
            &CD,
        ) -> EO,
        // Intentionally shared reference to avoid accidental mutation.
        // We can't return anything that borrow the lifetime of it anyway.
        model_f: impl FnOnce(&CD) -> P,
    ) -> TestCaseResult {
        let [iter, mut expected_remaining, iter_for_model] = self
            .iter_f
            .three_iters(&mut self.iter_data)
            .map(Iterator::fuse);

        let pred = model_f(&self.collector_data);
        let expected_output = expected_output_f(&mut expected_remaining, &self.collector_data);
        let collector = self.collector_f.collector(&mut self.collector_data);

        super::fuzzy_execute(
            iter,
            iter_for_model,
            collector,
            expected_remaining,
            expected_output,
            &self.seq,
            pred,
        )?;
        Ok(())
    }
}

impl<ID, CD, IF, CF> Debug for FuzzyExecutor<ID, CD, IF, CF>
where
    ID: Clone + Debug,
    CD: Clone + Debug,
    IF: TriIteratorFactory<ID>,
    CF: for<'a> CollectorFactory<CD, <IF as DefineItem<'a, ID>>::Item>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuzzyExecutor")
            .field("iter_data", &self.iter_data)
            .field("colector_data", &self.collector_data)
            .field("seq", &self.seq)
            .finish()
    }
}
