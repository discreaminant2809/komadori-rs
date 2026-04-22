//! Provides composable parallel reductions.

#![forbid(missing_docs)]
#![deny(missing_debug_implementations)]
#![cfg_attr(test, deny(deprecated))]
#![cfg_attr(docsrs, feature(doc_cfg))]
// To make doc examples in sync (prevent accidental deprecated items usage in doc).
#![doc(test(attr(deny(deprecated))))]

pub mod cmp;
pub mod collections;
pub mod collector;
pub mod iter;
pub mod num;
pub mod ops;
pub mod prelude;
pub mod slice;
// pub mod unit;
mod helpers;
#[cfg(test)]
// Will be touched in the future
#[allow(unused)]
mod test_utils;
pub mod vec;
