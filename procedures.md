# Procedures

## Checklist of pubishing a new version of a crate

- Pass all CIs.
- Clear any local dependencies (ones with `path = { ... }`).
- Check `README.md` (both the crate and the workspace).
- `cargo publish --dry-run` and check for abnormality.
- `cargo publish`.
- Push a tag for that crate's version.

## Procedure of implementing a feature

1. Write the change in `CHANGELOG.MD`.
2. Implements it.
3. Push the change.
4. Must pass all CIs.
5. If any failed, fix it and go back to step 3.
