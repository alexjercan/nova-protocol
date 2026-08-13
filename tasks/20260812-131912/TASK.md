# Epic: the editor is the star of v0.11.0

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.11.0, epic, editor

v0.11.0 theme per owner (2026-08-12): the editor is the main focus. The
parts_viewer gallery from closed spike `20260812-100246` sets the UX bar:
visual, previewable, and filterable.

Release story: a player opens the editor, browses parts and ships visually,
snaps a ship together, stamps prefab ships, builds a scenario around them,
saves it, and plays it. The sprint starts by removing known automation defects
so this path has trustworthy evidence.

## Ordered sprint plan

### Phase 0 - Make the evidence trustworthy

1. `20260805-015136` (p98) - fix `ScriptBuilder::on_enter` composition. This
   removes a silent hook-loss bug before any new editor harness work composes
   multi-action steps.
2. `20260806-140928` (p96) - reproduce and fix the intermittent combat
   screenshot torpedo intercept. Stabilize the release-wide screenshot fleet
   before editor screenshot updates increase its use.
3. `20260804-190142` (p94) - add wheel synthesis and cover the scenarios row
   below the fold. This completes the pointer vocabulary needed by long
   galleries and scenario lists.

Checkpoint: full correctness probe is green repeatedly, not only once.

### Phase 1 - Establish the visual editor surface

4. `20260812-131852` (p90) - build the gallery section picker with previews,
   dropdowns, search, focus turntable, and selection. Reuse it as the visual
   foundation for later section and ship catalogs.
5. `20260804-134347` (p85) - add the missing NOVA OS end-to-end system run.
   This is independent of editor placement but closes a known live RTT click
   coverage gap while the shared UI automation vocabulary is fresh.

Checkpoint: the player can browse and select a semantic part, and all major UI
surfaces have live system evidence.

### Phase 2 - Make semantic ship construction real

6. `20260812-131005` (p80) - snap semantic parts by authored link points,
   provide rolled placement, real-mesh ghosts, occupied-socket rejection, save
   and reload parity, and unhide parts from the palette.
7. `20260813-224826` (p75) - prototype and integrate persistent asteroid
   carving, detached chunks, and explicit health semantics. Treat semantic ship
   carving as a gated spike after asteroid performance and appearance pass.
8. `20260812-131901` (p70) - reuse the gallery for complete ship previews,
   stamping, and in-scene duplication.

Checkpoint: browse -> select -> snap -> save -> reload -> play works for parts;
prefab ships can be stamped and duplicated; asteroid hits leave bounded,
persistent geometric damage with correct collision and lifecycle behavior.

### Phase 3 - Expand from ship editor to scenario editor

9. `20260714-081703` (p60) - spike the current delta first, then add scenario
   object placement, objective/event wiring, factions, and RON save/load on top
   of the established gallery and placement systems. Deliver in vertical
   slices: object placement and round-trip first, then objectives/events.

Checkpoint: one authored scenario with ships and non-ship objects saves, reloads,
and completes its player path.

### Phase 4 - Broaden input after the editor interaction model settles

10. `20260714-001140` (p50) - run its required spike, then split it into two
   independently shippable child tasks:
   - gamepad focus/navigation for menus and the finished editor;
   - mobile virtual flight controls and touch pointer support.
   Implement gamepad first. Mobile is the final stretch item because its layout
   should target stable editor/menu and gameplay interactions, not moving ones.

Checkpoint: keyboard/mouse remains green; gamepad can operate menus and editor;
touch can operate the web gameplay path.

## Scope decisions

- `20260812-100246` is closed, not duplicated in this sprint. Its design and
  prototype were consumed by the v0.10.0 link-point and semantic-part work.
- `20260812-100256` is closed as completed research. Its planet/asteroid/prop
  escalation is valid future work but is outside the editor-focused v0.11.0
  story.
- `20260812-131005` now owns only remaining editor snapping and generic recipe
  candidate generation. It no longer claims shipped GLBs, prototypes, or the
  completed cube-to-parts migration.
- `20260813-224826` consumes the completed round-3 carving design. Asteroid
  carving is the implementation path; semantic ship carving remains gated on a
  separate representation and integrity spike.
- Bugs block feature phases. Chores and nice-to-haves do not displace the core
  editor path if the sprint must be cut; Phase 4 is the first release-cut line,
  followed by the later objective/event slices of Phase 3.

## Release definition of done

- Every child task retained for the release is closed or explicitly cut before
  tagging.
- The complete editor player path has harness coverage and reviewable rendered
  output.
- Full correctness probe, affected content lint, Rust checks, and web CI pass.
- Editor, modding, scenario, input, and screenshot documentation ships with the
  behavior it describes.
