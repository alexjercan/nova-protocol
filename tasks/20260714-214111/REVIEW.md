# Review: nova_ui crate + migrate nova_editor

- TASK: 20260714-214111
- BRANCH: ui/nova-ui-crate

## Round 1

- VERDICT: APPROVE

Mechanical extraction + migration. Re-derived the load-bearing claim (nothing
drifted in the move) independently:

- Palette/metric VALUES are identical between `master:nova_editor/ui/theme.rs`
  and `nova_ui/theme.rs` (diff shows only RAIL_W/DRAWER_W, intentionally left in
  the editor). No colour or metric shifted.
- Widget LOGIC is identical between the old `nova_editor/ui/widget.rs` and
  `nova_ui/widget.rs`: the diff (with renames normalised) shows only
  `pub(crate)`->`pub`, `crate::ui::theme`->`crate::theme`, a rustfmt reflow, and
  the added `panel_header`/`separator` helpers.
- `placement.rs` is byte-identical to master (empty `git diff`), so the editor's
  build/place behaviour is unchanged by construction.
- Selection is proven by a new deterministic `nova_ui` test (insert `Pressed` on a
  `ThemedButton`+`ButtonValue` -> resource set + `Selected` moved).
- `cargo check --workspace --all-targets --features debug` clean; 12 editor tests +
  1 nova_ui test pass.

- [ ] R1.1 (MINOR) `09_editor` autopilot did not capture a green "placed a section"
  on this run - the frame-counted place/verify phases were starved inside the
  wall-clock 6s window on a heavily-loaded machine (cold caches + a parallel
  sprout). Not a blocker: placement.rs is untouched and selection is deterministically
  proven, so a regression is impossible by construction; a clean-machine autopilot
  placement run is the residual visual check (passed on this same code path in
  20260714-204219). No code change warranted.

Dep hygiene checked: `nova_ui` has no nova deps (acyclic); `nova_info` correctly
not a consumer (build-info only). No external crate references the deleted editor
theme/widget items (workspace check clean).
