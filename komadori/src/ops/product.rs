use crate::collector::CollectorBase;

/// Types that provide a collector to calculate the product of all collected items,
/// multiplying to a provided initial value.
///
/// This trait should not be used in bound, since you would block
/// custom product without using this trait (e.g. using adapters).
/// Accepting a bland parallel collector as a parameter instead.
///
/// See its implementors for examples.
///
/// This trait corresponds to [`std::iter::Product`].
pub trait IntoProduct {
    /// Which product collector being produced?
    type IntoProduct: CollectorBase;

    /// Creates a new instance of a product collector from an initial value.
    fn into_product(self) -> Self::IntoProduct;
}
