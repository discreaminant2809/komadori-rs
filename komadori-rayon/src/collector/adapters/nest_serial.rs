use komadori::prelude::*;

pub(crate) trait DefineCollector<'a, Binder = &'a mut Self> {
    type Collector: CollectorBase + Clone;
}

/// Same as consumers, but does not support output reduction.
pub(crate) trait ClonableCollectorAnchor: for<'a> DefineCollector<'a> {
    fn splittable_collector(&mut self) -> <Self as DefineCollector<'_>>::Collector;

    #[inline]
    fn take_splittable_collector(&mut self) -> <Self as DefineCollector<'_>>::Collector {
        self.splittable_collector()
    }
}

pub(crate) trait IntoClonableCollectorAnchor {
    type ClonableCollectorAnchor: ClonableCollectorAnchor;

    fn into_clonable_collector_anchor() -> Self::ClonableCollectorAnchor;
}

/// ```ignore
/// # nest_serial_args! {}
/// ```
#[macro_export]
macro_rules! nest_serial_args {
    (state: $state:expr, collector_f: $collector_f:expr $(,)?) => {};
    (collector_f: $collector_f:expr, state: $state:expr $(,)?) => {};

    (state: $state:expr, fold_f: $fold_f:expr $(,)?) => {};
    (fold_f: $fold_f:expr, state: $state:expr $(,)?) => {};

    (state_f: $state_f:expr, fold_f: $fold_f:expr $(,)?) => {};
    (fold_f: $fold_f:expr, state_f: $state_f:expr $(,)?) => {};
}
