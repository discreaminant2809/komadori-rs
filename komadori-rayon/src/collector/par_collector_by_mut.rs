use super::{IntoParallelCollectorBase, ParallelCollectorBase};

///
#[allow(private_bounds)]
pub trait ParallelCollectorByMut: Sealed {
    ///
    type ParCollectorMut<'a>: ParallelCollectorBase
    where
        Self: 'a;

    ///
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
