# komadori 0.9.1

[![Crates.io Version](https://img.shields.io/crates/v/komadori.svg)](https://crates.io/crates/komadori)
[![Docs.rs](https://img.shields.io/docsrs/komadori)](https://docs.rs/komadori)
[![GitHub Repo](https://img.shields.io/badge/github-komadori--rs-blue?logo=github)](https://github.com/discreaminant2809/komadori-rs.git)
![MSRV](https://img.shields.io/crates/msrv/komadori)

Multi-reduction library. Provides a composable, declarative way to consume an iterator.

If [`Iterator`] is the "source half" of data pipeline, [`Collector`] is the "sink half" of the pipeline.

In order words, [`Iterator`] describes how to produce data, and [`Collector`] describes how to consume it.

## Motivation

Suppose we are given an array of `i32` and we are asked to calculate sum
and create a [`Vec`] of every num being doubled. What would be our approach?

- Approach 1: Two-pass

```rust
let nums = [1, 3, 2];
let sum: i32 = nums.into_iter().sum();
let doubles = nums.into_iter()
    .map(|num| num * 2)
    .collect::<Vec<_>>();

assert_eq!(sum, 6);
assert_eq!(doubles, [2, 6, 4]);
```

**Cons:** This performs two passes over the data, which may be worse than one-pass
due to increased memory traffic. It is fine for arrays,
but can be much worse for [`HashSet`], [`LinkedList`], or... data from an IO stream.

- Approach 2: `for`-loop (or [`Iterator::fold()`] if you prefer)

```rust
let nums = [1, 3, 2];

let mut sum = 0;
let mut doubles = Vec::with_capacity(nums.len());

for num in nums {
    sum += num;
    doubles.push(num * 2);
}

assert_eq!(sum, 6);
assert_eq!(doubles, [2, 6, 4]);
```

**Cons:** Not very declarative. Even with `fold()`,
the main logic is still kind of procedural because you have to ensure that
the logic is a one-pass.
Moreover, its performance is much worse due to `doubles.push(num * 2)`
being lowered into a scalar loop with repeaated capacity check
(even though we have reserved beforehand!),
inhibiting vectorization.

- Approach 3: [`Iterator::inspect()`]

```rust
let nums = [1, 3, 2];
let mut sum = 0;
let doubles = nums
    .into_iter()
    .inspect(|num| sum += num)
    .map(|num| num * 2)
    .collect::<Vec<_>>();

assert_eq!(sum, 6);
assert_eq!(doubles, [2, 6, 4]);
```

**Cons:** This approach has multiple drawbacks:

- If the requirement changes to "calculate sum and find any negative value,"
  this approach may produce incorrect results.
  The "any" logic may short-circuit on finding the desired value,
  preventing the "sum" logic from summing every value.
  It is possible that we can rearrange so that the "any" logic goes first,
  but if the requirement changes to "find any negative value and even value,"
  we cannot escape.
- The state is kept outside. Now the iterator cannot go anywhere else
  (e.g. returning from a function).
- Very unintuitive and hack-y (hard to reason about).
- Not declarative enough.
- Slower than the upcoming approach.
  (Pro tip: Use `map()` instead of `inspect()` results in way better performance,
  but the above issues still exist)

This crate proposes a one-pass, declarative approach:

```rust
use komadori::{prelude::*, cmp::Max};

let nums = [1, 3, 2];
let (sum, doubles) = nums
    .into_iter()
    .feed_into((
        0.into_sum(),
        vec![].into_collector().map(|num| num * 2),
    ));

assert_eq!(sum, 6);
assert_eq!(doubles, [2, 6, 4]);
```

This approach achieves:

- One-pass.
- Declarative.
- Performance (LLVM lowers it into a very nice vectorized one-pass loop).

See [this benchmark][sum-doubles-benchmark] to see more approaches.
You can run the benchmark by yourself!

This is only with integers. How about with non-`Copy` types?

```rust
// Suppose we open a connection...
fn socket_stream() -> impl Iterator<Item = String> {
    ["the", "noble", "and", "the", "singer"]
        .into_iter()
        .map(String::from)
}

// Task: Returns:
// - An array of data from the stream.
// - How many bytes were read.
// - The last-seen data.

// Usually, we're pretty much stuck with for-loop
// (tradition, `(try_)fold`, `(try_)for_each`).
// No common existing tools can help us here:
let mut byte_read = 0_usize;
let mut received = vec![];
let mut last_seen = None;

for data in socket_stream() {
    byte_read += data.len();
    received.push(data.clone());
    last_seen = Some(data);
}

let expected = (byte_read, received, last_seen);

// This crate's way:
use komadori::{prelude::*, iter::Last, clb_mut};

let (byte_read, received, last_seen) = socket_stream()
    .feed_into((
        0_usize
            .into_sum()
            .map(clb_mut!(|s: &mut String| -> usize { s.len() })),
        vec![].into_collector().cloning(),
        Last::new(),
    ));

assert_eq!((byte_read, received, last_seen), expected);
```

Very declarative! We describe what we want to collect.

You might think this is just like [`Iterator::unzip()`]...

Consider this example:

```rust
use std::collections::HashSet;
use komadori::{prelude::*, clb_mut};

// Suppose we open a connection...
fn socket_stream() -> impl Iterator<Item = String> {
    ["the", "noble", "and", "the", "singer"]
        .into_iter()
        .map(String::from)
}

// Task: Collect UNIQUE chunks of data and concatenate them.

// `Iterator::unzip`
let unzip_way: (String, HashSet<_>) = socket_stream()
    // Sad. We have to clone.
    // We can't take a reference, since the referenced data is returned too.
    .map(|chunk| (chunk.clone(), chunk))
    .unzip();

// Another approach is do two passes (collect to `Vec`, then iterate),
// which is still another allocation,
// or `Iterator::fold`, which's procedural.

// `Collector`
let collector_way = socket_stream()
    // No clone. The data flows smoothly.
    .feed_into((
        String::new()
            .into_concat()
            .map(clb_mut!(|s: &mut String| -> &str { &s[..] })),
        HashSet::new(),
    ));

assert_eq!(unzip_way, collector_way);
```

## Usage in API

If a function looks like `fn foo(State, Iterator<T>) -> Output`
and the iterator is accepted just to be traversed,
consider rewriting it to `fn foo(State) -> Collector<T, Output>`,
since the original unnecessarily owns the traversal, making it hard
to additional add another reduction to traverse alongside with
the source.

Consider this example:

```rust
fn stats<'a>(records: impl IntoIterator<Item = &'a Record>) -> Stats {
    // Implementations
}

fn checksum<'a>(records: impl IntoIterator<Item = &'a Record>) -> u64 {
    // Implementations
}
```

Now how can we obtain both `stats` and `checksum` in one traversal,
especially if records are streamed instead of in an array?

We can rewrite the above:

```rust
use komadori::prelude::*;

fn stats() -> impl for<'a> Collector<&'a Record, Output = Stats> {
    // Implementations
}

fn checksum() -> impl for<'a> Collector<&'a Record, Output = u64> {
    // Implementations
}

fn as_ref(record: &mut Record) -> &Record {
    record
}

// Now we can calculate both in one traversal!
let (stats, checksum) = records.feed_into((
    stats().map(as_ref),
    checksum().map(as_ref).funnel(),
));
```

## Crate stucture

Modules in this crate mirror those in the standard library, because this crate
extends many types there. There is also `collector` which
contains collector functionalities that work behind [`feed_into()`],
and `prelude` which re-exports commons items for easier use.

It is recommended to read the documentation of `collector` next
if you want to delve into how collectors work.

## Features

- **`alloc`** — Enables collectors and implementations for types in the
  [`alloc`] crate (e.g., [`Vec`], [`VecDeque`], [`BTreeSet`]).

- **`std`** *(default)* — Enables the `alloc` feature and implementations
  for [`std`]-only types (e.g., [`HashMap`]).

- **`itertools`** — Enables collectors and adapters that resemble those
  in the `itertools` crate.

- **`unstable`** — Enables experimental and unstable features.
  Items gated behind this feature do **not** follow normal semver guarantees
  and may change or be removed at any time.

[`Collector`]: https://docs.rs/komadori/latest/komadori/collector/trait.Collector.html
[`feed_into()`]: https://docs.rs/komadori/latest/komadori/iter/trait.IteratorExt.html#method.feed_into
[`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
[`Iterator::fold()`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold
[`Iterator::inspect()`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.inspect
[`Iterator::unzip()`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.unzip
[`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
[`HashSet`]: https://doc.rust-lang.org/std/collections/struct.HashSet.html
[`HashMap`]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
[`LinkedList`]: https://doc.rust-lang.org/std/collections/struct.LinkedList.html
[`alloc`]: https://doc.rust-lang.org/alloc/index.html
[`std`]: https://doc.rust-lang.org/std/index.html
[`VecDeque`]: https://doc.rust-lang.org/std/collections/struct.VecDeque.html
[`BTreeSet`]: https://doc.rust-lang.org/std/collections/struct.BTreeSet.html
[sum-doubles-benchmark]: https://github.com/discreaminant2809/komadori-rs/blob/main/komadori/benches/sum_doubles.rs
