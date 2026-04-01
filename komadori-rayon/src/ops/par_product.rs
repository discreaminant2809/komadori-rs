use crate::collector::ParallelCollectorBase;

/// Types that provide a parallel collector to calculate the product of all collected items,
/// multiplying to a provided initial value.
///
/// This trait should not be used in bound, since you would block
/// custom parallel product without using this trait (e.g. using adapters).
/// Accepting a bland parallel collector as a parameter instead.
///
/// See its implementors for examples.
///
/// This trait corresponds to [`std::iter::Product`].
pub trait IntoParProduct {
    /// The parallel collector that calculates the product of all collected items.
    type IntoParProduct: ParallelCollectorBase;

    /// Creates a new instance of a parallel product collector from an initial value.
    fn into_par_product(self) -> Self::IntoParProduct;
}
