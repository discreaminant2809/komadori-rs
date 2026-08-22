use std::{any::type_name, fmt::Debug, ops::ControlFlow};

use komadori::prelude::*;

use crate::{
    collector::plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    helpers::{unique, unique_unindexed},
};

use super::{
    IntoParallelCollectorBase, IntoUnindexedParallelCollectorBase, ParallelCollectorBase,
    UnindexedParallelCollectorBase,
};

/// A custom parallel collector built from an existing one.
///
/// This can also be used as an unindexed parallel collector,
/// but you can override the unindexed path with [`also_unindexed()`](Self::also_unindexed).
///
/// Be aware that the order the closures are called is unspecified,
/// so you should not rely on it.
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, collector::Custom};
/// use std::{collections::HashSet, hash::Hash};
///
/// fn hash_set_par_collector<T: Hash + Eq + Send>(
/// ) -> impl UnindexedParallelCollector<T, Output = HashSet<T>> {
///     Custom::new(
///         HashSet::new(),
///         |_, request| request,
///         |_| vec![],
///         |set, items| set.extend(items),
///     )
///     // It's is quite wasteful in the unindexed path, because
///     // we prematurely concatenate to Vec then push the Vec's items
///     // to the HashSet, instead of concatenating to the HashSet directly.
/// }
///
/// let set = [1, 4, 5, 4, 3, 1, 2]
///     .into_par_iter()
///     .feed_into(hash_set_par_collector());
///
/// assert_eq!(set, HashSet::from([1, 2, 3, 4, 5]));
/// ```
#[derive(Clone)]
pub struct Custom<S, MA, I, IF, IC> {
    state: S,
    max_afford: MA,
    indexed: Option<I>,
    indexed_f: IF,
    indexed_commit: IC,
}

/// A custom parallel collector built from existing ones,
/// with distinct indexed and unindexed paths from
/// two different parallel collectors.
///
/// This `struct` is created by [`Custom::also_unindexed()`].
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori::prelude::*;
/// use komadori_rayon::{prelude::*, collector::Custom, iter::ParReduce};
/// use std::{collections::{HashSet, LinkedList}, ops::ControlFlow, hash::Hash};
///
/// fn hash_set_par_collector<T: Hash + Eq + Send>(
/// ) -> impl UnindexedParallelCollector<T, Output = HashSet<T>> {
///     Custom::new(
///         HashSet::new(),
///         |_| ControlFlow::Continue(()),
///         |_| vec![],
///         |set, items| set.extend(items),
///     )
///     .also_unindexed(
///         |_| {
///             ParReduce::new(|(len1, chunk1), (len2, mut chunk2): (usize, LinkedList<_>)| {
///                 chunk1.append(&mut chunk2);
///                 *len1 += len2;
///             })
///             .nest_local_with((), |_| {
///                 vec![]
///                     .into_collector()
///                     .map_output(|v| (
///                         v.len(),
///                         LinkedList::from_iter((!v.is_empty()).then_some(v)),
///                     ))
///             })
///         },
///         |set, len_chunks| {
///             let (len, chunks) = len_chunks.unwrap_or_default();
///             set.reserve(len);
///             set.extend(chunks.into_iter().flatten());
///         },
///     )
/// }
///
/// let set = [1, 4, 5, 4, 3, 1, 2]
///     .into_par_iter()
///     .feed_into(
///         hash_set_par_collector()
///             // To test the unindexed path!
///             .unindexed_only()
///     );
///
/// assert_eq!(set, HashSet::from([1, 2, 3, 4, 5]));
/// ```
#[derive(Clone)]
pub struct CustomAlsoUnindexed<S, MA, I, IF, IC, U, UF, UC> {
    state: S,
    max_afford: MA,
    indexed: Option<I>,
    indexed_f: IF,
    indexed_commit: IC,
    unindexed: Option<U>,
    unindexed_f: UF,
    unindexed_commit: UC,
}

