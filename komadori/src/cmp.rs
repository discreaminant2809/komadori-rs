//! [`Collector`]s for comparing items.
//!
//! This module provides collectors that determine the maximum or minimum
//! values among the items they collect, using different comparison strategies.
//! They correspond to [`Iterator`]’s comparison-related methods, such as
//! [`Iterator::max()`], [`Iterator::min_by()`], and [`Iterator::max_by_key()`].
//!
//! This module corresponds to [`std::cmp`].
//!
//! [`Collector`]: crate::collector::Collector

#[cfg(feature = "itertools")]
mod all_equal;
#[cfg(feature = "itertools")]
mod comparator;
mod is_sorted;
mod is_sorted_base;
mod is_sorted_by;
mod is_sorted_by_key;
mod max;
mod max_by;
mod max_by_key;
mod min;
mod min_by;
mod min_by_key;
#[cfg(feature = "itertools")]
mod min_max;
#[cfg(feature = "itertools")]
mod min_max_base;
#[cfg(feature = "itertools")]
mod min_max_by;
#[cfg(feature = "itertools")]
mod min_max_by_key;
mod value_key;

#[cfg(feature = "itertools")]
pub use all_equal::*;
#[cfg(feature = "itertools")]
use comparator::*;
pub use is_sorted::*;
use is_sorted_base::*;
pub use is_sorted_by::*;
pub use is_sorted_by_key::*;
pub use max::*;
pub use max_by::*;
pub use max_by_key::*;
pub use min::*;
pub use min_by::*;
pub use min_by_key::*;
#[cfg(feature = "itertools")]
pub use min_max::*;
#[cfg(feature = "itertools")]
use min_max_base::*;
#[cfg(feature = "itertools")]
pub use min_max_by::*;
#[cfg(feature = "itertools")]
pub use min_max_by_key::*;
use value_key::*;

#[inline]
fn max_assign<T: Ord>(max: &mut T, value: T) {
    // Don't use `>=`. The `max` function does `other < self`.
    // See: https://doc.rust-lang.org/beta/src/core/cmp.rs.html#1025-1027
    if value < *max {
    } else {
        *max = value
    }
}

#[inline]
fn min_assign<T: Ord>(min: &mut T, value: T) {
    // Don't use `>=`. The `min` function does `other < self`.
    // See: https://doc.rust-lang.org/beta/src/core/cmp.rs.html#1064-1066
    if value < *min {
        *min = value
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test_utils {
    use core::cmp::Ordering;

    #[cfg(feature = "itertools")]
    use itertools::MinMaxResult;

    /// A struct that never compares the ID.
    /// This is crucial to test that the correct item is pertained
    /// if there are multiple equal maximal/minimal items.
    #[derive(Debug, Clone, Copy, Eq)]
    pub struct Id<T = i32> {
        pub id: usize,
        pub num: T,
    }

    impl<T> Id<T>
    where
        T: Eq,
    {
        pub fn full_eq(self, other: Self) -> bool {
            self.id == other.id && self.num == other.num
        }

        pub fn full_eq_opt(x: Option<Self>, y: Option<Self>) -> bool {
            Self::full_eq_opt_ref(&x, &y)
        }

        pub fn full_eq_opt_ref(x: &Option<Self>, y: &Option<Self>) -> bool {
            match (x, y) {
                (Some(x), Some(y)) => x.id == y.id && x.num == y.num,
                (None, None) => true,
                _ => false,
            }
        }

        #[cfg(feature = "itertools")]
        pub fn full_eq_minmax_res(x: MinMaxResult<Self>, y: MinMaxResult<Self>) -> bool {
            x == y
        }
    }

    impl<T> PartialEq for Id<T>
    where
        T: PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            self.num == other.num
        }
    }

    impl<T> PartialOrd for Id<T>
    where
        T: PartialOrd,
    {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            self.num.partial_cmp(&other.num)
        }
    }

    impl<T> Ord for Id<T>
    where
        T: Ord,
    {
        fn cmp(&self, other: &Self) -> Ordering {
            self.num.cmp(&other.num)
        }
    }
}
