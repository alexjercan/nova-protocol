# REVIEW: allegiance markers (task 20260723-233446)

Summary: a well-built, faithful implementation that correctly navigates the deferred-require spawn-order hazard and matches the sibling HUD templates; the border-triangle mechanism holds up against the 0.18.1 shader, and no MAJOR issues were found.

## Findings

### Verified load-bearing claims (positives)

- POSITIVE (spawn order): re-derived independently. `PlayerSpaceshipMarker` and `Allegiance` are inserted by a DEFERRED `commands.entity(entity).insert(...)` inside nova_scenario's own `Add<SpaceshipRootMarker>` observer (`crates/nova_scenario/src/objects/spaceship.rs:409` player, `:447` AI), so at `setup_allegiance_marker` time neither is present. Spawning grey and fixing up via `Changed<Allegiance>` + an `Added<PlayerSpaceshipMarker>` SYSTEM is the correct resolution. The player-skip must be a system, not an `Add` observer: the grey layer is spawned via a deferred `commands.spawn`, so an `Add<PlayerSpaceshipMarker>` observer would run before that flush and find no layer to despawn. The Update system defers past the flush and is guaranteed to see the layer. `allegiance_markers.rs:224-250`.
- POSITIVE (border triangle): verified against `bevy_ui_render-0.18.1`. `extract_uinode_borders` (lib.rs:604-676) gates on `computed_node.border() != BorderRect::ZERO` and skips `is_fully_transparent()` sides, emitting only the top border with `BORDER_TOP`. `nearest_border_active` (ui.wgsl:124-143) assigns each fragment to the min-normalized-distance side, so a colored top over a 0-content box paints only the downward mitered wedge = a filled down-triangle. `ContentBox` is genuinely load-bearing: under the default `BorderBox`, a 0x0 node collapses the border box and `computed_node.border()` would be ZERO, failing the extract gate. Claim holds.
- POSITIVE (recolor coverage): `SetAllegiance` does `world.entity_mut(ship).insert(allegiance)` (`actions.rs:1097`), which trips `Changed<Allegiance>`; the require-default landing at spawn-settle also trips it. Both the spawn-settle recolor and the runtime flip are covered. A `None`/controller-None ship never gets an Allegiance and keeps grey - matches the DoD.
- POSITIVE (lifecycle): `remove_allegiance_marker` on `Remove<SpaceshipRootMarker>` despawns every matching layer; death despawns the ship, so the marker goes with it. No orphan path found. For the player (layer already despawned) the loop is a harmless no-op.
- POSITIVE (test fidelity): the `#[require]` spawn of `PlayerSpaceshipMarker` DOES reproduce the ordering hazard - the layer's `commands.spawn` is deferred even under require, so an `Add<PlayerSpaceshipMarker>`-observer regression would leave the player marked and fail the test. It exercises the real observers + Update systems. This is a genuine delivery guard, not a tautology.
- POSITIVE (docs): `web/src/wiki/hud.md` and `factions.md` accurately describe green/red/grey, own-ship-none, off-screen hide, and runtime flip; links `../hud/`, `../factions/`, `../targeting-radar/` all resolve to existing wiki files. Instrument-tier reclassification in the tier list is correct.

### MINOR

- MINOR (`allegiance_markers.rs:257-272`) `recolor_allegiance_markers` is O(changed_ships * all_triangles) and does not break after matching a ship's single triangle. At HUD scale (dozens) this is fine and `Changed` keeps the outer set tiny, but the inner loop rescans every triangle for each changed ship. If a cheap win is wanted, index triangles by target or `break` on match. Not a bug; flagged only because the task calls out "keep it cheap". The sibling `beacon_chips`/`item_highlights` recolor passes iterate the child set once, so this is a slight divergence.
- MINOR (test, `allegiance_markers.rs:349`) the App test never spawns an `AISpaceshipMarker`, so the require-default recolor path (grey-at-spawn -> red when `Allegiance::Enemy` lands via a later insert) is not exercised end to end; the `enemy` ship is spawned with `Allegiance::Enemy` inline at spawn time, which is the easy case. The runtime-flip assertion partially covers the "Allegiance changes after the grey layer exists" transition, so coverage is adequate, but an `AISpaceshipMarker`-spawned wingman would more faithfully mirror production and the module's own spawn-order narrative.

### NIT

- NIT (`allegiance_markers.rs:96-97`) `AllegianceMarkerTargetEntity` derives `Copy` while the sibling `BeaconChipTargetEntity` / `ItemHighlightTargetEntity` do not. Harmless (Entity is Copy) but a small divergence from the template.
- NIT (`allegiance_markers.rs:267`) the `border.top != color` guard avoids a needless change-tick bump - good - but the same guard is absent from the sibling `breathe_item_highlights`; consistency only, no action needed.
- NIT (verification) the border-triangle visual has no rendered probe/screenshot evidence in the task folder; correctness was argued from the shader source only (and re-verified here). The DoD's manual/probe check is deferred to the perf task 20260723-233453. Worth a one-frame screenshot when that runs, since the whole look rides on an undocumented shader side effect.

## Post-review (author)

Both MINORs addressed on the branch after APPROVE:
- `recolor_allegiance_markers` now `break`s on the matched triangle (one per ship), closing the O(changed*triangles) divergence from the sibling passes.
- The App test now also spawns an `AISpaceshipMarker` hostile, so the require-default recolor path (Enemy allegiance arriving through the marker requirement, not an inline insert) is exercised end to end; it asserts red. All 3 module tests still pass.

The NITs are left as-is by design: `Copy` on the target newtype is a deliberate ergonomic choice (Entity is Copy), and the rendered-visual evidence is the deferred manual/probe check batched with perf task 20260723-233453.

## Verdict

No MAJOR findings; the load-bearing spawn-order, border-shader, recolor, and lifecycle claims all re-derived clean, and the test is a real delivery guard.

- VERDICT: APPROVE
