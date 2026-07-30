mod chain;
mod cloning;
mod copying;
mod enumerate;
mod filter;
mod filter_map;
mod flat_map;
mod flatten;
#[cfg(feature = "unstable")]
mod funnel;
mod fuse;
mod inspect;
mod map;
mod map_output;
mod map_while;
#[cfg(feature = "unstable")]
mod nest_family;
mod partition;
mod skip;
mod skip_while;
mod take;
mod take_while;
mod tee;
mod tee_base;
mod tee_clone;
mod tee_funnel;
mod tee_mut;
// #[cfg(feature = "unstable")]
// mod tee_with;
#[cfg(feature = "unstable")]
mod then;
mod trying_options;
mod trying_results;
mod unbatching;
mod unzip;
#[cfg(feature = "itertools")]
mod update;

pub use chain::*;
pub use cloning::*;
pub use copying::*;
pub use enumerate::*;
pub use filter::*;
pub use filter_map::*;
pub use flat_map::*;
pub use flatten::*;
#[cfg(feature = "unstable")]
pub use funnel::*;
pub use fuse::*;
pub use inspect::*;
pub use map::*;
pub use map_output::*;
pub use map_while::*;
#[cfg(feature = "unstable")]
pub use nest_family::*;
pub use partition::*;
pub use skip::*;
pub use skip_while::*;
pub use take::*;
pub use take_while::*;
pub use tee::*;
pub use tee_clone::*;
pub use tee_funnel::*;
pub use tee_mut::*;
// #[cfg(feature = "unstable")]
// pub use tee_with::*;
#[cfg(feature = "unstable")]
pub use then::*;
pub use trying_options::*;
pub use trying_results::*;
pub use unbatching::*;
pub use unzip::*;
#[cfg(feature = "itertools")]
pub use update::*;

use tee_base::*;

#[cfg(all(test, feature = "std"))]
use crate::test_utils::CollectorModel;

#[cfg(all(test, feature = "std"))]
pub(crate) fn take_collector_model<T>(
    n: usize,
) -> CollectorModel<usize, impl FnMut(&mut usize, T), impl FnMut(&usize, usize) -> usize> {
    CollectorModel {
        state: n,
        advance_f: |n: &mut usize, _| *n = n.saturating_sub(1),
        max_afford_f: |&n: &usize, request| n.min(request),
    }
}

#[cfg(all(test, feature = "std"))]
fn take_collector_model_filtered<T>(
    n: usize,
    mut pred: impl FnMut(T) -> bool,
) -> CollectorModel<usize, impl FnMut(&mut usize, T), impl FnMut(&usize, usize) -> usize> {
    CollectorModel {
        state: n,
        advance_f: move |n: &mut usize, item| {
            if pred(item) {
                *n = n.saturating_sub(1);
            }
        },
        max_afford_f: |&n: &_, request| if n == 0 { 0 } else { request },
    }
}
