# AGENTS: starship-jj Quick Guide
- Release interface: version in Cargo.toml `[package]`, `## Unreleased` in
  CHANGELOG.md, and annotated tags `vX.Y.Z` (leave legacy unprefixed upstream
  tags alone). The routine contract is exactly:

    nix run .#release -- --version X.Y.Z --check
    nix run .#release -- --version X.Y.Z

  The non-mutating check fails fast on a dirty, stale, or diverged checkout,
  invalid or downgrade versions, and local or GitHub tag conflicts. Release
  prepares and validates the versioned tree with fmt, clippy, test, and the
  release contract (ci-fmt keeps the custom nixfmt behavior), then atomically
  publishes `main` and its annotated tag with a lease. The read-only GitHub
  workflow builds two platforms. Follow `docs/release.md` to verify its six
  files, manually publish the four archive/sidecar assets, and dispatch Pages.
  No app or workflow automatically creates a GitHub Release or refreshes Pages.
  Lower-level `prepare-release` and `release-tag` helpers are recovery-only.
  `.builds/` and `builds/release-linux-x86_64.yml` are archival SourceHut
  material; do not submit them or dual-publish a new release.
  `nix run .#ci-fmt`, `ci-clippy`, and `ci-test` are also in `.jj-lint.toml`.
- Build: `cargo build --locked` (or `nix develop -c cargo build`)
- Run: `cargo run -- starship prompt` to print the prompt for the current repo
- Tests: unit `just test`; integration `just itest` (needs `jj`); all `just test-all`
- Single test: `cargo test <filter>` or exact: `cargo test -- --exact <name>`
- Format: `cargo fmt --all --check`; fix with `cargo fmt --all`
- Clippy: `cargo clippy --all-targets -- -D warnings`
- Supply-chain: `cargo deny check`; Audit: `cargo audit`
- Upstream helpers: `just upstream`; rebase: `just rebase-upstream`
- Upstream changes: `just upstream-changes` (deps/locks diffstat + full code diff)
- LLM review prompt: `just llm-review-upstream` (prints agent prompt + upstream diffs)
- Supply-chain review: `just llm-review-supply-chain` (prints supply-chain prompt + manifest/lock diffs)
- Upstream reviewed through `74c94705bad6f0f8019db6a4eb093611818dd94a` on 2026-10-09. All three commits after `8ca6a957` are accounted for in `docs/upstream-review.md`; future reviews start after the new cutoff.
- Ported from upstream after `0.6.0`: `jj`/`jj-cli` 0.39 API updates, flake overlay `prev.stdenv.hostPlatform.system`, RustSec ignore cleanup, snake_case/hex colors, text attrs, commit/change id rendering, `show_previous_if_empty`, metrics `hide_if_empty`, bookmark `ignore_empty_commits`.
- Upstream gotcha: hex color code had `!h.len() == 7`; this fork uses `hex.len() != 7` and tests wrong-length hex values.
- Dev helpers: `just fmt`, `just fmt-check`, `just clippy`, `just test`
- JSON schema: `cargo run --features json-schema -- starship prompt` (prints to stdout; redirect to `schema.json` if needed)
- Imports: order `std`, external crates, `jj_*`/others, then `crate::`/`super::`; avoid `*` imports
- Formatting: use rustfmt defaults; keep small focused modules like current layout
- Types: prefer explicit types on public items; borrow over clone; use `Option`/`Result` patterns
- Naming: snake_case funcs/vars/modules; UpperCamelCase types/enums; SCREAMING_SNAKE_CASE consts
- Errors: return `Result<T, jj_cli::command_error::CommandError>`; use `user_error`/`CommandError::with_message` with proper `CommandErrorKind`
- Avoid: `.unwrap()`/`.expect()` in non-tests; avoid `println!`; write via `Ui` or passed `Write`
- Features: keep `#[cfg(feature = "json-schema")]` isolated; no schema types in default builds
- CLI: extend via `clap` subcommands mirroring `StarshipCommands`; document env vars
- Dependencies: don’t add `anyhow`/`thiserror`/`tracing` unless agreed; stick to current stack
- Contrib: use `jj` (not git); conventional commits; run fmt, clippy, deny, audit before PR; no Cursor/Copilot rules present

<!-- Last audited: 2026-08-05 | updated dependencies and recorded unavailable upstream -->
