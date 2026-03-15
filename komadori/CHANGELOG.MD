# Changelog

## 0.6.0

### Added

- `crate::iter::TryForEach`.

### Changed

- API surface of `crate::iter::TryFold`.
- API surface of `crate::iter::Reduce`.
- `CollectorBase::funnel()` is demoted to the `unstable` feature.
- Stop behavior of `CollectorBase::unbatching()`.

### Fixed

- Added missing `Clone` implementations.

### Removed

- Many deprecated items.
- `crate::iter::Any::get()`.
- `crate::iter::All::get()`.
- `crate::iter::Count::get()`.

## 0.5.2

### Added

- `CollectorBase::skip_while()`.
- `crate::iter::ForEach`.
- `crate::iter::Position`.
- `crate::iter::FindMap`.
- `crate::cmp::IsSorted`.
- `crate::cmp::IsSortedBy`.
- `crate::cmp::IsSortedByKey`.

## 0.5.1

### Added

- `crate::cmp::MinMaxBy`.
- `crate::cmp::MinMaxByKey`.
- `CollectorBase::enumerate()`.
- `CollectorBase::filter_map()`.
- `CollectorBase::map_while()`.

## 0.5.0

### Added

- `itertools` feature.
- `IntoCollector` in `crate::prelude`.
- `CollectorBase::by_ref()`.
- `CollectorBase::inspect()`.
- `CollectorBase::update()`.
- `CollectorBase::partition_map()`.
- `crate::cmp::AllEqual`.
- `crate::cmp::MinMax`.
- Unstable: `CollectorBase::alt_break_hint()`.

### Changed

- The crate's name to `komadori`.

### Fixed

- `CollectorBase::skip()` not accounting for the underlying collector while still skipping.

## 0.4.1

### Added

- `CollectorBase::flatten()` and `CollectorBase::flat_map()`.
- Unstable: `Collector::nest(_exact)()` (reintroduced).

## 0.4.0

### Added

- `CollectorBase`.
- `IntoCollectorBase`.
- `crate::mem::Dropping`.
- `crate::mem::Forgetting`.
- Generalized concatenation collectors.
- Different versions of `Tee`.
- `clb_once!`, `clb_mut!` and `clb!`.

### Changed

- `Collector` now uses generic `T` rather than assosciated type `Item` for the item type.
- Major module restructure:
  - Collector-related traits and their adapters are now moved to `collector` module.
  - `BetterCollect` trait is moved to `iter` module and renamed to `IteratorExt`.
  - `All`, `Any`, `Count`, `Find`, `Last`, `Reduce` and `(Try)Fold`
    are moved to `iter` module.
  - `Sum` and `Product` are moved to `ops` module.
- Renamed `komadori` in `IteratorExt` (previously `BetterCollect`)
  to `feed_into` and it now takes `self` instead of `&mut self`.
- Unstable: Renamed `komadori_with_puller` in `IteratorExt` (previously `BetterCollect`)
  to `feed_into_with_puller` and it now takes `self` instead of `&mut self`.
- `Collector::skip()`: now stops accumulating
  if the underlying collector has stopped accumulating during skipping,
  regardless of whether the adaptor has skipped enough items or not.
- `Combine` to `Tee`.

### Fixed

- `Skip::collect()` not returning `ControlFlow` correctly.
- Missing `Debug`, `Clone` and `Default` implementations.
- Incorrect returning `ControlFlow` in many collector implementations.
- `collect_then_finish()` of `All` and `Any` not returning the correct results.
- `?Sized` bounds.

### Removed

- `RefCollector` and its implementations.
- `ConcatStr(ing)`.
- `T` generic in `MapOutput`.
- Unstable: `Collector::nest(_exact)()` and the aggregate API (for now). They will be re-added later.

### Deprecated

- `Sink`.
- `Fuse::finished()`.

## 0.3.2

### Added

- Unstable: `Collector::nest(_exact)()`.

### Fixed

- `Take::collect_many()` not returning `ControlFlow` correctly.

## 0.3.1

### Added

- `Collector::break_hint()`.
- `Collector::map_output()`.

### Fixed

- All items under the `unstable` flag not shown on `doc.rs`.

## 0.3.0

### Added

- Unstable: Aggregate API.
- `prelude` module.
- `CollectorByRef` and `CollectorByMut`.
- `(Max:Min)::by(_key)()`.

### Changed

- `()`, `Vec`, `String` and collections in the standard library now implements
  `IntoCollector` instead of `(Ref)Collector`.
- `IntoCollector` for `&(Sync)Sender`.
- `BetterCollect`'s methods now receive an `IntoCollector` instead.
- `(Ref)Collector` are now dyn compatible.
- `Copied` is renamed to `Copying`.
- `Cloned` is renamed to `Cloning`.
- `Then` is renamed to `Combine`.

### Fixed

- Feature flags.

### Deprecated

- `(Max|Min)By(Key)::new()`.
- `Copied`.
- `Cloned`.
- `Then`.

## 0.2.2

### Fixed

- Unintended recursion in `Take::collect_many()`.
- `Unzip::collect(_ref)()` should not return `Break` early on one returning `Break` to be consistent
  with `Unzip::collect_many()` and `Unzip::collect_then_finish()`.

## 0.2.1

### Added

- `Collector::skip()`.
- `(Ref)Collector` implementations for `&mut C`, where `C: (Ref)Collector`.
- `IntoCollector`.
- `Collector`s for `std::sync::mpsc`.
- `unstable` feature flag.

## 0.2.0

### Changed

- `[___]Ref` collectors have their constructors moved to their non-`Ref` verions.

### Fixed

- `Chain::collect_ref()` returning `Break` when the first collector stops.

## 0.1.2

### Added

- `Collector::unbatching()`.
- `Product`.
- Implementations for `Sum<Wrapping<Int>>` and `Product<Wrapping<Int>>`.
- `Fold(Ref)`.

### Changed

- Optimized `Take::collect_many()`.

### Fixed

- Added doc for `Funnel` (struct).

## 0.1.1

### Added

- `ConcatStr(ing)::with_buf()` that initializes `ConcatStr(ing)` with an initial string.
- `Sink`.
- `Collector::take_while()`.
- Doc for `RefCollector::funnel()`.

## 0.1.0

### Added

- This crate.
