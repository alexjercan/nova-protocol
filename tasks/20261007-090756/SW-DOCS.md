# SW-DOCS: docs and changelog for resumable worlds

Scope: CHANGELOG.md, docs/architecture.md, web/src/wiki/**/*.md, and the
doc comments (//! and ///) in nova_world_base/src/lib.rs, nova_world/src/
frozen.rs, nova_menu/src/world_setup.rs and nova_ship/src/camera/zoom.rs.
No code touched. Nothing staged or committed.

## Files changed, one line each

- `web/src/wiki/getting-started.md` - added a "Saved worlds" section (folder
  location, Create, save triggers, leave overlay, death, what a Load
  restores, refusal table, no Delete); updated the New Game bullet and added
  a Load bullet in the menu rundown.
- `web/src/wiki/scenarios.md` - Open World bullet: destruction/looting now
  persist (session and disk), replacing the false "not saved yet" claim.
- `web/src/wiki/interface.md` - two fixes: repair's spent plates persist in
  a saved Open World; the Inventory pane's items/credits persist in a saved
  Open World. Both keep the true "nothing saved" case for other scenarios.
- `web/src/wiki/flight-autopilot.md` - chase-camera zoom is now kept in a
  saved world's `CameraView`.
- `web/src/wiki/sections/cargo-intake.md` - canisters are now saved in a
  saved Open World.
- `web/src/wiki/sections/mining-beam.md` - same canister fix, shorter form.
- `docs/architecture.md` - `nova_world_base` row: "nothing is saved to disk"
  replaced; `nova_menu` row: added Create/Load/status line/leave overlay/
  Retry; `ClockFreeze` bullet: added `FreezeOwner::WorldResume` with what
  holds it and when.
- `CHANGELOG.md` - `[Unreleased]`: merged the frozen-sectors entry with the
  saved-worlds behavior, added a saved-worlds mechanism entry, a **(breaking)**
  Modding & Mod Portal entry, two Interface & HUD entries (one pre-existing,
  kept), and a Web & Platform entry.

Not changed (checked, already correct - see "Doc comments checked" below):
`crates/nova_world_base/src/lib.rs`, `crates/nova_world/src/frozen.rs`,
`crates/nova_menu/src/world_setup.rs`, `crates/nova_ship/src/camera/zoom.rs`.
These four files are mid-edit by another worker (uncommitted); I only read
them, I made no edits in them.

## Evidence for each claim

Canisters go into the world save (live window and off-window), verified in
code rather than assumed:
- `crates/nova_world/src/frozen.rs:565-571` - `PersistentBody` type includes
  `With<CargoCanister>`.
- `crates/nova_world/src/frozen.rs:463-464` - `freeze_body` turns a canister
  into `FrozenBodyType::Canister`.
- `crates/nova_world/src/frozen.rs:774-813` - `snapshot_sectors` runs
  `freeze_sector_bodies` over every LIVE root's children (the 125-cell
  window), not just the off-window ledger, so a live-window canister is
  captured too.

Spent plates, ammo and reloads are part of the saved ship:
- `crates/nova_ship/src/sections/frozen.rs:83-85,224-226` - `FrozenSection`
  carries `ammo`, `suspended_ammo`, `reload`.
- `crates/nova_ship/src/sections/frozen.rs:50-68` - plates and decor are
  frozen fixtures on a section.
- `crates/nova_scenario/src/objects/spaceship.rs:562-594,672-690` -
  `FrozenShip`/`FrozenShipState` carry `inventory` (hold) and `credits` for
  ANY ship, not only the player, so a generated ship's stock and credits are
  saved too.

Key bindings are saved:
- `crates/nova_scenario/src/objects/spaceship.rs:663-671` - `freeze_ship`
  overwrites the controller's `input_mapping` with the live table before
  freezing.

Chase-camera zoom is saved:
- `crates/nova_ship/src/camera/resume.rs:30-40` - `CameraView.zoom`.
- `crates/nova_ship/src/camera/resume.rs:79` - captured from
  `ChaseZoom.manual`.

Death writes nothing; the last good save stays:
- `crates/nova_world_base/src/save/session.rs:310-341` - a save request is
  dropped with "no player ship in an open world to save; the last save is
  kept" when there is no player.

Retry in a saved world loads the last save instead of restarting:
- `crates/nova_menu/src/pause.rs:403-415` - the button label is "Load last
  save" in a saved world, "Retry" otherwise.
- `crates/nova_menu/src/leave.rs:12-16,74,119,326` - `LeaveTarget::Retry`
  writes nothing, waits for any writer, and reopens the world from disk.

Back to Main Menu / Exit / window close wait on the leave save:
- `crates/nova_menu/src/leave.rs:1-18` (module doc), `:334` (
  `on_window_close_requested`).
