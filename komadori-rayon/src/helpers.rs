pub mod unique {
    crate::uniquify_serial!(inner);
    pub use inner::*;
}

pub mod unique_unindexed {
    crate::uniquify_serial!(inner, unindexed = true);
    pub use inner::*;
}
