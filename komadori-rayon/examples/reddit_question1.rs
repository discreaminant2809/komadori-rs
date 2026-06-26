//! See: [https://www.reddit.com/r/rust/comments/kp6dxc/rayon_collect_into_multiple_arrays]

use komadori_rayon::prelude::*;
use rayon::prelude::*;

fn main() {
    let nums = [1, 4, -2, 23];
    let ((group_a, group_b), group_c) = nums.into_par_iter().feed_into(
        vec![]
            .into_par_collector()
            .filter(condition_a)
            .tee(vec![].into_par_collector().filter(condition_b))
            .tee(vec![].into_par_collector().filter(condition_c)),
    );

    println!("Group A: {group_a:?}");
    println!("Group B: {group_b:?}");
    println!("Group C: {group_c:?}");
}

fn condition_a(&x: &i32) -> bool {
    x >= 0
}

fn condition_b(&x: &i32) -> bool {
    x % 2 == 0
}

fn condition_c(&x: &i32) -> bool {
    69 % x == 0
}
