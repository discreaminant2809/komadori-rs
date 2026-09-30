# komadori-rayon 0.2.0

[![Crates.io Version](https://img.shields.io/crates/v/komadori-rayon.svg)](https://crates.io/crates/komadori_rayon)
[![Docs.rs](https://img.shields.io/docsrs/komadori-rayon)](https://docs.rs/komadori_rayon)
[![GitHub Repo](https://img.shields.io/badge/github-komadori--rs-blue?logo=github)](https://github.com/discreaminant2809/komadori-rs.git)

Parallel multi-reduction library. Provides composable parallel reductions.

If [`ParallelIterator`] is the "source half" of data pipeline,
[`ParallelCollector`] is the "sink half" of the pipeline.

In order words, [`ParallelIterator`] describes how to produce data in parallel,
and [`ParallelCollector`] describes how to consume it in parallel.

## Motivation

Suppose we are given an array of `i32` and we are asked to
find its sum and create a [`Vec`] of every integer being doubled, in parallel.
What would be our approach?

- Approach 1: Two-pass

```rust
use rayon::prelude::*;

let nums = [1, 3, 2];

let sum: i32 = nums.into_par_iter().sum();
let doubles: Vec<_> = nums
    .into_par_iter()
    .map(|num| num * 2)
    .collect();

assert_eq!(sum, 6);
assert_eq!(doubles, [2, 6, 4]);
```

**Cons:** This performs two passes over the data, which may be worse than one-pass
due to increased memory traffic.
Also, we submit more tasks to the thread pool, making it busier, hence blocking more other tasks,
hence even worse performance in practice.

- Approach 2: `fold().reduce()`

```rust
use rayon::prelude::*;

fn id() -> (i32, Vec<i32>) {
    (0, vec![])
}

let (sum, doubles) = [1, 3, 2]
    .into_par_iter()
    .fold(id, |(sum, mut v), num| {
        v.push(num * 2);
        (sum + num, v)
    })
    .reduce(id, |(sum1, mut v1), (sum2, mut v2)| {
        v1.append(&mut v2);
        (sum1 + sum2, v1)
    });

assert_eq!(sum, 6);
assert_eq!(doubles, [2, 6, 4]);
```

**Cons:** This is incredibly verbose and performs worse due to concatenation
instead of mutating the [`Vec`] in-place (like the first approach does).
You can improve the performance a bit by using a linked list of [`Vec`],
but then it is still worse than in-place mutation.

- Approach 3: `inspect()` and atomic

```rust
use rayon::prelude::*;
use std::sync::atomic::{AtomicI32, Ordering};

let sum = AtomicI32::new(0);
let doubles: Vec<_> = [1, 3, 2]
    .into_par_iter()
    .map(|num| {
        sum.fetch_add(num, Ordering::Relaxed);
        num * 2
    })
    .collect();

assert_eq!(sum.into_inner(), 6);
assert_eq!(doubles, [2, 6, 4]);
```

**Cons:** This has the worst possible performance, because the collection to [`Vec`]
is cheap so the costs of CAS and cache ping-pong dominate.
By "the worst possible performance," I mean... hundreds times slower than serial `for`-loop!
(To be fair, this is fine if the pipeline is expensive, such as image processing)

This crate proposes a one-pass, declarative approach:

```rust
use rayon::prelude::*;
use komadori_rayon::prelude::*;

let (sum, doubles) = [1, 3, 2]
    .into_par_iter()
    .feed_into((
        0.into_par_sum(),
        vec![].into_par_collector().map(|num| num * 2),
    ));

assert_eq!(sum, 6);
assert_eq!(doubles, [2, 6, 4]);
```

This approach is both one-pass and declarative, while is also composable.
Moreoever, it still utilizes the indexed path which is to mutate the [`Vec`]
in-place.

See [here][sum_doubles_bench_mark] for the benchmark of the above and more approaches.

## Usage in API

If a function looks like `fn foo(State, ParallelIterator<T>) -> Output`
and the iterator is accepted just to be traversed,
consider rewriting it to `fn foo(State) -> ParallelCollector<T, Output>`,
since the original unnecessarily owns the traversal, making it hard
to additional add another reduction to traverse alongside with
the source.

Consider this example:

```rust
use rayon::prelude::*;

fn sum_even(nums: impl IntoParallelIterator<Item = i32>) -> i32 {
    nums.into_par_iter().filter(|&num| num % 2 == 0).sum()
}

fn max_abs(nums: impl IntoParallelIterator<Item = i32>) -> Option<i32> {
    nums.into_par_iter().map(i32::abs).max()
}
```

Now how can we obtain both `sum_even` and `max_abs` in one traversal,
especially if numbers are not from an array?

We can rewrite the above:

```rust
use rayon::prelude::*;
use komadori_rayon::{prelude::*, cmp::ParMax};

fn sum_even() -> impl UnindexedParallelCollector<i32, Output = i32> {
    0_i32.into_par_sum().filter(|&num| num % 2 == 0)
}

fn max_abs() -> impl UnindexedParallelCollector<i32, Output = Option<i32>> {
    ParMax::new().map(i32::abs)
}

// Now we can calculate both in one traversal!
let (sum_even, max_abs) = nums.feed_into((
    sum_even().copying(),
    max_abs(),
));
```

## Crate stucture

Modules in this crate mirror those in the standard library, because this crate
extends many types there. There is also `collector` which
contains functionalities of parallel collectors that work behind [`feed_into()`],
and `prelude` which re-exports commons items for easier use.

It is recommended to read the documentation of `collector` next
if you want to delve into how parallel collectors work.

## Features

- **`rayon`** *(default)* — Yes! Even though the crate is basically an integration with `rayon`,
  there is this feature which can be turned off and effectively make the crate
  not an integration with `rayon` anymore.

  Because the idea is *thread-pool-agnostic* (as long as the parallel approach follow the `rayon` model),
  you can turn off this feature and use your own thread pool if you find `rayon` not satisfy
  your use case, such as `chili` or `forte`.
  Be aware that in this case, [`feed_into()`] and similar methods will **not** be available,
  so you have to drive parallel collectors by yourself, or wait until this crate
  add more integrations with other thread pools.

  See [this example][par-iter-example].

- **`unstable`** — Enables experimental and unstable features.
  Items gated behind this feature do **not** follow normal semver guarantees
  and may change or be removed at any time.

[`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
[`ParallelIterator`]: https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html
[`ParallelCollector`]: https://docs.rs/komadori-rayon/0.2.0/komadori_rayon/collector/trait.ParallelCollector.html
[`feed_into()`]: https://docs.rs/komadori-rayon/0.2.0/komadori_rayon/iter/trait.RayonParallelIteratorExt.html#method.feed_into
[sum_doubles_bench_mark]: https://github.com/discreaminant2809/komadori-rs/blob/main/komadori-rayon/benches/sum_doubles.rs
[par-iter-example]: https://github.com/discreaminant2809/komadori-rs/blob/main/komadori-rayon/examples/par_iter_crate
