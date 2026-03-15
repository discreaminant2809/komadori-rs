use crate::collector::CollectorBase;

/// Types that provide a [`Collector`] to calculate the product of all collected items.
///
/// Implementors should provide an "identity value" as the [`Output`];
/// in other words, the value produced by `T::muling().finish()`.
/// For example, the identity value for a collector that multiplies integers is `1`.
///
/// Typically, the [`Output`] type is the same as the implementing type.
///
/// See its implementors for examples.
///
/// This trait corresponds to [`std::iter::Product`].
///
/// [`Collector`]: crate::collector::Collector
/// [`Output`]: CollectorBase::Output
pub trait Muling {
    /// The result type of the product.
    type Output;

    /// The collector that calculates the product of all collected items.
    type Muling: CollectorBase<Output = Self::Output>;

    /// Creates a new instance of [`Muling`](Muling::Muling) collector.
    fn muling() -> Self::Muling;
}
