# Changelog

## 0.2.0

### Added

- `crate::collector::plumbing::SerialOf`.
- `crate::collector::plumbing::SerialOutputOf`.
- `crate::collector::plumbing::UnindexedSerialOf`.
- `crate::collector::plumbing::UnindexedSerialOutputOf`.

### Changed

- Reworked parallel collector traits.

### Fixed

- `ParallelCollectorBase::tee_mut()` now can accept mutable reference of
  a `?Sized` type.

## 0.1.1

### Fixed

- Missing `?Sized` in blanket implementations.

## 0.1.0

### Added

- This crate.
