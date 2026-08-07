# L6 - nova_editor

**Baseline: NEUTRAL.** Behavior-only.

Findings: **F11, F29, F30, F31, F32**.

**Depends on:** L1.

**Five defects in 2,378 LOC - the worst defect density in the workspace, and
the crate was not on the epic's list at all.** It is small enough to read whole
in one sitting, which no other lane's scope is. Keep it as one lane and one
reader: whoever fixes one of these is already holding the entire crate in their
head.

## F11 - five panic sites on a mod overlay

```rust
// crates/nova_editor/src/placement.rs:42  and :100
sections.get_section("reinforced_hull_section").unwrap()
sections.get_section("basic_controller_section").unwrap()
// plus panic!() on kind mismatch or missing id at :46, :104, :205
//   ^ :205 is the FIFTH site; the review reported four.
//   A mod overlay redefining or dropping either id panics the process on
//   "New Hull Ship". Every other catalog lookup in the codebase logs and skips.
```

```rust
// CHANGE  placement.rs - one accessor, five call sites
/// The built-in section the editor seeds a new ship with. `None` (logged) when
/// a mod overlay dropped or retyped the id - the editor must degrade to "no
/// preview", never panic.
fn required_section<'a>(
    sections: &'a GameSections,
    id: &str,
    kind: SectionKind,
) -> Option<&'a SectionConfig>
```

**Test:** a missing catalog id logs and skips rather than panicking. The crate
has 13 tests and no in-workspace dependents, so nothing pins it and nothing
else breaks - which cuts both ways.

## F29 - the placed section binds whatever key is held

```rust
// crates/nova_editor/src/placement.rs:315
//   Placement captures whatever key happens to be held as the new section's
//   binding, and the editor camera is driven by those same keys. Hold Space or
//   W while placing a turret and the turret fires on every burn in flight.
//   ButtonInput::get_pressed() iterates a HashSet, so W+D makes the bind
//   nondeterministic.
```

```rust
// CHANGE  placement.rs:315
//   Exclude the editor's own camera keys from the candidate set, and take
//   just_pressed rather than pressed so a held movement key cannot be
//   captured. Ordering must be deterministic - sort before picking.
fn capture_binding(input: &ButtonInput<KeyCode>, reserved: &[KeyCode]) -> Option<KeyCode>
```

## F30 - keybind chips block the picking ray

```rust
// crates/nova_editor/src/keybind.rs:60
//   Keybind chips are root UI nodes with NO Pickable override, so they block
//   the picking ray to the sections they label. Reads to a player as "clicking
//   randomly does nothing".
//   card.rs:24 and tooltip.rs:22 define an IGNORE Pickable for exactly this.
+ Pickable::IGNORE
```

One line. Copy the constant the two siblings already use.

## F31 - re-entering the Editor leaves a ghost config

```rust
// crates/nova_editor/src/lib.rs:110   <- the OnEnter(ExampleStates::Editor)
//    registration. CITATION RE-ANCHORED: the DespawnOnExit markers the review
//    cited are at placement.rs:32,90.
//   Re-entering the Editor never resets or rebuilds PlayerSpaceshipConfig.
//   Sandbox -> build -> Play -> F1 back to Editor: no preview exists, every
//   click is dropped, yet Play spawns the old ship from the surviving config.
```

```rust
// NEW  registered on OnEnter(ExampleStates::Editor), before the spawners
/// Rebuild the preview from `PlayerSpaceshipConfig` on every entry, so a
/// second visit shows the ship the first visit built instead of nothing.
fn rebuild_editor_preview_on_enter(...)
```

Decide explicitly which of the two behaviors is intended - **reset the config**
or **rebuild the preview from it**. The bug is that neither happens.

## F32 - rebinding accepts a conflicting key

```rust
// crates/nova_editor/src/keybind.rs:187
//   Click-to-rebind accepts any key with NO conflict check. Authored content
//   with that mapping is rejected by `scenario_input_overlaps`, but an
//   editor-built ship is constructed at runtime and never linted.
```

```rust
// CHANGE  keybind.rs:187 - reuse the lint's rule rather than restating it
fn binding_conflict(config: &PlayerSpaceshipConfig, key: KeyCode) -> Option<SectionId>
//   If `scenario_input_overlaps` can be called on a runtime config directly,
//   call it. A second implementation of "do these bindings overlap" is how the
//   editor and the lint drift apart.
```

## Verified by

The crate's 13 tests, plus one new test per finding. No in-workspace dependents
means the lane is low-risk to land and has almost no existing safety net -
budget for writing the tests, not just the fixes.
