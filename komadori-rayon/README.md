# komadori-rayon 0.1.2

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
find its maximum value and collect to a [`Vec`], in parallel.
What would be our approach?

- Approach 1: Two-pass

```rust
use rayon::prelude::*;

let nums = [1, 3, 2];
let max = nums.into_par_iter().max();
let v: Vec<_> = nums.into_par_iter().collect();

assert_eq!(max, Some(3));
assert_eq!(v, [1, 3, 2]);
```

**Cons:** This performs two passes over the data, which is worse than one-pass in performance.
Also, we submit more tasks to the thread pool, making it busier, hence blocking more other tasks,
hence even worse performance in practice.

- Approach 2: `fold().reduce()`

```rust
use rayon::prelude::*;

fn id() -> (i32, Vec<i32>) {
    (i32::MIN, vec![])
}

let (max, v) = [1, 3, 2]
    .into_par_iter()
    .fold(id, |(max, mut v), num| {
        v.push(num);
        (max.max(num), v)
    })
    .reduce(id, |(max1, mut v1), (max2, mut v2)| {
        v1.append(&mut v2);
        (max1.max(max2), v1)
    });

assert_eq!(max, 3);
assert_eq!(v, [1, 3, 2]);
```

**Cons:** This is incredibly verbose and performs worse due to concatenation
instead of mutating the [`Vec`] in-place (like the first approach does).
You can improve the performance a bit by using a linked list of [`Vec`],
but then it is still worse than in-place mutation.

- Approach 3: `inspect()` and atomic

```rust
use rayon::prelude::*;
use std::sync::atomic::{AtomicI32, Ordering};

let max = AtomicI32::new(0);
let v: Vec<_> = [1, 3, 2]
    .into_par_iter()
    .inspect(|&num| {
        max.fetch_max(num, Ordering::Relaxed);
    })
    .collect();

assert_eq!(max.into_inner(), 3);
assert_eq!(v, [1, 3, 2]);
```

**Cons:** This has the worst possible performance, because the collection to [`Vec`]
is cheap so the costs of CAS and cache ping-pong dominate.
By "the worst possible performance," I mean... hundreds times slower than serial execution!
This is fine when the pipeline is expensive (e.g. processing each image).

This crate proposes a one-pass, declarative approach:

```rust
use rayon::prelude::*;
use komadori_rayon::{prelude::*, cmp::ParMax};

let (max, v) = [1, 3, 2]
    .into_par_iter()
    .feed_into(ParMax::new().tee(vec![]));

assert_eq!(max, Some(3));
assert_eq!(v, [1, 3, 2]);
```

This approach is both one-pass and declarative, while is also composable.
Moreoever, it still utilizes the indexed path which is to mutate the [`Vec`]
in-place.

See [here][max_vec_bench_mark] for the benchmark of the above and more approaches.

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

  Because the idea is *thread-pool-agnostic* (as long as the parallel approach is fork-join),
  you can turn off this feature and use your own thread pool if you find `rayon` not satisfy
  your use case, such as `chili` or `forte`.
  Be aware that in this case, [`feed_into()`] and similar methods will not be available,
  so you have to drive parallel collectors by yourself, or wait until this crate
  add more integrations with other thread pools.

  See [this example][par-iter-example].

- **`unstable`** — Enables experimental and unstable features.
  Items gated behind this feature do **not** follow normal semver guarantees
  and may change or be removed at any time.

[`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
[`ParallelIterator`]: https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html
[`ParallelCollector`]: https://docs.rs/komadori-rayon/0.1.2/komadori_rayon/collector/trait.ParallelCollector.html
[`feed_into()`]: https://docs.rs/komadori-rayon/0.1.2/komadori_rayon/iter/trait.RayonParallelIteratorExt.html#method.feed_into
[max_vec_bench_mark]: https://github.com/discreaminant2809/komadori-rs/blob/main/komadori-rayon/benches/max_vec.rs
[par-iter-example]: https://github.com/discreaminant2809/komadori-rs/blob/main/komadori-rayon/examples/par_iter_crate
