use super::{IntoParallelCollectorBase, ParallelCollectorBase};

///
#[allow(private_bounds)]
pub trait ParallelCollectorByRef: Sealed {
    ///
    type ParCollector<'a>: ParallelCollectorBase
    where
        Self: 'a;

    ///
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
