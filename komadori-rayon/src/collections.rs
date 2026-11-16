//! Parallel collectors for collections in the standard library.
//!
//! This module corresponds to [`std::collections`].

mod collection;
// pub mod btree_set;
pub(crate) mod linked;
pub mod linked_list;

pub(crate) use collection::*;
