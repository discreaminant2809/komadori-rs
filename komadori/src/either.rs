//! Re-export of [`Either`] type.
//!
//! This module is so that you can refer to [`Either`] without importing [`either`] crate.
//! However, to use all the functionalities assosciated with that type,
//! use that crate instead.
//!
//! The crate also provides collector implementations for [`Either`].

mod collector_impl;

pub use either::Either;
