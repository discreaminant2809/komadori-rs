use super::{IntoParallelCollectorBase, ParallelCollectorBase};

/// A type that can be converted into a parallel collector by mutable reference.
///
/// This trait's main purpose is to provide a convenience method to creates
/// a parallel collector from `&mut T`.
///
/// You cannot implement this trait directly.
/// Instead, you should implement [`IntoParallelCollectorBase`] for `&mut T`
/// (where `T` is your type)
/// and this trait is automatically implemented for `T`.
///
/// This trait is not intended for use in bounds.
/// Use [`IntoParallelCollectorBase`] and similar traits in trait bounds instead.
#[allow(private_bounds)]
pub trait ParallelCollectorByMut: Sealed {
    /// Which parallel collector being produced?
    type ParCollectorMut<'a>: ParallelCollectorBase
    where
        Self: 'a;

    /// Creates a parallel collector from a mutable reference of a value.
    fn par_collector_mut(&mut self) -> Self::ParCollectorMut<'_>;
}

trait Sealed {}

impl<T> ParallelCollectorByMut for T
where
    T: ?Sized,
    for<'a> &'a mut T: IntoParallelCollectorBase,
{
    type ParCollectorMut<'a>
        = <&'a mut T as IntoParallelCollectorBase>::IntoParCollector
    where
        Self: 'a;

    #[inline]
    fn par_collector_mut(&mut self) -> Self::ParCollectorMut<'_> {
        self.into_par_collector()
    }
}

impl<T> Sealed for T
where
    T: ?Sized,
    for<'a> &'a mut T: IntoParallelCollectorBase,
{
}
