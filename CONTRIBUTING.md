# How to contribute

## Code of conduct

* This project follows the rules layed out in the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct)

## Coding conventions

* Commits in this Repository follow [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) so please format your Commit messages accordingly.
* Please follow the rust code conventions and use rustfmt to ensure compliance.
* Code will only be merged when it passes the ci pipelines.

## Testing locally (pre-PR)

* Unit tests: `just test` (fast; runs crate unit tests only)
* Integration tests: `just itest` (requires `jj` in PATH; snapshots in `tests/snapshots/`)
* Full suite: `just test-all`
* Snapshot changes:
  * Install helper: `cargo install cargo-insta`
  * Review/accept: `cargo insta review` (then commit updated `.snap` files)
* Notes:
  * Integration tests initialize repos via `jj git init` and skip if `jj` is not installed.
  * Prompt snapshots are stabilized with `SJJ__MODULE_SEPARATOR="|"` and `SJJ__RESET_COLOR=false`.
