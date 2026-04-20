# Procedures

## Procedure of pubishing a new version of a crate

- Must pass all CIs.
- Clear any local dependencies (ones with `path = { ... }`).
- `cargo publish --dry-run` and check for abnormality.
- `cargo publish`.

## Procedure of implementing a feature

1. Write the change in `CHANGELOG.MD`.
2. Implements it.
3. Push the change.
4. Must pass all CIs.
5. If any failed, fix it and go back to step 3.