impl<S, MA, I, IF, IC> Custom<S, MA, I::IntoParCollector, IF, IC>
where
    MA: Fn(&S, usize) -> usize,
    I: IntoParallelCollectorBase,
    IF: FnMut(&mut S) -> I,
    IC: FnMut(&mut S, I::Output),
{
    /// Creates a new instance of this parallel collector with a state,
    /// a closure that determines whether to stop,
    /// a closure that provides an existing parallel collector,
    /// and a closure that commits its output back to the state.
    pub fn new(state: S, max_afford: MA, indexed_f: IF, indexed_commit: IC) -> Self {
        Self {
            state,
            max_afford,
            indexed: None,
            indexed_f,
            indexed_commit,
        }
    }

    /// Creates a new instance of [`CustomAlsoUnindexed`] from this parallel collector
    /// with a closure that provides an existing unindexed parallel collector,
    /// and a closure that commits its output back to the state.
    pub fn also_unindexed<U, UF, UC>(
        self,
        unindexed_f: UF,
        unindexed_commit: UC,
    ) -> CustomAlsoUnindexed<S, MA, I::IntoParCollector, IF, IC, U::IntoParCollector, UF, UC>
    where
        U: IntoUnindexedParallelCollectorBase,
        UF: FnMut(&mut S) -> U,
        UC: FnMut(&mut S, U::Output),
    {
        CustomAlsoUnindexed {
            state: self.state,
            max_afford: self.max_afford,
            indexed: self.indexed,
            indexed_f: self.indexed_f,
            indexed_commit: self.indexed_commit,
            unindexed: None,
            unindexed_f,
            unindexed_commit,
        }
    }

    #[inline]
    fn clean_up(&mut self) {
        if let Some(indexed) = self.indexed.take() {
            (self.indexed_commit)(&mut self.state, indexed.finish());
        }
    }
}

impl<S, MA, I, IF, IC, U, UF, UC>
    CustomAlsoUnindexed<S, MA, I::IntoParCollector, IF, IC, U::IntoParCollector, UF, UC>
where
    MA: Fn(&S, usize) -> usize,
    I: IntoParallelCollectorBase,
    IF: FnMut(&mut S) -> I,
    IC: FnMut(&mut S, I::Output),
    U: IntoUnindexedParallelCollectorBase,
    UF: FnMut(&mut S) -> U,
    UC: FnMut(&mut S, U::Output),
{
    #[inline]
    fn clean_up(&mut self) {
        // We must NOT be able to have both being Some.
        // If so, it's a BUG!
        debug_assert!(
            self.indexed.is_none() || self.unindexed.is_none(),
            "`indexed` and `unindexed` are somehow both `Some`",
        );

        if let Some(indexed) = self.indexed.take() {
            (self.indexed_commit)(&mut self.state, indexed.finish());
        }

        if let Some(unindexed) = self.unindexed.take() {
            (self.unindexed_commit)(&mut self.state, unindexed.finish());
        }
    }
}

impl<S, MA, I, IF, IC> Debug for Custom<S, MA, I, IF, IC>
where
    S: Debug,
    I: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Custom")
            .field("state", &self.state)
            .field("max_afford", &type_name::<MA>())
            .field("indexed", &self.indexed)
            .field("indexed_f", &type_name::<IF>())
            .field("indexed_commit", &type_name::<IC>())
            .finish()
    }
}

impl<S, MA, I, IF, IC, U, UF, UC> Debug for CustomAlsoUnindexed<S, MA, I, IF, IC, U, UF, UC>
where
    S: Debug,
    I: Debug,
    U: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomAlsoUnindexed")
            .field("state", &self.state)
            .field("max_afford", &type_name::<MA>())
            .field("indexed", &self.indexed)
            .field("indexed_f", &type_name::<IF>())
            .field("indexed_commit", &type_name::<IC>())
            .field("unindexed", &self.unindexed)
            .field("unindexed_f", &type_name::<UF>())
            .field("unindexed_commit", &type_name::<UC>())
            .finish()
    }
}

impl<'a, S, MA, I, IF, IC> DefineSerial<'a> for Custom<S, MA, I, IF, IC>
where
    I: DefineSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, I::Serial>;
}

impl<'a, S, MA, I, IF, IC> DefineUnindexedSerial<'a> for Custom<S, MA, I, IF, IC>
where
    I: DefineUnindexedSerial<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, I::UnindexedSerial>;
}

