use std::{iter::FusedIterator, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, Fuse as CollectorFuse};

/// An [`Iterator`] that "drives" the underlying iterator to feed the underlying collector.
///
/// This `struct` is a part of [`Iterator::feed_into_with_puller()`].
/// See its documentation for more.
///
/// [`Iterator::feed_into_with_puller()`]: crate::iter::IteratorExt::feed_into_with_puller
#[derive(Debug)]
pub struct Driver<'a, I, C> {
    // We need to fuse it since we call `for_each(drop)` later.
    iter: I,
    collector: CollectorFuse<&'a mut C>,
}

impl<'a, I, C> Driver<'a, I, C>
where
    I: Iterator,
    C: for<'i> Collector<&'i mut I::Item>,
{
    pub(in crate::iter) fn new(iter: I, collector: &'a mut C) -> Self {
        Self {
            iter,
            collector: <&mut C>::fuse(collector),
        }
    }
}

impl<I, C> Driver<'_, I, C>
where
    I: Iterator,
    C: for<'i> Collector<&'i mut I::Item>,
{
    // Helper for fold-related methods.
    fn fold_then_forward_once<B>(
        mut self,
        init: B,
        mut forwardable_fold: impl ForwardableFold<B, I>,
    ) -> B {
        match self.iter.try_fold(init, {
            let forwardable_fold = &mut forwardable_fold;
            move |accum, mut item| {
                let cf = self.collector.collect(&mut item);
                let accum = forwardable_fold.fold(accum, item);
                if cf.is_continue() {
                    ControlFlow::Continue(accum)
                } else {
                    ControlFlow::Break(accum)
                }
            }
        }) {
            ControlFlow::Continue(accum) => accum,
            ControlFlow::Break(accum) => forwardable_fold.forward(accum, self.iter),
        }
    }

    // fn fold_then_forward_mut<B, FF>(&mut self, init: B, mut forwardable_fold: FF) -> FF::Ret
    // where
    //     FF: ForwardableTryFold<B, I>,
    // {
    //     enum WhichBreak<T, U> {
    //         Collector(T),
    //         TryFold(U),
    //     }

    //     match self.iter.try_fold(init, {
    //         let forwardable_fold = &mut forwardable_fold;
    //         let collector = &mut self.collector;
    //         move |accum, mut item| match (
    //             collector.collect_ref(&mut item),
    //             forwardable_fold.try_fold(accum, item),
    //         ) {
    //             (ControlFlow::Continue(_), ControlFlow::Continue(accum)) => {
    //                 ControlFlow::Continue(accum)
    //             }
    //             (ControlFlow::Break(_), ControlFlow::Continue(accum)) => {
    //                 ControlFlow::Break(WhichBreak::Collector(accum))
    //             }
    //             (_, ControlFlow::Break(ret)) => ControlFlow::Break(WhichBreak::TryFold(ret)),
    //         }
    //     }) {
    //         ControlFlow::Continue(accum) | ControlFlow::Break(WhichBreak::TryFold(accum)) => accum,
    //         ControlFlow::Break(WhichBreak::Collector(accum)) => {
    //             forwardable_fold.forward(accum, &mut self.iter)
    //         }
    //     }
    // }
}

