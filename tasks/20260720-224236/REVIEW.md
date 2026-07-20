# Review: relocate nova_meta_gen to tools/

- TASK: 20260720-224236
- BRANCH: refactor/meta-gen-to-tools

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings from a fresh reviewer with no sight of the implementing
session. It verified BOTH key risks by RUNNING them: (1) the `.wav.meta` still
gets written by the tool from its new `tools/` home (feature unification via
nova_modding -> nova_gameplay survived - `3 written, s.wav.meta` present); and
(2) `cargo metadata` cross-check - the ONLY member absent from `default-members`
is `tools/nova_meta_gen`, no game crate silently dropped, and the root package
`.` IS in default-members (bare build still builds the game). Also confirmed the
4 files are git RENAMES (history preserved), `cargo check -p nova_meta_gen`
builds from the new path-dep, the Trunk `-p nova_meta_gen` hook is unchanged, and
no CI/justfile hardcodes the old path.

- [x] R1.1 (NIT) web/src/wiki/dev/project-tour.md:48, architecture.md:29 - both
  wiki crate tables still listed `nova_meta_gen` among game crates without the
  `tools/` note (name-only, no stale `crates/` path; outside the task's explicit
  README+AGENTS+152304 sweep scope).
  - Response: fixed - both rows now read "Binary under `tools/` (web-build
    tooling, not a game crate): ...", consistent with the README/AGENTS rows.
    (keep-docs-in-sync: sweep the NAME tree-wide, not just the scoped surfaces.)

No BLOCKER/MAJOR. The two correctness risks (.wav sidecar, default-members
completeness) were verified by running and pass.