impl<S, MA, I, IF, IC> ParallelCollectorBase for Custom<S, MA, I::IntoParCollector, IF, IC>
where
    MA: Fn(&S, usize) -> usize,
    I: IntoParallelCollectorBase,
    IF: FnMut(&mut S) -> I,
    IC: FnMut(&mut S, I::Output),
{
    type Output = S;

    #[inline]
    fn finish(mut self) -> Self::Output {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();
        self.state
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        (self.max_afford)(&self.state, request)
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();

        let indexed = self
            .indexed
            .insert((self.indexed_f)(&mut self.state).into_par_collector());
        let (consumer, commit) = indexed.take_parts(len);

        unique::uniquify((consumer, |output| {
            commit(output);
            if (self.max_afford)(&self.state, 1) > 0 {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        }))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();

        let indexed = self
            .indexed
            .insert((self.indexed_f)(&mut self.state).into_par_collector());

        unique::take_uniquify(indexed.take_parts(len))
    }
}

impl<S, MA, I, IF, IC> UnindexedParallelCollectorBase for Custom<S, MA, I::IntoParCollector, IF, IC>
where
    MA: Fn(&S, usize) -> usize,
    I: IntoUnindexedParallelCollectorBase,
    IF: FnMut(&mut S) -> I,
    IC: FnMut(&mut S, I::Output),
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();

        let indexed = self
            .indexed
            .insert((self.indexed_f)(&mut self.state).into_par_collector());
        let (consumer, commit) = indexed.take_parts_unindexed();

        unique_unindexed::uniquify((consumer, |output| {
            commit(output);
            if (self.max_afford)(&self.state, 1) > 0 {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        }))
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();

        let indexed = self
            .indexed
            .insert((self.indexed_f)(&mut self.state).into_par_collector());

        unique_unindexed::take_uniquify(indexed.take_parts_unindexed())
    }
}

impl<'a, S, MA, I, IF, IC, U, UF, UC> DefineSerial<'a> for CustomAlsoUnindexed<S, MA, I, IF, IC, U, UF, UC>
where
    I: DefineSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, I::Serial>;
}

impl<'a, S, MA, I, IF, IC, U, UF, UC> DefineUnindexedSerial<'a>
    for CustomAlsoUnindexed<S, MA, I, IF, IC, U, UF, UC>
where
    U: DefineUnindexedSerial<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, U::UnindexedSerial>;
}

impl<S, MA, I, IF, IC, U, UF, UC> ParallelCollectorBase
    for CustomAlsoUnindexed<S, MA, I::IntoParCollector, IF, IC, U::IntoParCollector, UF, UC>
where
    MA: Fn(&S, usize) -> usize,
    I: IntoParallelCollectorBase,
    IF: FnMut(&mut S) -> I,
    IC: FnMut(&mut S, I::Output),
    U: IntoUnindexedParallelCollectorBase,
    UF: FnMut(&mut S) -> U,
    UC: FnMut(&mut S, U::Output),
{
    type Output = S;

    fn finish(mut self) -> Self::Output {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();
        self.state
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        (self.max_afford)(&self.state, request)
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();

        let indexed = self
            .indexed
            .insert((self.indexed_f)(&mut self.state).into_par_collector());
        let (consumer, commit) = indexed.take_parts(len);

        unique::uniquify((consumer, |output| {
            commit(output);
            if (self.max_afford)(&self.state, 1) > 0 {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        }))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();

        let indexed = self
            .indexed
            .insert((self.indexed_f)(&mut self.state).into_par_collector());

        unique::take_uniquify(indexed.take_parts(len))
    }
}

impl<S, MA, I, IF, IC, U, UF, UC> UnindexedParallelCollectorBase
    for CustomAlsoUnindexed<S, MA, I::IntoParCollector, IF, IC, U::IntoParCollector, UF, UC>
where
    MA: Fn(&S, usize) -> usize,
    I: IntoParallelCollectorBase,
    IF: FnMut(&mut S) -> I,
    IC: FnMut(&mut S, I::Output),
    U: IntoUnindexedParallelCollectorBase,
    UF: FnMut(&mut S) -> U,
    UC: FnMut(&mut S, U::Output),
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();

        let unindexed = self
            .unindexed
            .insert((self.unindexed_f)(&mut self.state).into_par_collector());
        let (consumer, commit) = unindexed.take_parts_unindexed();

        unique_unindexed::uniquify((consumer, |output| {
            commit(output);
            if (self.max_afford)(&self.state, 1) > 0 {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        }))
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        // Don't forget to clean up the previous `parts()` call!
        self.clean_up();

        let unindexed = self
            .unindexed
            .insert((self.unindexed_f)(&mut self.state).into_par_collector());

        unique_unindexed::take_uniquify(unindexed.take_parts_unindexed())
    }
}
