# Contributing

## Build and test

You need Rust with the `wasm32-unknown-unknown` target, `wasm-tools`, Python 3 and the SolidCraft CLI
(`solidcraft-cli`, set `SOLIDCRAFT_CLI` if it is not on your PATH).

    python build.py            # unit tests, the package in dist/, then every end-to-end test
    python build.py corner     # the same, with only the end-to-end tests whose name has "corner"
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

All four must pass. CI runs them on the SolidCraft version named in `.github/workflows/build.yml`.

## Pull requests

- One branch and one pull request per feature or fix, into `develop`. No stacked PR chains; a PR is
  merged on its own once it is done and green.
- Every fix and every function has a test that fails without it.
- Update `docs/usage.md` in the same PR (and `README.md` when a function is added).
- The description says what changes in plain words, what you ran to check it, and has a
  screenshot for anything visible in a dialog or the model (or a line `No picture: <reason>`).
- Link the issue with `Closes #N`, or write a line `No issue: <reason>`.

The PR checks enforce the last two and the base branch. `main` is what was last released: at a release the maintainer merges `develop` into `main` (a pull request) and tags it, so the help links and the Workshop show only released work.
