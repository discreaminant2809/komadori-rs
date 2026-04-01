use crate::collector::ParallelCollectorBase;

/// Types that provide a parallel collector to calculate the sum of all collected items,
/// adding to a provided initial value.
///
/// This trait should not be used in bound, since you would block
/// custom parallel sum without using this trait (e.g. using adapters).
/// Accepting a bland parallel collector as a parameter instead.
///
/// See its implementors for examples.
///
/// This trait corresponds to [`std::iter::Sum`].
pub trait IntoParSum {
    /// The parallel collector that calculates the sum of all collected items.
    type IntoParSum: ParallelCollectorBase;

    /// Creates a new instance of a parallel sum collector from an initial value.
    fn into_par_sum(self) -> Self::IntoParSum;
}

// ///
// pub trait ParSumMut: Sealed {
//     ///
//     type ParSumMut<'a>: ParallelCollectorBase
//     where
//         Self: 'a;

//     ///
//     fn par_sum_mut(&mut self) -> Self::ParSumMut<'_>;
// }

// trait Sealed {}

// impl<S> Sealed for S
// where
//     S: ?Sized,
//     for<'a> &'a mut S: IntoParSum,
// {
// }

// impl<S> ParSumMut for S
// where
//     S: ?Sized,
//     for<'a> &'a mut S: IntoParSum,
// {
//     type ParSumMut<'a>
//         = <&'a mut S as IntoParSum>::IntoParSum
//     where
//         Self: 'a;

//     #[inline]
//     fn par_sum_mut(&mut self) -> Self::ParSumMut<'_> {
//         self.into_par_sum()
//     }
// }
