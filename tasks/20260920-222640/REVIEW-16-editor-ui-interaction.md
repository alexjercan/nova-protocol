# Review 16: Editor UI and interaction

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor findings

## Findings

### MINOR - `crates/nova_editor/src/ui/inspector.rs:770-771` - Inspector tooltip contains visible accidental whitespace

The `OVERRIDDEN` tooltip string contains six spaces between "later" and "change". Bevy text preserves this spacing, so the Inspector displays a visible gap in player-facing guidance.

### MINOR - `crates/nova_editor/src/ui/window.rs:502` - Colour-picker documentation is duplicated on one line

The sentence "Open the colour picker on the swatch that was clicked." appears twice with no line break between the copies. This is a source-documentation defect only.

## Adjudication

No ECS ordering, input-mode, drag/place/snap, hierarchy-lifetime, reconcile, material/mesh rebuild, or runtime-ID defect survived. The apparent gallery subtree measurement cost is limited to a small visible grid while the gallery is open and uses write-on-difference tolerance; no frame cost was established.

## Coverage

Fully read every editor file not covered by review 15, including configuration, cues, frame, gallery, gizmo, glyph, highlight, keybinds, palette, placement, preview, probe, readout, skin, snap, stage, all UI modules, and direct tests. Together with review 15, every Rust file in `nova_editor` has a full static pass.

Not checked: no Cargo command, content lint, content generation, game/editor run, rendered visual inspection, probe, GPU measurement, wasm build, workspace test, or Clippy run. Saved artifacts and serialization behavior remain covered only by source/tests read in review 15.
