//! Parallel collectors for slice manipulation.
//!
//! This module is empty for now. More will be added alter.
//!
//! This module corresponds to [`std::slice`].

#[cfg(test)]
pub(crate) mod drainer;
pub(crate) mod in_place_write;
#[cfg(test)]
pub(crate) mod par_iter;
