//!

#![forbid(missing_docs)]
#![deny(missing_debug_implementations)]
#![cfg_attr(test, deny(deprecated))]
#![cfg_attr(docsrs, feature(doc_cfg))]
// To make doc examples in sync (prevent accidental deprecated items usage in doc).
#![doc(test(attr(deny(deprecated))))]

pub mod collections;
pub mod collector;
pub mod iter;
pub mod prelude;
pub mod slice;
// pub mod unit;
pub mod vec;
