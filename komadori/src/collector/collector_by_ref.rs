use super::{CollectorBase, IntoCollectorBase};

/// A type that can be converted into a collector by shared reference.
///
/// This trait's main purpose is to provide a convenience method to creates
/// a collector from `&T`.
///
/// You cannot implement this trait directly.
/// Instead, you should implement [`IntoCollectorBase`] for `&T`
/// (where `T` is your type)
/// and this trait is automatically implemented for `T`.
///
/// This trait is not intended for use in bounds.
/// Use [`IntoCollector`] or [`IntoCollectorBase`] in trait bounds instead.
///
/// [`IntoCollector`]: super::IntoCollector
#[allow(private_bounds)]
pub trait CollectorByRef: Sealed {
    /// Which collector being produced?
    type Collector<'a>: CollectorBase
    where
        Self: 'a;

    /// Creates a collector from a shared reference of a value.
    fn collector(&self) -> Self::Collector<'_>;
}

impl<T> CollectorByRef for T
where
    T: ?Sized,
    for<'a> &'a T: IntoCollectorBase,
{
    type Collector<'a>
        = <&'a T as IntoCollectorBase>::IntoCollector
    where
        T: 'a;

    #[inline]
    fn collector(&self) -> Self::Collector<'_> {
        self.into_collector()
    }
}

trait Sealed {}

impl<T> Sealed for T
where
    T: ?Sized,
    for<'a> &'a T: IntoCollectorBase,
{
}
