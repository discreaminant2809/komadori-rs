use super::{CollectorBase, IntoCollectorBase};

/// A type that can be converted into a collector by mutable reference.
///
/// This trait's main purpose is to provide a convenience method to creates
/// a collector from `&mut T`.
///
/// You cannot implement this trait directly.
/// Instead, you should implement [`IntoCollectorBase`] for `&mut T`
/// (where `T` is your type)
/// and this trait is automatically implemented for `T`.
///
/// This trait is not intended for use in bounds.
/// Use [`IntoCollector`] or [`IntoCollectorBase`] in trait bounds instead.
///
/// [`IntoCollector`]: super::IntoCollector
#[allow(private_bounds)]
pub trait CollectorByMut: Sealed {
    /// Which collector being produced?
    type CollectorMut<'a>: CollectorBase
    where
        Self: 'a;

    /// Creates a collector from a mutable reference of a value.
    fn collector_mut(&mut self) -> Self::CollectorMut<'_>;
}

impl<T> CollectorByMut for T
where
    T: ?Sized,
    for<'a> &'a mut T: IntoCollectorBase,
{
    type CollectorMut<'a>
        = <&'a mut T as IntoCollectorBase>::IntoCollector
    where
        T: 'a;

    #[inline]
    fn collector_mut(&mut self) -> Self::CollectorMut<'_> {
        self.into_collector()
    }
}

trait Sealed {}

impl<T> Sealed for T
where
    T: ?Sized,
    for<'a> &'a mut T: IntoCollectorBase,
{
}
