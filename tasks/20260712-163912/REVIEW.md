# Review: Editor visible + editable section keybinds

- TASK: 20260712-163912
- BRANCH: feature/editor-section-keybinds

## Round 1

- VERDICT: APPROVE

Out-of-context (fresh-context agent) review. Verified: goal delivered (per-section
keybind chips + click-to-rebind); hold-key-while-placing path byte-identical
(only the previously-empty `None` click arm changed); rebind updates BOTH the live
`Spaceship*InputBinding` component AND `PlayerSpaceshipConfig::inputs`, preserves
non-keyboard binds, Escape cancels, and guards a section deleted while armed;
arming only fires for bindable sections; `binding_label` matches the real
bevy_enhanced_input `Binding` enum and is panic-free + prelude-exported;
`EditorRebind` reset on both state entries, labels `DespawnOnExit(Editor)`. The
flagged `Single<camera>` is panic-SAFE: Bevy 0.19 skips a system whose `Single`
matches != 1, the editor camera exists whenever the Editor-gated system runs, and
the other WASD cameras live in non-concurrent states (matches existing editor
`Single` systems). Tests meaningful/falsifiable; no existing test weakened.

Checks: `cargo check --workspace --all-targets` clean; nova_editor 8/8 (3 new);
nova_gameplay binding_label 1/1; `cargo fmt --check` clean.

NITs (cosmetic, no blocker):
- [x] R1.1 (NIT) position_section_keybind_labels runs in Update, not the
  PostUpdate the plan mentioned (one-frame lag, invisible on a static editor
  scene). Addressed: documented the Update choice + rationale in the fn docstring
  so code and comment agree.
- R1.2 (NIT) sync's has_label is O(n*m) - matches sync_ammo_readouts; left as-is.
