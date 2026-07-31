# Notes - KISS pass on the NOVA OS drawer HUD surfaces

## Structure

Three oversized files became folder modules. Public paths and the
`nova_gameplay` prelude are unchanged; every split is moves, renames and
visibility widening only.

### `hud/nova_os.rs` (8274) -> `hud/nova_os/`

| File | Lines | Concern |
| --- | --- | --- |
| `mod.rs` | 257 | Module docs, `NovaOsPlugin`, the re-export surface. |
| `style.rs` | 217 | Layout metrics, palette, z-layers, font helpers. |
| `components.rs` | 386 | Markers, `NovaOsMonitorSettings`, shell resources. |
| `crt.rs` | 464 | CRT material and the render-to-texture pointer/hover pipeline. |
| `content.rs` | 403 | Pure text/row builders for the read-only commands. |
| `input.rs` | 434 | Keyboard, gamepad and wheel handling. |
| `sound.rs` | 124 | Power cues and the ambient bed. |
| `shell.rs` | 565 | Header/footer/prompt reconcilers, the slide, the app surface. |
| `lists.rs` | 389 | Flight log model and the objective / log row lists. |
| `spawn.rs` | 718 | Shell setup/teardown and the header/main/footer regions. |
| `casing.rs` | 663 | Casing, bezel, glass and the chin controls. |
| `tests/` | 3767 | Nine modules, one per concern, over a shared rig in `tests/mod.rs`. |

### `hud/nova_os_ship.rs` (3208) -> `hud/nova_os_ship/`

`mod.rs` (173), `sections.rs` (542), `app.rs` (353), `scene.rs` (841),
`tests.rs` (1334).

### `hud/nova_os_map.rs` (2422) -> `hud/nova_os_map/`

`mod.rs` (146), `contacts.rs` (498), `app.rs` (223), `scene.rs` (618),
`tests.rs` (971).

`nova_os_pointer_rig.rs` (396) was already one cohesive concern and stayed put.

No file exceeds 1500 lines, so DoD 4 needs no exception.

### Visibility

Splitting a file full of private items forces the moved items to widen. The
scheme is uniform: items moved into a child module become `pub(crate)`, the
child modules stay private, and `nova_os/mod.rs` re-exports exactly the names
that `hud` and the sibling app modules already imported. The seven names only
`nova_os_pointer_rig` (a `#[cfg(test)]` module) uses are re-exported behind
`#[cfg(test)]` so a release build sees no unused re-export.

`SectionCode` and `MapContactCode` stay `pub` - they travel out through the
app preludes.

## Comments

DoD 3 is satisfied outright: `grep -rnE '//.*[0-9]{8}-[0-9]{6}'` over the
scope returns nothing, so there is no deliberate-reference list to record
here. Every tatr-ID clause was provenance ("task 20260726-214617: ...",
"see `tasks/20260728-115435/DECISION.md`"); the clause was deleted and the
surrounding constraint kept. Four sites where the constraint was doing real
work were promoted to `NOTE:` rather than left as bare prose:

- `crt.rs` - the overscan uniform must not be duplicated as a WGSL constant.
- `crt.rs` - applying the barrel inverse or skipping the overscan mis-places
  clicks by up to 27 px at the corners.
- `sound.rs` - the bed's own marker is what keeps it out of `pause_loops`.
- `nova_os_ship/scene.rs` - `left: 14` leaves a 4 px dead band in the blip.

### What was deliberately NOT cut

- **`PoC .case` / `PoC .caret` references.** These look like provenance but
  `examples/ui/nova_os_terminal_poc.html` is still checked in, so they are
  live pointers into a reference artifact, not dead history. Kept.
- **The rest of the non-doc comments.** The rubric makes bare prose fluff by
  default, and the burden is on keeping it. Reading them, they are almost
  entirely the categories the rubric says to keep: guards on a tuned value
  ("on-phase alpha 0.85, not 1.0, so the letter under the caret reads"),
  non-obvious Bevy behaviour ("read through the immutable `Deref` so an empty
  queue does not mark the terminal changed"), and headless-rig fallbacks.
  Almost none narrate what the code plainly does. Two sites were pure history
  and were deleted: a note that the flight-view tab handle had been removed,
  and its matching note in the toggle tests.
- **The quoted WGSL lines in `nova_os_pointer_rig.rs`.** They read as
  commented-out code but are the shader source the reference helper is
  transcribed from - the point of that helper is to not reuse the production
  formula.

## Defects found (not fixed here)

`20260731-174911` - the NOVA OS objective / flight-log row lists are dead in
production. Nothing in the shell spawns `NovaOsObjectivesListMarker` or
`NovaOsFlightLogListMarker`; only the tests do. That strands the whole row
build path in `lists.rs`, plus a one-variant `NovaOsObjectiveRowStatus` whose
assertions are tautological and a `#[cfg(test)]`-only
`NovaOsObjectiveStrikeMarker` nothing ever spawns. The `NovaOsFlightLog`
resource itself is live - the terminal commands and the boot banner read it.

## Doc links after the move (review round 1)

Moving items one module deeper re-pointed two relative doc paths at the wrong
module - `DRAWER_EXEMPT_Z`'s `super::HudNovaOsExempt` and `NovaOsBedSfx`'s
`super::super::audio`, both of which resolved on master. Re-anchored to
`super::super::HudNovaOsExempt` and `crate::audio`.

The same move broke fourteen more links that used to resolve inside one file
and now cross a private module boundary. Cross-module ones were qualified with
the sibling path (`[`super::shell::sync_nova_os_app_ui`]`); links to items the
private modules do not export, and the module-layout tables' own entries, are
plain code spans instead - those modules are private by design, so linking to
them is wrong rather than merely noisy.

`cargo doc --no-deps --document-private-items` now emits 3 warnings under
`hud/nova_os*`, against 5 on master. The three are pre-existing (`MapOrbit`,
`assign_map_contact_codes`, `assign_section_codes`); two of master's five went
away because the items they name became `pub(crate)`, so their docs are no
longer public documentation.

## Verification

| Check | Result |
| --- | --- |
| `cargo check --workspace --all-targets` | green, no warnings |
| `cargo fmt --check` | clean |
| `cargo test -p nova_gameplay --lib hud::nova_os` | 102 passed, 0 failed |
| HUID grep over the scope | no hits |
| `cargo doc` warnings under the scope | 3, vs 5 on master |
| `wc -l` over the scope | largest file 1334 |

The 102 tests are the same 102 that existed before the split - none were
added, removed, weakened or renamed.
