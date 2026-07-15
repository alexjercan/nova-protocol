# Retro: developer docs on-ramp (20260715-224041)

## What shipped

The contributor on-ramp is complete: a "Start here" reading-order callout atop
project-tour.md; an "Update vs FixedUpdate" rule in architecture.md derived from
the two-clocks spike; a "Contributing a change" subsection and an expanded
"Debug tooling" subsection in development.md; and 12 hardcoded `~line N`
citations across the two extension guides converted to grep-for-symbol pointers.

## Decisions

- Kept step 1 to a markdown callout on project-tour.md only (no new landing
  page, no nav-renderer change), which was both the task's stated minimum and
  the clean-merge choice - the whole task stayed markdown-only, so it could not
  collide with the player/creator tasks' registration-file edits.
- Symbol pointers over line numbers everywhere, with a disambiguation note where
  a symbol is not unique (`match &section.kind` appears three times in
  placement.rs; the guide now tells the reader which one). Grep-verified every
  cited symbol still exists rather than trusting the old citation.

## The task text was wrong in three places; the code won

The task described the debug surface loosely and I had the agent follow the code:
- The feature is spelled `debug`; `dev` is an alias (root Cargo.toml). The task
  said "under `--features dev`" - true but imprecise; the doc states both.
- CI runs `cargo test --workspace --features debug` (and clippy `--features
  debug`, plus a windowed `examples_smoke` under Xvfb and a license gate), NOT
  the task's bare `cargo test --workspace`. Confirmed by reading
  .github/workflows/ci.yaml, not guessing.
- `--debugdump` dumps the `Update` schedule specifically (the PostUpdate/
  FixedUpdate dumps are commented out in nova_debug), not a generic full graph.
This is the same class of lesson as the creator task's stale-picker premise:
plan text is a starting point, the code is the authority. `verify-first` on
every mechanism claim caught all three.

## What went well

- The FixedUpdate rule is the payoff of grounding: it rests on the spike's exact
  mechanism (two poses, two clocks, GlobalTransform in FixedUpdate holds frame
  N-1's PostUpdate pose) and its measured failure (7.1 rad/s in 15 frames at 150
  u/s, 0 post-fix), cross-checked against the live `configure_sets` in
  nova_gameplay::plugin and the fixed `thruster_impulse_system`. Reviewing it
  meant re-reading the spike lines and confirming the number was quoted, not
  rounded or invented - it was verbatim.
- Constraining the agent to five markdown files up front made the review a pure
  content check with zero merge risk.

## What to do differently

- The debug-tooling paragraph needed four source reads (main.rs, nova_debug,
  Cargo.toml, ci.yaml) to state a handful of flag/feature facts correctly. A
  short "developer surface" reference table (flags, features, CI steps) kept in
  the repo and linked from the wiki would let the doc cite one authority instead
  of re-deriving from four files each time these drift.
