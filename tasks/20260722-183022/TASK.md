# Pre-land/pre-commit cargo fmt --check guard (the real fix behind 20260721-163942)

- STATUS: OPEN
- PRIORITY: 22
- TAGS: v0.8.0,tooling,ci

## Goal

Make rustfmt drift *impossible to land on master*, not merely reported after
the fact. This is the real fix behind task 20260721-163942, which was filed on
the false premise that CI has no fmt gate.

CI has run `cargo fmt --check` since 2026-07-09 (ci.yaml "Formatting" step,
commit f1720672), yet drift still reached master (four files healed in
8c7be318 on 2026-07-21). The reason the existing CI step does not stop it:

- This project lands via local `sprout land` (squash-merge) + a **direct push
  to master**, not via PRs. CI on push-to-master is **advisory** - a red run
  does not block or revert the commit that already landed.
- There is **no local guard**: no git hooks (`core.hooksPath` unset), and
  neither `/work`'s verify step nor `sprout land`'s preflight runs
  `cargo fmt --check`.

So the "tool > prose" fix is a **pre-land / pre-commit `cargo fmt --check`
guard** that fails before the drift is committed. LESSONS.md independently
flags this: `lint-gate-is-the-last-step` (x3) under Pending promotions
recommends exactly "a pre-commit / pre-land `cargo fmt --check` guard".

## Steps

- [ ] Decide the guard's seam(s). Candidates, not mutually exclusive:
      (a) a tracked git pre-commit hook + a `core.hooksPath` installer the
          repo README/AGENTS documents;
      (b) fold `cargo fmt --check` into `/work`'s verify step so no branch
          reaches land unformatted;
      (c) a `sprout land` preflight check (owned by the sprout tool - may be
          out of this repo's scope; note if so).
      Prefer a guard that cannot be silently skipped and needs no per-clone
      ritual, or document the one-line setup if it does.
- [ ] Implement the chosen guard using the same nightly toolchain the CI fmt
      step and the fmt ritual use (rust-toolchain.toml pins `nightly`).
- [ ] Prove it: deliberately introduce a formatting diff and show the guard
      refuses the commit/land; then show a clean tree passes.
- [ ] Update LESSONS.md `lint-gate-is-the-last-step` (resolve or annotate the
      pending promotion once the guard lands).

## Definition of Done

- A local guard refuses to commit/land unformatted Rust
  (test: introduce a fmt diff, run the guard, observe non-zero exit).
- A clean tree passes the guard (cmd: `cargo fmt --check`).
- The setup/usage is documented wherever the repo documents its dev ritual
  (manual: a fresh clone can enable the guard from the documented steps).
