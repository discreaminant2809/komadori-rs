#[cfg(feature = "unstable")]
mod alt_break_hint;
mod chain;
mod cloning;
mod copying;
mod enumerate;
mod filter;
mod filter_map;
mod flat_map;
mod flatten;
mod funnel;
mod fuse;
mod inspect;
mod map;
mod map_output;
mod map_while;
#[cfg(feature = "unstable")]
mod nest_family;
mod partition;
#[cfg(feature = "itertools")]
mod partition_map;
mod skip;
mod skip_while;
mod take;
mod take_while;
mod tee;
mod tee_clone;
mod tee_funnel;
mod tee_mut;
#[cfg(feature = "unstable")]
mod tee_with;
mod unbatching;
mod unzip;
#[cfg(feature = "itertools")]
mod update;

#[cfg(feature = "unstable")]
pub use alt_break_hint::*;
pub use chain::*;
pub use cloning::*;
pub use copying::*;
pub use enumerate::*;
pub use filter::*;
pub use filter_map::*;
pub use flat_map::*;
pub use flatten::*;
pub use funnel::*;
pub use fuse::*;
pub use inspect::*;
pub use map::*;
pub use map_output::*;
pub use map_while::*;
#[cfg(feature = "unstable")]
pub use nest_family::*;
pub use partition::*;
#[cfg(feature = "itertools")]
pub use partition_map::*;
pub use skip::*;
pub use skip_while::*;
pub use take::*;
pub use take_while::*;
pub use tee::*;
pub use tee_clone::*;
pub use tee_funnel::*;
pub use tee_mut::*;
#[cfg(feature = "unstable")]
pub use tee_with::*;
pub use unbatching::*;
pub use unzip::*;
#[cfg(feature = "itertools")]
pub use update::*;
