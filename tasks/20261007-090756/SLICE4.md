# Slice 4: menu, leave and window close (plan for approval)

Owner approved A+B+C. This file states the exact surface. All new items are
in `nova_menu` unless marked. Web keeps today's session-only New Game (D4a).

## nova_world_base (owning interface first)

```rust
impl WorldSaveSession {
    /// Never write again. Drops a wanted save; an in-flight write still
    /// finishes. Retry (D3a) calls it so nothing is written before the reopen.
    pub fn stop_saving(&mut self);           // spent = true, wanted = None
    /// A write is in flight.
    pub fn is_writing(&self) -> bool;        // writer.is_some()
}
```

## nova_menu: shared root

```rust
/// The folder saved worlds live in; `None` when the platform names none.
/// Inserted from `nova_assets::worlds_root()` unless a test inserted one.
#[derive(Resource, Clone)] #[cfg(not(wasm32))]
pub(crate) struct WorldsRoot(pub(crate) Option<PathBuf>);
```
`None` is visible: Create shows "This system has no folder for saved
worlds", Load lists that line, and both start nothing.

## world_setup.rs (Create)

- New marker `WorldNameField`. A name field (`text_field`, `max_chars(32)`,
  empty at open: no default name) above the seed field, native only.
- `read_world_seed` becomes `read_world_setup`: it validates the name with
  `world_slug` and the seed with `parse_world_seed`. Each field gets its
  own `TextFieldError`. Create is greyed until both are valid.
- `on_create_world` (native): `create_world(root, name)`. On `Err`, the
  name field gets the refusal text (`NameTaken`: "A world named <name>
  exists") and nothing starts. On `Ok`, insert `WorldSaveSession::created`
  and `OpenWorldSession`, then start as today.
- Web: no name field; one label line "Saved worlds need the desktop build."
  Create starts a session as today.
- Module doc: replace "Nothing here is saved".

## load_screen.rs (new, native only)

- A "Load" menu button under New Game. It opens `LoadPanel` (the
  Scenarios layout: `overlay_root`, `list_detail_screen`, footer Back).
- `WorldListings(Result<Vec<WorldListing>, WorldRefusal>)` resource, read
  from `list_worlds(root, packs)` each time the panel opens.
  `SelectedWorldSlug(Option<String>)`.
- Markers: `LoadPanel`, `LoadWorldList`, `LoadWorldDetails`,
  `LoadWorldRow { slug }`, `LoadWorldButton`.
- `refresh_load_list` / `refresh_load_details` rebuild on change, the same
  way as Scenarios. A row shows the name, or the slug when the header did
  not read, and the refusal. The details show name, seed, sector, credits,
  game version and saved time, or the refusal. Load is greyed on a refusal.
- `on_load_world`: `open_world`. On `Err`, the details show the refusal (a
  world locked since the list was read) and nothing starts. On `Ok`, queue
  `resume_world`, set `pick.0 = None`, `GameMode::NewGame`, and
  `GameStates::Playing`.

## save_status.rs (new, native only)

- `SaveStatusLine` marker. `sync_save_status_line` (Update, in `Playing`)
  spawns, updates or despawns one small text node at the top right from
  `WorldSaveSession::status()`: "World saved", "Saving world...",
  "Not saved yet", "Waiting to save: <why>", and "SAVE FAILED: <err>"
  (danger colour, stays until the next good save).

## leave.rs (new, native only)

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LeaveTarget { Menu, Exit, Retry }
/// A leave that waits on the world's save before it happens.
#[derive(Resource, Debug)]
pub(crate) struct PendingLeave { pub(crate) target: LeaveTarget }
pub(crate) fn begin_leave(target, &mut Commands, &mut WorldSaveSession, &mut Clocks);
pub(crate) fn drive_pending_leave(world: &mut World);   // exclusive, Update
pub(crate) fn sync_leave_overlay(..);                   // LeaveOverlay marker
pub(crate) fn on_leave_try_again(..);                   // Failed -> request_leave again
pub(crate) fn on_leave_without_saving(..);              // explicit consent
pub(crate) fn on_window_close_requested(..);            // Update
```
- Menu, Exit: `request_leave`. `PauseStates` stays `Paused` (input and
  sections off), and only the `FreezeOwner::PauseMenu` hold is released
  (D2a), so bodies settle. `drive_pending_leave` finishes only when the
  session is idle AND `Saved`, then removes the session (drops the lock)
  and does the transition or `AppExit`. On `Failed`, it re-takes the
  `PauseMenu` hold, and the overlay shows the error with "Try again" and
  "Leave without saving".
- Retry (D3a; the pause button reads "Load last save" when a session
  exists): `stop_saving`, then wait while `is_writing`, then remove the
  session, `open_world` the same slug, `resume_world`, and trigger
  `LoadScenario(current)`. On a refusal, the overlay shows it with "Back to
  Main Menu" (no write).
- The pause overlay is not drawn while `PendingLeave` exists.
- Without a `WorldSaveSession` (web, scenarios, editor), every button does
  what it does today.
- Window close: `nova_core` sets `close_when_requested: false`.
  `on_window_close_requested` reads `WindowCloseRequested`. With a session
  in `Playing`, it starts `Exit` (a second close while a leave is pending
  does nothing). Otherwise it writes `AppExit::Success` at once.

## Tests (each needs approval)

- Update `tests/world_setup.rs`: Create needs a name. The test app inserts
  `WorldsRoot(Some(tempdir))`. The existing Create test also asserts the
  world folder and the session.
- New `create_refuses_a_taken_world_name_inline_and_starts_nothing`.
- New `load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one`.
- New `leaving_a_saved_world_waits_for_the_leave_save_with_input_still_paused`
  (asserts virtual time advances, `PauseStates::Paused`, the state changes
  only after `Saved`, and the files on disk are generation 2).
- New `a_failed_leave_save_holds_the_clock_and_leaves_only_on_consent`.
- New `load_last_save_reopens_the_saved_world_without_writing`.
- New `window_close_with_a_saved_world_waits_for_the_leave_save`.
- P8 rendered frames: Create with name, Load list (one valid, one refused),
  status line, leave overlay.
- Update the examples that press Create (`screenshot_menu.rs`,
  `loop_world_start.rs`) to type a name.
