# https://github.com/casey/just

# Fetch from upstream and update local tracking bookmarks
upstream:
	jj git fetch --remote upstream

# Rebase our main stack onto upstream/main, keeping our changes on top
rebase-upstream:
	jj rebase -b main -d main@upstream

# Show changes since upstream/main: summarize deps/locks, full code diff
upstream-changes: upstream
	@echo "== Dependencies and lockfiles (summary) =="
	jj diff --no-pager --from main@upstream --to @ --stat 'root-file:"Cargo.toml"' 'root-file:"Cargo.lock"' 'root-file:"flake.lock"'
	@echo
	@echo "== Code changes (full diff) =="
	jj diff --no-pager --from main@upstream --to @ 'all() ~ (root-file:"Cargo.toml" | root-file:"Cargo.lock" | root-file:"flake.lock")' --git

# LLM-oriented: Print a guided prompt + upstream diffs for review
llm-review-upstream: upstream
	@echo "=== Prompt for LLM Code Review of Upstream Changes ==="
	@echo
	@echo "Instructions:"
	@echo "- Review changes from upstream/main to current working copy (@)."
	@echo "- Summarize dependency and lockfile changes only; keep brief."
	@echo "- Perform a detailed review of code changes (non-lock/config files)."
	@echo "- Focus on correctness, safety (no unwrap/expect), error handling, performance,"
	@echo "  style, docs, tests, and public API/CLI behavior changes."
	@echo "- Call out risky upgrades, breaking changes, and follow-up actions."
	@echo
	@echo "Output format:"
	@echo "- Summary: 3-6 bullets on scope and risk."
	@echo "- Dependencies: concise bullets (not line-by-line)."
	@echo "- Code Review: detailed bullets grouped by file/theme with rationale."
	@echo "- Breaking Changes: explicit section if any."
	@echo "- Tests: note coverage changes and gaps."
	@echo
	@echo "Project hints: Rust; prefer Result/CommandError; avoid println!; keep style consistent."
	@echo
	@echo "=== Dependencies & Lockfiles (summary) ==="
	jj diff --no-pager --from main@upstream --to @ --stat 'root-file:"Cargo.toml"' 'root-file:"Cargo.lock"' 'root-file:"flake.lock"' || true
	@echo
	@echo "=== Code Changes (full diff) ==="
	jj diff --no-pager --from main@upstream --to @ 'all() ~ (root-file:"Cargo.toml" | root-file:"Cargo.lock" | root-file:"flake.lock")' --git || true

# LLM-oriented: Supply-chain focused dependency review for upstream changes
llm-review-supply-chain: upstream
	@echo "=== Prompt for LLM Supply-Chain Review of Dependency Changes ==="
	@echo
	@echo "Scope:"
	@echo "- Analyze dependency changes between upstream/main and current (@)."
	@echo "- Identify potential supply-chain risks and required mitigations."
	@echo
	@echo "What to look for:"
	@echo "- New dependencies, transitive adds, or source changes (e.g., git->crates.io)."
	@echo "- Version bumps with known CVEs, yanked releases, or suspicious jumps."
	@echo "- New maintainers, repo/namespace moves, or unmaintained crates."
	@echo "- Build-script changes (build.rs), proc-macros, unsafe usage flags, features toggled."
	@echo "- License changes or incompatible licenses."
	@echo "- Use of network at build time or optional features enabling network."
	@echo
	@echo "Data to use (if available in output):"
	@echo "- Cargo.toml changes and feature diffs."
	@echo "- Cargo.lock adds/removes/version changes and checksums."
	@echo "- flake.lock source revisions and inputs."
	@echo
	@echo "Output format:"
	@echo "- High-Risk Findings: bullets with crate@version, reason, mitigation."
	@echo "- Medium/Low: concise bullets with rationale."
	@echo "- SBOM/Follow-ups: list commands or steps (e.g., cargo audit, cargo deny)."
	@echo
	@echo "Project hints: Prefer minimal features; avoid network in build; check cargo-deny config."
	@echo
	@echo "=== Dependency Manifests (diffstat) ==="
	jj diff --no-pager --from main@upstream --to @ --stat 'root-file:"Cargo.toml"' 'root-file:"Cargo.lock"' 'root-file:"flake.lock"' || true
	@echo
	@echo "=== Cargo.toml Changes ==="
	jj diff --no-pager --from main@upstream --to @ 'root-file:"Cargo.toml"' || true
	@echo
	@echo "=== Cargo.lock Changes (filtered, names/versions) ==="
	jj diff --no-pager --from main@upstream --to @ 'root-file:"Cargo.lock"' | /usr/bin/rg 'name = |version = |checksum = ' -n --no-heading || true
	@echo
	@echo "=== flake.lock Changes ==="
	jj diff --no-pager --from main@upstream --to @ 'root-file:"flake.lock"' || true
	@echo
	@echo "(Tip) Useful checks to run locally: cargo deny check; cargo audit; nix flake lock --update-input <input> --commit; verify checksums and yanks."

# Dev: formatting, linting, and tests
fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

clippy:
	cargo clippy --all-targets -- -D warnings

clippy-fix:
	cargo clippy --fix --allow-dirty --all-targets -- -D warnings

test:
	cargo test --bin starship-jj

# Integration tests only
itest:
	cargo test --tests

# Full test suite (unit + integration + doc)
test-all:
	cargo test

# Supply-chain: cargo-deny
deny:
	cargo deny check

# Security audit: cargo-audit
audit:
	cargo audit

# Run formatter check and clippy
check: fmt-check clippy
	@:

# Full CI: format check, clippy, tests, deny & audit
ci: fmt-check clippy test-all deny audit
	@echo "ok: ci checks passed"

# run lints that fix the things they find
lint: fmt clippy-fix
