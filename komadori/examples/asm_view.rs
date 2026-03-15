use komadori::{
    cmp::Max,
    iter::{Find, Fold},
    prelude::*,
};

fn main() {}

#[unsafe(no_mangle)]
fn bc_tee_with_max(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter()
        .copied()
        .feed_into(i32::adding().tee(Max::new()))
}

#[unsafe(no_mangle)]
fn for_loop_wo_initial(nums: &[i32]) -> (i32, Option<i32>) {
    let mut sum = 0;
    let mut max = None;

    for &num in nums {
        sum += num;
        max = max.max(Some(num));
    }

    (sum, max)
}

#[unsafe(no_mangle)]
fn for_loop_w_initial(nums: &[i32]) -> (i32, i32) {
    let mut sum = 0;
    let mut max = i32::MIN;

    for &num in nums {
        sum += num;
        max = max.max(num);
    }

    (sum, max)
}

#[unsafe(no_mangle)]
fn fold_w_initial(nums: &[i32]) -> (i32, i32) {
    nums.iter()
        .copied()
        .fold((0, i32::MIN), |(sum, max), num| (sum + num, max.max(num)))
}

#[unsafe(no_mangle)]
unsafe fn fold_counter(nums: &[usize]) -> [usize; 1000] {
    nums.iter().fold([0; 1000], |mut counts, &num| {
        unsafe { *counts.get_unchecked_mut(num) += 1 };
        counts
    })
}

#[unsafe(no_mangle)]
unsafe fn bc_counter(nums: &[usize]) -> [usize; 1000] {
    nums.iter().feed_into(Fold::new([0; 1000], |counts, &num| {
        unsafe { *counts.get_unchecked_mut(num) += 1 };
    }))
}

#[unsafe(no_mangle)]
unsafe fn for_loop_counter(nums: &[usize]) -> [usize; 1000] {
    let mut counts = [0; 1000];

    for &num in nums {
        unsafe { *counts.get_unchecked_mut(num) += 1 };
    }

    counts
}

#[unsafe(no_mangle)]
fn iter_find_0(nums: &[i32]) -> Option<i32> {
    nums.iter().find(|&&num| num == 0).copied()
}

// Use manual `collect` because `tee_*` uses this method anyway.
// Not to mention `Find`'s `collect_then_finish` forwards to `find()`.
#[unsafe(no_mangle)]
fn bc_find_0(nums: &[i32]) -> Option<i32> {
    let mut collector = Find::new(|&num| num == 0);
    let mut nums = nums.iter();

    while let Some(&num) = nums.next()
        && collector.collect(num).is_continue()
    {}

    collector.finish()
}

#[unsafe(no_mangle)]
fn bc_sum_find(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter()
        .copied()
        .feed_into(i32::adding().tee(Find::new(|&num| num % 2 != 0)))
}

#[unsafe(no_mangle)]
fn fold_find(nums: &[i32]) -> (i32, Option<i32>) {
    nums.iter().fold((0, None), |(sum, first_odd), &num| {
        (
            sum + num,
            first_odd.or_else(|| (num % 2 != 0).then_some(num)),
        )
    })
}
