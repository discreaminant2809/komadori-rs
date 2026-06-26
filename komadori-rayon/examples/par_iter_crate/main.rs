//! In `komadori-rayon` directory, run this with
//! `cargo r --example par_iter_crate --no-default-features`!

pub mod par_iter_integration;

use komadori_rayon::prelude::*;
use par_iter::prelude::*;

use par_iter_integration::ParIterParallelIteratorExt;

fn main() {
    let nums = (1..=5).into_par_iter().feed_into(vec![]);
    assert_eq!(nums, [1, 2, 3, 4, 5]);

    let nums = (1..=5_u8).into_par_iter().feed_into_indexed(vec![]);
    assert_eq!(nums, [1, 2, 3, 4, 5]);

    let evens = (1..=10)
        .into_par_iter()
        .filter(|&num| num % 2 == 0)
        .feed_into(vec![]);
    assert_eq!(evens, [2, 4, 6, 8, 10]);

    let evens = (1..=10)
        .into_par_iter()
        .feed_into(vec![].into_par_collector().filter(|&num| num % 2 == 0));
    assert_eq!(evens, [2, 4, 6, 8, 10]);
}
