//!

#[cfg(feature = "rayon")]
mod par_iter_ext;

#[cfg(feature = "rayon")]
pub use par_iter_ext::*;
