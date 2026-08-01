# Review: KISS: small crates and root binary

- TASK: 20260731-170448
- BRANCH: refactor/kiss-small-crates-root

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [x] R1.1 (MINOR) crates/nova_core/src/lib.rs:269 - the retained NOTE points at
  `nova_gameplay's hud/nova_os.rs`, which no longer exists; the symbol lives at
  `crates/nova_gameplay/src/hud/nova_os/shell.rs:452` after an earlier epic
  child split the module. The pass rewrote this exact comment and carried the
  dead path forward. Change the parenthetical to name
  `hud/nova_os/shell.rs` instead of `hud/nova_os.rs`.
  - Response: fixed - the parenthetical now names `hud/nova_os/shell.rs` and
    the block was re-wrapped at word boundaries. `cargo fmt --check` clean,
    `cargo check -p nova_core` green. Comment-only, so the inertness result
    above still holds. Verified by the round's reviewer.

### Verified

- DoD 1: `nix develop --command cargo check --workspace --all-targets` exit 0.
  Only warnings are the four pre-existing `ambiguous import visibility` ones
  from `nova_gameplay` (tracked as 20260801-005057) and the
  `proc-macro-error2` future-incompat note. Neither originates in scope.
- DoD 2: `nix develop --command cargo fmt --check` exit 0, no output.
- DoD 3: the HUID grep over the six paths returns ZERO hits, so there is no
  exception list to justify. NOTES.md records this. The one surviving
  task-record pointer in scope is `crates/nova_core/Cargo.toml:25`, a `#`
  comment outside that grep and a live WebGPU/WebGL2 guard, not provenance -
  correctly left alone.
- DoD 4: largest file in scope is `crates/nova_modding/src/lib.rs` at 439
  lines, total 1957. Far under 1500; no exception needed. The NOTES.md
  inventory (base max 446, total 2036) reconciles with the post-pass numbers.
- DoD 5: `cargo test -p nova_core -p nova_events -p nova_info -p nova_modding
  -p nova_mod_format --lib` exit 0, 12 passed / 0 failed; `cargo test -p
  nova_core --test cubemap_meta_app_config` 3 passed / 0 failed.
- No-behavior-change, re-derived independently rather than taken from the
  close-out: stripping comment-only lines, trailing `//` tails and blanks from
  each of the eight touched `.rs` files makes base and branch identical in all
  eight. `--numstat` confirms `src/main.rs` is a pure two-line comment
  deletion. No statement, literal, signature, import, `mod` line or visibility
  keyword moved, so no `Plugin::build` body, loader or algorithm can have
  changed.
- Honesty: every number in the close-out reproduced here. The 446-vs-439
  discrepancy between the Steps text and the Evidence text is base-vs-branch,
  not a contradiction.
- Docs sweep: the deleted `nova_info` "workspace `missing_docs` exemplar"
  paragraph is genuinely stale and strands no cross-reference - `AGENTS.md:108`
  and `LESSONS.md:102` carry the rule generically and never name `nova_info`.
  The `nova_events` module-doc fix (adding the omitted `OnNeutralizedEvent`) is
  a correction, matching the rubric's "improve if wrong". Spot-checked the
  surviving cross-references: `nova_scenario::apply_pending_skybox_swaps`
  resolves (`crates/nova_scenario/src/actions/view.rs:246`), the root
  `index.html` named by the canvas NOTE exists, and
  `nova_assets::mod_cache::register_mods_source` is the call directly under its
  NOTE. Only R1.1 failed this check.
- Structure axis: agreed it is a no-op. Every file in scope holds one concern,
  and the task header's "largest file: lib.rs at 622 lines" is simply wrong -
  no file in scope is close. Splitting `nova_core/src/lib.rs`'s three private
  plugin-config helpers would have been a one-caller boundary the epic rubric
  and the global KISS rule both forbid. Not splitting was the correct call.
- Judgment calls on retained vs deleted comments were spot-checked against the
  rubric's "burden is on keeping". The rewrites preserve meaning: the
  `setup_status_ui` NOTE still states the load-bearing fact (the bar is
  deliberately NOT `HudNovaOsExempt`), the `assets_plugin` rustdoc still
  carries the "do not narrow it" guard and the reason a fixed `Paths` set
  cannot cover `mods://`, and the `deps.rs` NOTEs keep the three non-obvious
  expected values. No guarding comment was lost.

- Process signal: two independent sprouts implemented this one task in
  parallel - `refactor/kiss-small-crates` (16:07) and this branch (16:24) -
  from separate cuts of `master`, with no shared history. Duplicated effort;
  the older branch is abandoned by the owner's choice and should be deleted
  with its worktree.
- Process signal: the planned size figure in the task header was off by ~40%
  (622 claimed, 446 actual), which would have mis-scoped this task as a split
  had the implementer trusted it. Worth checking the inventory against the plan
  before choosing an approach on the remaining epic children.

### Pending user checks

- DoD 6 (`manual:`) - owner skims the diff and agrees no behavior changed. The
  comment-stripped identity result above is the mechanical half of that claim;
  the read-through is the owner's.
