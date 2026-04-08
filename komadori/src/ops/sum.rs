use crate::collector::CollectorBase;

/// Types that provide a collector to calculate the sum of all collected items,
/// adding to a provided initial value.
///
/// This trait should not be used in bound, since you would block
/// custom sum without using this trait (e.g. using adapters).
/// Accepting a bland parallel collector as a parameter instead.
///
/// See its implementors for examples.
///
/// This trait corresponds to [`std::iter::Sum`].
pub trait IntoSum {
    /// Which sum collector being produced?
    type IntoSum: CollectorBase;

    /// Creates a new instance of a sum collector from an initial value.
    fn into_sum(self) -> Self::IntoSum;
}