- `crates/nova_core/src/lib.rs:475,746` - `close_when_requested` is `false`
  on native (the game's own handler fires instead), `true` only on
  `wasm32`.

Save status line and leave overlay text (exact strings quoted in the wiki):
- `crates/nova_menu/src/save_status.rs:15-20` - `status_text`: "Not saved
  yet", "World saved", "Saving world...", "Waiting to save: {why}", "SAVE
  FAILED: {err}".
- `crates/nova_menu/src/leave.rs:200-285` - "Saving world...", "Try again",
  "Leave without saving".

World folder location:
- `crates/nova_assets/src/storage.rs:133-148` - `worlds_root()`:
  `$NOVA_CONFIG_ROOT/worlds` when the test/tooling override is set, else
  `dirs::data_dir()/nova-protocol/worlds`.
- The per-OS example paths in the wiki (`~/.local/share/...` Linux,
  `~/Library/Application Support/...` macOS, `%APPDATA%\...` Windows) are
  the documented behavior of the `dirs` crate's `data_dir()`, not something
  I ran on each OS. Labelled unverified-by-execution below.

World name rule and Create refusal:
- `crates/nova_world_base/src/save/mod.rs:57` - `WORLD_NAME_MAX = 32`.
- `crates/nova_world_base/src/save/mod.rs:504-530` - `world_slug`: trims,
  refuses empty, over 32 chars, or a character outside
  `[A-Za-z0-9 _-]`.

Load refusal strings (table in the new wiki section), read off the enum's
`Display` impl, which is also what the Load screen renders on a row:
- `crates/nova_world_base/src/save/mod.rs:157-206` - `WorldRefusal` and its
  `Display`.
- `crates/nova_menu/src/load_screen.rs:302-306,392-395` - the row and the
  details panel both print `refusal.to_string()`.

240-second resume bound and the loading-screen progress line:
- `crates/nova_world_base/src/save/transients.rs:56` -
  `WORLD_RESUME_SECONDS_MAX: f32 = 240.0`.
- `crates/nova_core/src/loading_screen.rs:524` - `format!("RESTORING SECTORS
  {} / {}", progress.live, progress.desired)`.

Transient kinds a Load restores, and visual-only exclusion:
- `crates/nova_world_base/src/save/transients.rs:68-81` -
  `FrozenTransientType`: `Round`, `Torpedo`, `ShedFixture`, `RockChunk`,
  `DetachedPiece`.
- `crates/nova_world_base/src/save/transients.rs:1-8` (module doc) - "A body
  with no physics, such as a spark or a light, is visual only and is not
  saved."

**(breaking)** section/object id rule:
- `crates/nova_scenario/src/objects/ship_design.rs:441,443,462,465,477-488` -
  `ShipDesignError::ReservedSectionId`/`DuplicateSectionId`,
  `section_id_errors`.
- `crates/nova_assets/src/merge.rs:656` - the bundle gate calls
  `section_id_errors` on every `Content::Ship` before any merge (base
  content fails fatally, a mod is quarantined).

`FreezeOwner::WorldResume` (architecture.md ClockFreeze line):
- `crates/nova_gameplay/src/freeze.rs:27-64` - the variant, its doc comment
  ("Held until every saved sector is live and the saved transients are
  back; released at once when the Load is refused"), and its slot in
  `FreezeOwner::ALL`.

Main-menu Load button is desktop-only:
- `crates/nova_menu/src/menu_ui.rs:111-123` - `#[cfg(not(target_arch =
  "wasm32"))]` on the Load button's spawn.

## Full new/changed CHANGELOG entries, with character counts

All counts are the joined (unwrapped) bullet text, excluding the leading
`- ` marker, matching how the file wraps them across lines.

Gameplay & Flight (merged with the existing frozen-sectors entry):
```
Open-world sectors you leave now freeze and, with a named save, come back
exactly as left: mined rocks, owed ore, looted ships, canisters, wrecks, a
docked partner and in-flight rounds and torpedoes.
```
199 characters.

Gameplay & Flight (new):
```
New Game now names and saves its world: Create, Load, a save on each sector
crossing and on leaving, and death keeps the last good save instead of
losing it.
```
157 characters.

Modding & Mod Portal (new, **(breaking)**):
```
**(breaking)** A scenario object or ship section id may not contain '/'; a
duplicate section id in a ship fails lint and load, not a skipped section.
```
149 characters.

Interface & HUD (new, the existing GOTO entry is unchanged and kept):
```
New Game's Create now asks for a world name. A Load screen lists saved
worlds with refusal reasons, a status line shows the open world's save
state, and leaving shows a save overlay.
```
182 characters.

Web & Platform (new section in Unreleased):
```
The web build keeps no saved worlds: New Game stays a one-off session
there, with no Load button or world name field.
```
117 characters.

All five are at most 200 characters. No bug introduced and fixed within
this branch is listed. Reread the whole `[Unreleased]` block after the
edits; order matches the release-banner heading order.

## Checks run

- `nix develop --command mdbook build` - succeeded (docs/architecture.md).
- `cd web && npm ci && npm run build` - webpack compiled successfully;
  confirmed in the built output that `id="saved-worlds"` exists on
  `dist/wiki/getting-started/index.html` and that the cross-links from
  `dist/wiki/interface/index.html` and `dist/wiki/scenarios/index.html`
  resolve to `../getting-started/#saved-worlds`.
- Did not run `npm run lint`/`format:check`/`test`: those check the site's
  TypeScript source, which I did not touch; the build is the affected check
  for a markdown-only change.

## Unverified

- The per-OS example save-folder paths quoted in the new wiki section
  (`~/.local/share/...`, `~/Library/Application Support/...`,
  `%APPDATA%\...`) are the `dirs` crate's documented platform mapping for
  `data_dir()`, not something I ran on macOS or Windows to confirm.
- Did not re-run the Rust test suite for any of the `nova_world_base`,
  `nova_scenario`, `nova_ship` or `nova_menu` code I read as evidence: out
  of scope (docs-only task, no code edited, and the instruction is not to
  run workspace-wide tests).
