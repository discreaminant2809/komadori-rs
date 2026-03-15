use crate::collector::CollectorBase;

/// Types that provide a [`Collector`] to calculate the sum of all collected items.
///
/// Implementors should provide an "identity value" as the [`Output`];
/// in other words, the value produced by `T::adding().finish()`.
/// For example, the identity value for a collector that adds integers is `0`.
///
/// Typically, the [`Output`] type is the same as the implementing type.
///
/// See its implementors for examples.
///
/// This trait corresponds to [`std::iter::Sum`].
///
/// [`Collector`]: crate::collector::Collector
/// [`Output`]: CollectorBase::Output
pub trait Adding {
    /// The result type of the sum.
    type Output;

    /// The collector that calculates the sum of all collected items.
    type Adding: CollectorBase<Output = Self::Output>;

    /// Creates a new instance of [`Adding`](Adding::Adding) collector.
    fn adding() -> Self::Adding;
}
