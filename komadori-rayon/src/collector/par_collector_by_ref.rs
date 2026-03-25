use super::{IntoParallelCollectorBase, ParallelCollectorBase};

/// A type that can be converted into a parallel collector by shared reference.
///
/// This trait's main purpose is to provide a convenience method to creates
/// a parallel collector from `&T`.
///
/// You cannot implement this trait directly.
/// Instead, you should implement [`IntoParallelCollectorBase`] for `&T`
/// (where `T` is your type)
/// and this trait is automatically implemented for `T`.
///
/// This trait is not intended for use in bounds.
/// Use [`IntoParallelCollectorBase`] and similar traits in trait bounds instead.
#[allow(private_bounds)]
pub trait ParallelCollectorByRef: Sealed {
    /// Which parallel collector being produced?
    type ParCollector<'a>: ParallelCollectorBase
    where
        Self: 'a;

    /// Creates a parallel collector from a shared reference of a value.
    fn par_collector(&self) -> Self::ParCollector<'_>;
}

trait Sealed {}

impl<T> ParallelCollectorByRef for T
where
    T: ?Sized,
    for<'a> &'a T: IntoParallelCollectorBase,
{
    type ParCollector<'a>
        = <&'a T as IntoParallelCollectorBase>::IntoParCollector
    where
        Self: 'a;

    #[inline]
    fn par_collector(&self) -> Self::ParCollector<'_> {
        self.into_par_collector()
    }
}

impl<T> Sealed for T
where
    T: ?Sized,
    for<'a> &'a T: IntoParallelCollectorBase,
{
}
