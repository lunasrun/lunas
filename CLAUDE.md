# CLAUDE.md

## Project

Lunas — a complete web front-end framework (Vue-parity feature set), built as
a Rust compiler (`crates/`) + a tiny JS runtime (`packages/lunas`).

- **Roadmap:** `roadmap.yml` at the repo root is the single source of truth.
  Statuses: `done` / `in_progress` / `todo` / `deferred`.
- **Design contract:** `crates/lunas_compiler/docs/output-design.md` defines the
  compiled-output shape and the runtime API. Codegen and runtime changes must
  stay consistent with it (update the doc in the same PR when the contract
  evolves).
- Cargo workspace manifest is `crates/Cargo.toml` (NOT the repo root). Rust
  toolchain pinned via `rust-toolchain.toml`. `Cargo.lock` is committed on
  purpose — don't bump serde/swc casually.

## Development workflow (autonomous)

Claude drives development from the roadmap autonomously:

1. **Pick tasks from `roadmap.yml`** (`todo` → work on it; respect dependency
   order: codegen core → dom/components features → app layer → tooling).
2. **Parallelize with subagents.** Run independent features as parallel
   background agents with `isolation: "worktree"`. Choose the agent model by
   difficulty — `opus` for hard compiler/runtime work, cheaper models for
   mechanical tasks — to spread rate-limit load.
3. **One PR per feature — never push to `main` directly.** Every change
   (features, docs, roadmap edits, config) lands via a PR. Each feature
   branch: `feat/<area>-<name>` (or `fix/`, `chore/`). Agents commit, push,
   and open a PR with `gh pr create`. The orchestrator merges PRs (squash)
   once checks pass — merging is pre-authorized, no need to ask. Merge
   sequentially; rebase follow-ups on fresh `main`.
   - Branch prefix convention (also drives automatic PR labeling via
     `add-labels.yml`): `feat/` features, `fix/` bug fixes, `refactor/`
     refactors, `chore/` chores, `docs/` documentation, `version/` release
     version bumps. Those same labels categorize PRs into the auto-generated
     beta release PR (`pr-release-beta.yml`, `git-pr-release` from `main`
     into `beta`), so picking the right prefix keeps the release notes
     accurate.
4. **Quality gate before any PR:** `cargo fmt --check`, `cargo clippy
   --workspace -D warnings`, `cargo test --workspace` (run inside `crates/`),
   plus the runtime test driver (`node packages/lunas/test/run-all.mjs`) for
   runtime changes. Never merge a red PR.
5. **`roadmap.yml` updates ride inside the feature PR.** A PR that completes
   roadmap items flips those item statuses to `done` in the same PR (touch
   only your own item lines; leave `updated:` alone to avoid same-line
   conflicts — it gets bumped in periodic housekeeping PRs). The public viz
   reads `roadmap.yml` from `main` at page load.

## Conventions

- Never-panic guarantee: public compiler entry points return diagnostics, they
  don't panic. Add fuzz/robustness tests for new public APIs.
- Runtime JS is dependency-free ES2015+, `.mjs`, tested with `node --test`.
- Commit style: `feat(codegen): …`, `fix(runtime): …`, `chore(roadmap): …`.
