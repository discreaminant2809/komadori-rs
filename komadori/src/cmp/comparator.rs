use std::cmp::Ordering;

/// Comparator acting as an `FnMut(&T, &T) -> Ordering`
/// for internal implementation.
pub trait Comparator<T> {
    fn cmp(&mut self, a: &T, b: &T) -> Ordering;

    fn lt(&mut self, a: &T, b: &T) -> bool {
        self.cmp(a, b).is_lt()
    }

    fn le(&mut self, a: &T, b: &T) -> bool {
        self.cmp(a, b).is_le()
    }

    fn min_assign(&mut self, min: &mut T, value: T) {
        // See: https://doc.rust-lang.org/beta/src/core/cmp.rs.html#1064-1066
        if self.lt(&value, min) {
            *min = value;
        }
    }

    fn max_assign(&mut self, max: &mut T, value: T) {
        // See: https://doc.rust-lang.org/beta/src/core/cmp.rs.html#1025-1027
        if self.lt(&value, max) {
        } else {
            *max = value;
        }
    }
}

#[derive(Clone)]
pub struct OrdComparator;

impl<T> Comparator<T> for OrdComparator
where
    T: Ord,
{
    #[inline]
    fn cmp(&mut self, a: &T, b: &T) -> Ordering {
        a.cmp(b)
    }

    #[inline]
    fn lt(&mut self, a: &T, b: &T) -> bool {
        a < b
    }

    #[inline]
    fn le(&mut self, a: &T, b: &T) -> bool {
        a <= b
    }
}

impl<F, T> Comparator<T> for F
where
    F: FnMut(&T, &T) -> Ordering,
{
    #[inline]
    fn cmp(&mut self, a: &T, b: &T) -> Ordering {
        self(a, b)
    }
}