impl<I, C> Iterator for Driver<'_, I, C>
where
    I: Iterator,
    C: for<'i> Collector<&'i mut I::Item>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let mut item = self.iter.next()?;
        let _ = self.collector.collect(&mut item);
        Some(item)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        // Overriden since the iterator may just use `ExactSizeIterator::len()`
        // for their `count()` implementation.
        struct ForwardableCount;

        impl<I: Iterator> ForwardableFold<usize, I> for ForwardableCount {
            #[inline]
            fn fold(&mut self, accum: usize, _item: <I as Iterator>::Item) -> usize {
                accum + 1
            }

            #[inline]
            fn forward(self, accum: usize, items: I) -> usize
            where
                Self: Sized,
            {
                accum + items.count()
            }
        }

        self.fold_then_forward_once(0, ForwardableCount)
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        // Overriden since the iterator may just use `DoubleEndedIterator::next_back()`
        // for their `last()` implementation.
        struct ForwardableLast;

        impl<T, I: Iterator<Item = T>> ForwardableFold<Option<T>, I> for ForwardableLast {
            #[inline]
            fn fold(&mut self, _accum: Option<T>, item: <I as Iterator>::Item) -> Option<T> {
                Some(item)
            }

            #[inline]
            fn forward(self, accum: Option<T>, items: I) -> Option<T>
            where
                Self: Sized,
            {
                items.last().or(accum)
            }
        }

        self.fold_then_forward_once(None, ForwardableLast)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let mut item = self.iter.next()?;
        for n in (0..n).rev() {
            if self.collector.collect(&mut item).is_break() {
                return self.iter.nth(n);
            }

            item = self.iter.next()?;
        }

        Some(item)
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.fold_then_forward_once(init, f)
    }

    // fn all<F>(&mut self, f: F) -> bool
    // where
    //     Self: Sized,
    //     F: FnMut(Self::Item) -> bool,
    // {
    //     struct ForwardableAll<F>(F);

    //     impl<F, I: Iterator> ForwardableTryFold<(), I> for ForwardableAll<F>
    //     where
    //         F: FnMut(I::Item) -> bool,
    //     {
    //         type Ret = ();

    //         fn try_fold(
    //             &mut self,
    //             _accum: (),
    //             item: <I as Iterator>::Item,
    //         ) -> ControlFlow<Self::Ret, ()> {
    //             if (self.0)(item) {
    //                 ControlFlow::Continue(())
    //             } else {
    //                 ControlFlow::Break(())
    //             }
    //         }
    //     }

    //     self.fold_then_forward_mut((), ForwardableAll(f))
    //         .is_continue()
    // }

    // fn any<F>(&mut self, f: F) -> bool
    // where
    //     Self: Sized,
    //     F: FnMut(Self::Item) -> bool,
    // {
    //     struct ForwardableAll<F>(F);

    //     impl<F, I: Iterator> ForwardableTryFold<(), I> for ForwardableAll<F>
    //     where
    //         F: FnMut(I::Item) -> bool,
    //     {
    //         type Ret = bool;

    //         fn try_fold(
    //             &mut self,
    //             _accum: (),
    //             item: <I as Iterator>::Item,
    //         ) -> ControlFlow<Self::Ret, ()> {
    //             if (self.0)(item) {
    //                 ControlFlow::Continue(())
    //             } else {
    //                 ControlFlow::Break(())
    //             }
    //         }
    //     }

    //     self.fold_then_forward_mut((), ForwardableAll(f))
    //         .is_continue()
    // }
}

impl<I, C> ExactSizeIterator for Driver<'_, I, C>
where
    I: ExactSizeIterator,
    C: for<'i> Collector<&'i mut I::Item>,
{
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<I, C> FusedIterator for Driver<'_, I, C>
where
    I: FusedIterator,
    C: for<'i> Collector<&'i mut I::Item>,
{
}

// Helper for fold-related methods.
trait ForwardableFold<A, I: Iterator> {
    fn fold(&mut self, accum: A, item: I::Item) -> A;

    // Can be overriden if there's a more efficient implementation in [`Iterator`]
    #[inline]
    fn forward(self, accum: A, items: I) -> A
    where
        Self: Sized,
    {
        let mut this = self;
        items.fold(accum, move |accum, item| this.fold(accum, item))
    }
}

impl<A, I, F> ForwardableFold<A, I> for F
where
    I: Iterator,
    F: FnMut(A, I::Item) -> A,
{
    #[inline]
    fn fold(&mut self, accum: A, item: <I as Iterator>::Item) -> A {
        self(accum, item)
    }
}

// trait ForwardableTryFold<A, I: Iterator> {
//     type Ret;

//     fn try_fold(&mut self, accum: A, item: I::Item) -> ControlFlow<Self::Ret, A>;

//     #[inline]
//     fn forward(self, accum: A, items: &mut I) -> Self::Ret
//     where
//         Self: Sized;
// }
