# Pre-land/pre-commit cargo fmt --check guard (the real fix behind 20260721-163942)

- STATUS: CLOSED
- PRIORITY: 22
- TAGS: v0.8.0, tooling, ci

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

- [x] Decide the guard's seam(s). Chose (a) a tracked `.githooks/pre-commit` +
      `scripts/setup-hooks.sh` (core.hooksPath installer). Seams (b) and (c)
      collapse into it: `/work` is a global skill outside this repo, and
      `sprout` is an external nix-store binary - but a repo hook covers BOTH
      paths, because worktrees share this repo's core.hooksPath (gating every
      `/work` commit) and `sprout land` commits with a plain `git commit` that
      fires the hook and rolls back on its failure by design (sprout land
      source, lines 266-274). No new per-repo tool needed.
- [x] Implement the chosen guard using the nightly toolchain. Hook runs
      whole-workspace `cargo fmt --check` (parity with the CI Formatting step),
      only when staged changes touch `.rs`, with a `nix develop` fallback for
      NixOS where bare cargo is off PATH. Also pinned in CI
      (`.github/workflows/ci.yaml` "Fmt hook self-test").
- [x] Prove it: `scripts/test-fmt-hook.sh` drives the real hook in a throwaway
      git repo + crate - misformatted commit REFUSED, clean commit ACCEPTED,
      docs-only commit NOT gated. Passes (exit 0).
- [x] Update LESSONS.md `lint-gate-is-the-last-step` - annotated SHIPPED with
      this task id; the tool guard the pending promotion asked for now exists.

## Definition of Done

- A local guard refuses to commit/land unformatted Rust
  (test: introduce a fmt diff, run the guard, observe non-zero exit).
- A clean tree passes the guard (cmd: `cargo fmt --check`).
- The setup/usage is documented wherever the repo documents its dev ritual
  (manual: a fresh clone can enable the guard from the documented steps).
