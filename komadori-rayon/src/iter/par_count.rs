use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector_base,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

/// A parallel collector that counts how many items it collected.
///
/// This collector corresponds to [`Iterator::count()`].
///
/// # Overflow Behavior
///
/// This collector does no guarding against overflows, so feeding it
/// more than [`usize::MAX`] items either produces the wrong result or panics.
/// If overflow checks are enabled, a panic is guaranteed.
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParCount};
///
/// let count = (1..=10000)
///     .into_par_iter()
///     .feed_into(ParCount::new());
///
/// assert_eq!(count, 10000);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ParCount {
    count: usize,
}

impl ParCount {
    /// Creates a new instance of this parallel collector with an initial count of 0.
    #[inline]
    pub const fn new() -> Self {
        assert_unindexed_par_collector_base(Self { count: 0 })
    }
}

impl<'this> DefineConsumer<'this> for ParCount {
    type Consumer = __count_internal::Consumer;
}

impl ParallelCollectorBase for ParCount {
    type Output = usize;

    #[inline]
    fn finish(self) -> Self::Output {
        self.count
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> std::ops::ControlFlow<()>,
    ) {
        let (consumer, commit) = self.parts_unindexed();
        (len, consumer, commit)
    }
}

impl<'this> DefineUnindexedConsumer<'this> for ParCount {
    type UnindexedConsumer = __count_internal::Consumer;
}

impl UnindexedParallelCollectorBase for ParCount {
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        (__count_internal::Consumer::new(), |count| {
            self.count += count;
            ControlFlow::Continue(())
        })
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
mod __count_internal {
    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumerBase};

    pub struct Consumer(());

    pub struct Combiner(());

    impl Consumer {
        #[inline]
        pub(super) fn new() -> Self {
            Self(())
        }
    }

    impl IntoCollectorBase for Consumer {
        type Output = usize;

        type IntoCollector = komadori::iter::Count;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Self::IntoCollector::new()
        }
    }

    impl plumbing::ConsumerBase for Consumer {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl plumbing::UnindexedConsumerBase for Consumer {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self::new()
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner(())
        }
    }

    impl plumbing::Combiner<usize> for Combiner {
        #[inline]
        fn combine(self, left: &mut usize, right: usize) {
            *left += right;
        }
    }
}
