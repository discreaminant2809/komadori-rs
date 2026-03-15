//! Module containing items for aggregation.

mod aggregate_op;
mod group;
mod group_map;
mod imp;
mod ref_aggregate_op;

pub use aggregate_op::*;
pub use group::*;
pub use group_map::*;
pub use imp::*;
pub use ref_aggregate_op::*;

#[macro_export]
// Somehow the doc.rs does not render the feature flag.
#[cfg_attr(docsrs, doc(cfg(feature = "unstable")))]
/// Combines multiple aggregate ops into a single "`struct`-based" aggregate.
///
/// The `struct` can be constructed normally (even with a base struct),
/// except every field is "initialized"
/// with an [`AggregateOp`] whose [`Value`] matches that field's type.
///
/// Among the specified fields to be aggregated, every field except the last one
/// must implement [`RefAggregateOp`].
///
/// # Limitations
///
/// This macro currently supports structs with 1 to 12 aggregated fields
/// (i.e., fields explicitly specified with an aggregate op, not those
/// filled via a base struct such as `..Default::default()`).
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use komadori::{
///     prelude::*, aggregate_struct,
///     aggregate::{self, AggregateOp, GroupMap},
/// };
///
/// #[derive(Debug, Default, PartialEq)]
/// struct Stats {
///     sum: i32,
///     max: i32,
///     version: u32,
/// }
///
/// let groups = [(1, 1), (1, 4), (2, 1), (1, 2), (2, 3)]
///     .into_iter()
///     .feed_into(
///         HashMap::new()
///             .into_aggregate(aggregate_struct!(Stats {
///                 sum: aggregate::Sum::new().cloning(),
///                 max: aggregate::Max::new(),
///                 ..Default::default()
///             }))
///     );
///
/// let expected_groups = HashMap::from_iter([
///     (1, Stats { sum: 7, max: 4, version: 0 }),
///     (2, Stats { sum: 4, max: 3, version: 0 }),
/// ]);
/// assert_eq!(groups, expected_groups);
/// ```
///
/// [`Value`]: AggregateOp::Value
macro_rules! aggregate_struct {
    (
        $ty_name:path {
            $($fields:ident: $aggregate_ops:expr,)+
            $(..$base_struct:expr)?
        }
    ) => {
        $crate::aggregate::Combine::new(
            ($($aggregate_ops,)+),
            |_, ($($fields,)+)| { $ty_name { $($fields,)+ $(..$base_struct)? } },
            |&mut $ty_name { $(ref mut $fields,)+ .. }| ($($fields,)+),
        )
    };

    (
        $ty_name:path {
            $($fields:ident: $aggregate_ops:expr),+
        }
    ) => {
        $crate::aggregate_struct!(
            $ty_name {
                $($fields: $aggregate_ops,)+
            }
        )
    };
}

#[inline(always)]
const fn assert_op<Op: AggregateOp>(op: Op) -> Op {
    op
}

#[inline(always)]
const fn assert_ref_op<Op: RefAggregateOp>(op: Op) -> Op {
    op
}

// To fix the macro.
#[cfg(feature = "std")]
fn _example() {
    #[allow(unused_imports)]
    use crate::{
        aggregate::{self, AggregateOp, GroupMap},
        aggregate_struct,
        prelude::*,
    };
    use std::collections::HashMap;
    #[derive(Debug, Default, PartialEq)]
    struct Stats {
        sum: i32,
        max: i32,
        version: u32,
    }
    let groups = [(1, 1), (1, 4), (2, 1), (1, 2), (2, 3)]
        .into_iter()
        .feed_into(HashMap::new().into_aggregate(aggregate_struct!(Stats {
            sum: aggregate::Sum::new().cloning(),
            max: aggregate::Max::new(),
            ..Default::default()
        })));
    let expected_groups = HashMap::from_iter([
        (
            1,
            Stats {
                sum: 7,
                max: 4,
                version: 0,
            },
        ),
        (
            2,
            Stats {
                sum: 4,
                max: 3,
                version: 0,
            },
        ),
    ]);
    assert_eq!(groups, expected_groups);
}
