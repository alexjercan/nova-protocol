# SW-REFUSE: `refuse_resumed_world` + Loading screen resume gate

Scope: `crates/nova_menu/src/load_screen.rs`, `crates/nova_menu/src/lib.rs`
(registration only), `crates/nova_core/src/loading_screen.rs`.
`crates/nova_menu/src/leave.rs` was read but not edited - see "1. Teardown"
below for why.

## 1. `refuse_resumed_world` (item 1)

New `pub(crate)` fn in `crates/nova_menu/src/load_screen.rs:155-196`
(after the imports; before `refresh_load_list`):

```rust
pub(crate) fn refuse_resumed_world(
    mut commands: Commands,
    refused: Res<WorldResumeRefused>,
    mut listings: ResMut<WorldListings>,
    mut state: ResMut<NextState<GameStates>>,
    mut pause: ResMut<NextState<PauseStates>>,
    panel: Option<Single<&mut Visibility, With<LoadPanel>>>,
) {
    commands.remove_resource::<WorldSaveSession>();
    state.set(GameStates::MainMenu);
    pause.set(PauseStates::Unpaused);
    let Some(mut panel) = panel else {
        return;
    };
    if let Ok(rows) = &mut listings.0 {
        if let Some(row) = rows.iter_mut().find(|row| row.folder.slug == refused.slug) {
            row.header = Err(WorldRefusal::Io(refused.reason.clone()));
        }
    }
    **panel = Visibility::Visible;
    commands.remove_resource::<WorldResumeRefused>();
}
```

Registered in `crates/nova_menu/src/lib.rs` (new import at lib.rs:88-91 adding
`refuse_resumed_world` to the existing `load_screen::{...}` use, and a new
`#[cfg(not(target_arch = "wasm32"))] use nova_world_base::prelude::WorldResumeRefused;`
at lib.rs:97-98), and wired as its own ungated system right after the Load
picker's chained pair (lib.rs ~323-329):

```rust
#[cfg(not(target_arch = "wasm32"))]
app.add_systems(
    Update,
    refuse_resumed_world.run_if(resource_exists::<WorldResumeRefused>),
);
```

**Teardown is NOT shared with `leave.rs`.** The Menu-target body of
`on_leave_without_saving` (leave.rs:305-328) is three lines:
`commands.remove_resource::<WorldSaveSession>()`,
`state.set(GameStates::MainMenu)`, `pause.set(PauseStates::Unpaused)`.
Extracting a shared helper would be a new `pub(crate)` fn beyond
`refuse_resumed_world`, which item 3 forbids; duplicating three lines inline
is cheaper and keeps `leave.rs` untouched, matching "(only if the teardown is
shared)" in the file-ownership note.

### Schedule placement and ordering vs. `nova_world_base`

`WorldResumeRefused` is inserted by `nova_world_base::save::refuse`, called
from `restore_resumed_transients` (`crates/nova_world_base/src/save/transients.rs:436-440`,
called at `transients.rs:345-349`), which runs in `Update`,
`.after(NovaWorldSystems::Retire)`, gated on
`resource_exists::<ResumedTransients>` (`crates/nova_world_base/src/lib.rs:196-199`).
That system is `pub(crate)` to `nova_world_base` - not exported - so
`refuse_resumed_world` cannot carry an explicit `.after()` against it. None is
needed: `refuse_resumed_world`'s own `run_if(resource_exists::<WorldResumeRefused>)`
is evaluated against the resource's state at the start of this system's own
turn, not "at the top of the frame", so if the two systems run in ambiguous
order within the *same* Update pass and `nova_world_base`'s insert lands after
`refuse_resumed_world`'s turn, the refusal is picked up on the *next* frame's
Update pass instead - one frame of latency, not a dropped refusal. This is the
same tolerance every other `resource_changed`-gated redraw in this plugin
already accepts (see `refresh_load_list`/`refresh_load_details`, lib.rs ~323-334).

The harder ordering fact is **inside** `refuse_resumed_world` itself: the Load
panel (`LoadPanel`, spawned by `setup_menu_ui` only at
`OnEnter(GameStates::MainMenu)`, `crates/nova_menu/src/menu_ui.rs:454-461`,
`DespawnOnExit(GameStates::MainMenu)`) does not exist on the frame this system
first reacts, because that frame is still `Playing` (`NextState::Pending(MainMenu)`
does not apply until the *next* frame's `StateTransition`, which runs *before*
`Update` in the schedule order `PreUpdate -> StateTransition -> Update`). So:

- Frame N (still `Playing`, no `LoadPanel` yet): the teardown runs
  unconditionally; `panel` is `None`; the fn returns early, **without**
  removing `WorldResumeRefused`.
- Frame N+1 (`StateTransition` has already run `OnEnter(MainMenu)` ->
  `setup_menu_ui` spawned a fresh, hidden `LoadPanel`): `refuse_resumed_world`
  runs again (still gated true), finds the panel, writes the row, opens the
  panel, and removes `WorldResumeRefused` - which is what stops it running a
  third time.

The teardown block repeating on frame N+1 (remove an already-removed
`WorldSaveSession`, re-`set` an already-pending state) is a deliberate
no-op repeat rather than a second system, since item 3 allows only the one
fn. This covers **both** entry points named in the task: the Load screen
(already in `MainMenu -> Playing`, so the state-set is what *starts* the
transition out) and the `leave.rs` Retry path (`LeaveTarget::Retry`,
begun from `pause.rs:630-650`'s `on_retry`, which never visits `MainMenu`
before the refusal - this system is what first sends it there).

### Does the reason survive the menu's own listing refresh? (yes, for one case - flagging the other)

`WorldListings` (`load_screen.rs:28-35`) is `init_resource`'d once by
`NovaMenuPlugin::build` (lib.rs:168) and after that is written **only** by
`on_load_screen` (load_screen.rs:66-83, re-reads `list_worlds` from disk,
every time the Load button is pressed) and `on_load_world`'s `Err` branch
(load_screen.rs ~141-148, patches one row). **Nothing runs on
`OnEnter(GameStates::MainMenu)` that touches `WorldListings`** - I grepped
`lib.rs` for every `OnEnter(GameStates::MainMenu)` registration
(lib.rs:238-246, 485) and none of them is load-screen related. So a row
`refuse_resumed_world` patches stays patched across the `Playing -> MainMenu`
transition; `refresh_load_list`/`refresh_load_details`
(lib.rs ~323-329, chained, `run_if(resource_changed::<WorldListings>)`,
`run_if(in_state(MainMenu))`) redraw it because `ResMut<WorldListings>`
deref-mut inside my `if let Ok(rows) = &mut listings.0` branch marks the
resource changed on the exact frame the panel opens.

**This only works when `WorldListings` already has a row for the refused
slug** - i.e., it was populated by a *previous* `on_load_screen` read that
found this world on disk. I traced both ways a `WorldResumeRefused` can
exist back to their one shared cause, `resume_world`
(`crates/nova_world_base/src/save/session.rs:212`, the only fn that calls
`hold_resumed_transients` and therefore the only path that can ever produce
`ResumedTransients`/`WorldResumeRefused`):

- `grep -rn "resume_world(" crates/nova_menu/src` -> exactly two call sites:
  `load_screen.rs:135` (`on_load_world`, which runs only after the player
  already opened the Load panel and selected the row - so `WorldListings`
  has the row) and `leave.rs:153` (the Retry command, inside
  `drive_pending_leave`'s `LeaveTarget::Retry` arm).
- Retry is reachable from `on_retry` (`pause.rs:630-650`), gated only on
  `Option<ResMut<WorldSaveSession>>` existing - **not** on how that session
  was created. `on_create_world` (New Game's Create,
  `world_setup.rs:318-363`) builds a session with
  `WorldSaveSession::created(...)` (world_setup.rs:349-354) and never calls
  `resume_world` and never touches `WorldListings`.
- So: a player who starts a brand-new world (New Game -> Create) and then
  hits the pause menu's Retry ("Load last save") on it is reopening that
  world through `resume_world` for the **first** time this session, with no
  prior `on_load_screen` read ever having put a row for that slug into
  `WorldListings`. If that resume is refused, `rows.iter_mut().find(...)` in
  my fn finds nothing, the write is a silent no-op, and the reason is lost -
  the player lands in `MainMenu` with no explanation, and a later Load-panel
  open re-reads the (unchanged, still-good) save fresh from disk and shows no
  error either.

I implemented exactly what was specified (find-and-patch, mirroring
`on_load_world`'s own `Err` handling) and did **not** invent a fix for this
gap, per the task's "if it cannot... STOP and report the options" clause.
Options, none applied:

- **A - insert a synthetic row when none is found.** Build a `WorldListing`
  from `WorldSaveSession::folder()` (already a public getter, used identically
  at `leave.rs:138`/`load_screen.rs` call sites) with `header: Err(reason)`,
  and push it into `listings.0`'s `Ok(rows)` instead of only patching.
  Needs no new type or field. Changes an implicit invariant though: every
  other `WorldListing` in `WorldListings` today came from an actual
  `list_worlds` disk read; a synthetic one would not, and would vanish (or be
  silently superseded) the next time `on_load_screen` re-reads the root -
  which is probably fine (the reason is just as temporary as the pre-existing
  `on_load_world` refusal pattern) but is a behavior nobody asked for yet.
- **B - a small new resource** (e.g. `PendingLoadRefusal { slug, reason }`)
  that `on_load_screen` consults and overlays onto its fresh read before
  replacing `WorldListings`. Explicit new resource - exactly what the task
  told me to stop on rather than add unasked.
- **C - accept the gap.** A brand-new world is the least likely case to ever
  hit `WORLD_RESUME_SECONDS_MAX` (it has nothing saved to stream back in
  except itself), so the player-visible cost may be acceptable; document it
  rather than build for it.

I recommend **A**: it is the smallest change, uses only types that already
exist, and makes `refuse_resumed_world`'s guarantee uniform ("the reason is
always on some row") instead of conditional on how the session was opened -
but it is a scope decision belonging to the owner, not something to decide
silently.

### `WorldRefusal::Io` is a borrowed variant, not a perfect fit - flagging

`row.header: Result<WorldSaveHeader, WorldRefusal>` forces the `Err` arm to
be a `WorldRefusal` (`crates/nova_world_base/src/save/mod.rs:146-150`), a
closed enum I cannot extend (it lives outside my file ownership, and item 3
forbids new types/variants anyway). None of its variants
(`mod.rs:155-180`) are "this world did not come back in time" - the two with
an unprefixed `Display` (so the authored `reason` string shows verbatim, with
no misleading prefix glued on) are `InvalidName(String)` and `Io(String)`
(`mod.rs:182-200`). I used `Io`, since "an I/O-shaped thing between the save
and now" is a closer miss than "the name is invalid". Flagging rather than
treating as settled: if a dedicated variant (e.g. `ResumeTimedOut(String)`) is
wanted, that is a `nova_world_base::save::mod.rs` change outside this file
ownership and outside item 3's "no new types" - the owner's call.

## 2. `nova_core/src/loading_screen.rs` (item 2)

**Implemented - the "stays up" gate**, needing no new type: added
`#[cfg(not(target_arch = "wasm32"))] use nova_world_base::prelude::WorldResumeProgress;`
(loading_screen.rs:39-40) and a new `#[cfg(not(target_arch = "wasm32"))]
world_resume: Option<Res<WorldResumeProgress>>` parameter to
`dismiss_scenario_load_screen` (loading_screen.rs ~418-427), with the early
return extended exactly like the existing two gates
(loading_screen.rs ~444-447):

```rust
#[cfg(not(target_arch = "wasm32"))]
if world_resume.is_some() {
    return;
}
```

placed directly after the `is_settling`/`ScenarioPreload` check, before the
dwell and cap math, matching "before the dwell and the cap" and the existing
`is_settling`/`ScenarioPreload` pattern (loading_screen.rs ~403-440 in the
pre-edit file). This half needed zero new types, fields or functions, so it
is implemented and checked clean (see Checks below).

**STOPPING on the "RESTORING SECTORS `<live>` / `<desired>`" panel line.**
Every other dynamic element in this panel - `LoadingCursorMarker`,
`LoadingDotsMarker`, `LoadingSweepMarker` (loading_screen.rs:121-133) - is a
private marker `#[derive(Component)]` spawned once inside `loading_panel`
(loading_screen.rs:184-238) and then found again every frame by
`animate_loading_screen`'s typed queries (loading_screen.rs:460-492) to
redraw it. There is no existing marker, resource, or `Name`-keyed convention
in this file for "a line that only exists conditionally and must redraw on a
changed count" - the closest things (`LoadingDotsMarker`'s text, the field
note slot) are either unconditional or built once from data that does not
change after spawn. Giving the new line the same idiom (so a changed
`live`/`desired` redraws it, and so it only shows on the scenario screen, not
the boot screen, which never has `WorldResumeProgress`) needs at minimum one
new private marker component - something item 3 ("no new types... in this
task") rules out everywhere, not just in `load_screen.rs`.

I did not invent a type-free workaround (e.g., matching the line by spawned
`Name` text, or re-purposing `LoadingDotsMarker`'s text to sometimes show a
sector count instead of marching dots) because both depart from this file's
own established pattern for exactly this problem, and the task says not to
invent one. Options, none applied:

- **D - add one new private marker** (e.g. `LoadingResumeMarker`), spawned as
  an always-present (possibly empty-text) child of `loading_panel`'s column,
  and a small addition to `animate_loading_screen` (or a sibling system) that
  sets its `Text` from `WorldResumeProgress` each frame it exists and clears
  it otherwise. Matches the file's own conventions; is the smallest version
  of a new type.
- **E - match by `Name`** instead of a typed marker (`Name::new("Loading
  Resume Line")`, queried via `Query<(&Name, &mut Text)>`). No new type, but
  no other line in this file is found this way, and it is fragile to a label
  edit.
- **F - ship the "stays up" gate only this round**, leave the visible line for
  a follow-up once the type question is settled.

I recommend **D** and left the file at **F** (gate shipped, line not
attempted) pending that decision.

## 3. Checks

```
$ nix develop --command cargo check -j 8 -p nova_menu -p nova_core --tests
   Checking nova_gameplay v0.15.0 ... (full dependency chain)
   Checking nova_world_base v0.15.0 (...)
   Checking nova_menu v0.15.0 (...)
   Checking nova_console v0.15.0 (...)
   Checking nova_core v0.15.0 (...)
    Finished `dev` profile [optimized + debuginfo] target(s) in 31.02s
```
Clean, no warnings from either crate (only the pre-existing workspace-wide
`proc-macro-error2` future-incompat notice, unrelated).

```
$ nix develop --command cargo test -j 8 -p nova_menu --lib tests::load_screen
running 1 test
test tests::load_screen::load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 199 filtered out
```
Stable at 1/1 across two runs.

```
$ nix develop --command cargo test -j 8 -p nova_menu --lib tests::leave
```
**Not stably 9/9 as expected.** `tests/leave.rs` has exactly 4 `#[test]` fns
(`grep -c '#\[test\]' crates/nova_menu/src/tests/leave.rs` -> 4;
confirmed by name:
`leaving_a_saved_world_waits_for_the_leave_save_with_input_still_paused`,
`a_failed_leave_save_holds_the_clock_and_leaves_only_on_consent`,
`load_last_save_reopens_the_saved_world_without_writing`,
`window_close_with_a_saved_world_waits_for_the_leave_save`), so 9/9 does not
match this file's current content - unverified why the task expected 9;
possibly a stale count from before another lane's edit, or counting
`tests::load_screen` in too (1 + 4 = 5, still not 9).

Five consecutive runs, mixed `--test-threads` default and `=1`:

| run | result |
|---|---|
| 1 (right after my edit, before `cargo fmt`) | 4 passed, 0 failed |
| 2 (after `cargo fmt`, default threads) | 2 passed, 2 failed (`leaving_a_saved_world_...`, `window_close_...`) |
| 3 (`--test-threads=1`) | 3 passed, 1 failed (`window_close_...`) |
| 4 (`--test-threads=1`) | 3 passed, 1 failed (`leaving_a_saved_world_...`) |
| 5 (`--test-threads=1`) | 3 passed, 1 failed (`leaving_a_saved_world_...`) |
| 6 (`--test-threads=1`) | 4 passed, 0 failed |

Every failure is the same panic, in **`nova_world_base`'s** production save
system, not in anything I touched:
```
thread '...' panicked at crates/nova_world_base/src/save/session.rs:315:25:
Requested resource <Enable the debug feature to see the name> does not exist in the `World`.
<&mut nova_world_base::save::session::snapshot_world as ...>::call_mut
<bevy_ecs::system::exclusive_function_system::ExclusiveFunctionSystem<..., snapshot_world> as ...>::run
```
and which **test** fails is different each run (2, 3, 5, 6), including
single-threaded - which rules out a cross-test race and points to a
timing-sensitive resource lookup inside `snapshot_world`'s own real-IO save
pipeline (`WorldSaveTestPlugin`, `tests/leave.rs:44`, using a real
`bevy::tasks::IoTaskPool` and a real `TempDir`) under the load this shared
sandbox is under (a concurrent `nix develop` build on this same box hit
`Blocking waiting for file lock on build directory` mid-session). My edit
never touches `nova_world_base::save`, never adds a system that reads or
writes any resource `snapshot_world` reads, and is `resource_exists`-gated
on a resource (`WorldResumeRefused`) these four tests never insert - I see
no mechanism by which it could cause this. Flagging as unverified-but-likely
pre-existing/environmental rather than claiming it clean; the owner may want
to re-run this filter on a quiet host before trusting any count from this
box.

## 4. Where P-T11 can live (report only, not written)

Needed: the real `nova_world_base` restore pipeline (so
`WorldResumeProgress`/refusal/clock-release are the production systems, not
stand-ins), the Load/leave menu wiring (`refuse_resumed_world`, Retry), and
(once item 2's line exists) `nova_core`'s `LoadingScreenPlugin` to assert the
panel text - three crates' worth of plugins in one app.

Nearly all of the non-UI half of this already exists, purpose-built for
exactly this fixture shape:

- `nova_world_base::test_support::WorldSaveTestPlugin` and `arm_save_fixture`
  (`crates/nova_world_base/src/test_support.rs:33-49, 57-123`, behind the
  dev-only `test-support` feature) wire the **production**
  `restore_resumed_transients` + `save_systems()` in production order
  (test_support.rs:37-44) and arm a `WorldConfig` with one player ship and
  **no streaming** ("Nothing streams: the window stays empty, and the player
  is the whole world" - test_support.rs:7-8). That is precisely "a load that
  cannot fill its window": `desired_sectors(...)` is non-empty,
  `live_sectors(...)` never gains an entry, so `WorldResumeProgress` sits at
  `live: 0, desired: N` forever and the 240 s bound is what ends it - no
  generator or content pipeline needed to reach the refused case.
- `crates/nova_menu/src/tests/leave.rs:31-64`'s `saved_world()` already
  composes `support::app()` + `WorldSaveTestPlugin` + `arm_save_fixture` +
  a manual `Time<Real>`/`Time<Virtual>` stepper into a saved, on-disk world
  and drives it to its first save - the exact starting point P-T11 needs
  before it can Load or Retry that world back in and watch the refusal.
- `crates/nova_menu/src/tests/ambience.rs:521` already uses
  `TimeUpdateStrategy::ManualDuration` to step `Time<Real>` by named amounts,
  the mechanism P-T11 needs to cross `WORLD_RESUME_SECONDS_MAX` (240 s)
  without 240 real seconds of test runtime.

What is missing is only `nova_core::loading_screen::LoadingScreenPlugin`
(`crates/nova_core/src/lib.rs:50-51`, `pub mod loading_screen` / `pub struct
LoadingScreenPlugin`) for the panel-text assertion. Two places to put the
test, by who would have to reach across a crate boundary:

- **Option B (recommended): `crates/nova_menu/src/tests/` as a new sibling
  file to `leave.rs`/`load_screen.rs`.** Reuses `support::app()`,
  `saved_world()`-style fixture, and the `ManualDuration` stepper verbatim -
  near-zero new harness code. Cost: `nova_core` is not currently a dependency
  of `nova_menu` in any form (`crates/nova_menu/Cargo.toml` has no
  `nova_core` entry at all), so this needs one new
  `[dev-dependencies] nova_core = { path = "../nova_core" }` line. That is a
  legal Cargo dev-dependency cycle (nova_core's own `[dependencies]` already
  point at `nova_menu`, `crates/nova_core/Cargo.toml:23`; dev-dependencies
  are excluded from the normal build graph, so this does not create a real
  cycle for anything that depends on either crate) but it is a structural,
  cross-crate choice and a Cargo.toml edit - outside this task's file
  ownership, so not made here.
- **Option A: `crates/nova_core/tests/` (new file) or the inline `#[cfg(test)]
  mod tests` at `crates/nova_core/src/lib.rs:1095`.** No new dependency edge
  - `nova_core` already depends on both `nova_menu` and `nova_world_base`
  normally (`Cargo.toml:23,37`). Cost: `support::app()`
  (`crates/nova_menu/src/tests/support.rs:63-137`, ~75 lines) is `pub(crate)`
  to `nova_menu` and compiled only under nova_menu's own `#[cfg(test)]` - a
  `nova_core` test cannot import it and would have to rebuild an equivalent
  rig (states, both settings/training stores, `WorldsRoot`, `EntropyPlugin`,
  four `register_input_actions` calls, the three bare `Time<...>` resources,
  `ClockFreeze`) from scratch.

I recommend **Option B** for the LOC saved, flagging only that it needs the
owner's sign-off on the Cargo.toml edit and the dev-dependency direction
before anyone writes it.

## Round 2 (owner decision D-T8, `TRANSIENT-GATE.md:796-813`)

Scope unchanged: `crates/nova_menu/src/load_screen.rs`,
`crates/nova_menu/src/lib.rs` (registration only, none needed this round),
`crates/nova_core/src/loading_screen.rs`. No new type, function, or field
beyond `WorldRefusal::Unrestored`, which the owner had already added to
`crates/nova_world_base/src/save/mod.rs:182` ahead of this round.

### 1. Re-read on refusal (`load_screen.rs:154-216`)

`refuse_resumed_world` now takes `root: Res<WorldsRoot>` and
`packs: Res<LoadedSectionPacks>` (both already `pub(crate)`-visible to this
module, already imported for `on_load_screen`), and on the frame the panel
exists:

```rust
listings.0 = match root.0.as_deref() {
    Some(root) => list_worlds(root, &packs),
    None => Ok(Vec::new()),
};
if let Ok(rows) = &mut listings.0 {
    match rows.iter_mut().find(|row| row.folder.slug == refused.slug) {
        Some(row) => row.header = Err(WorldRefusal::Unrestored(refused.reason.clone())),
        None => error!(
            "refuse_resumed_world: {} is not in its own fresh listing ({})",
            refused.slug, refused.reason
        ),
    }
}
```

This is `on_load_screen`'s own read expression (`load_screen.rs:77-80`),
copied verbatim rather than factored into a shared helper - item 3 (Round 1's
numbering reused here) still caps this task at no new function, and the
expression is three lines.

**Why the re-read now finds the row that Round 1 flagged as missing.** Round
1's gap (SW-REFUSE.md:105-179) was: a `New Game` world, resumed for the first
time via Retry, was never listed by any earlier `on_load_screen` call, so
`WorldListings` had no row for it to patch. Re-reading from disk instead of
patching the stale in-memory list closes this at the root: by the time this
system's panel-exists branch runs, the world's lock was already dropped by
the teardown at the top of this same function
(`commands.remove_resource::<WorldSaveSession>()`, which drops the
`_lock: WorldLock` field held inside it - `crates/nova_world_base/src/save/session.rs:80-83` -
and `WorldLock`'s underlying `File`'s advisory lock releases on `Drop`, the
same mechanism `on_leave_without_saving`'s Menu arm relies on), so
`list_worlds` can see this folder as unlocked and read its header fresh. The
brand-new-world case from Round 1 is no longer a gap: its folder is on disk
(Create already wrote it, same as every `WorldFolder` `list_worlds`
enumerates), so the fresh read finds it like any other world.

**What is still possible, and handled without a gap this time:** a world
whose save folder was deleted or renamed on disk between being opened and
being refused. The fresh read would then succeed (`list_worlds`'s `Ok` arm,
since a missing *one* folder is just an absent row in a loop over
`read_dir`, not an error) but find no row for `refused.slug` - the
`None => error!(...)` arm. No panic; the player still lands in `MainMenu`,
just without a Load-row reason (there is no row to put it on). This is the
same "no mechanism, log it" shape the task asked for, not a new design.

**If the re-read itself errors** (the root existed and was readable before
but, e.g., permissions changed or the drive went away under the Load) -
`listings.0` becomes that `Err(WorldRefusal)` directly, same as
`on_load_screen`'s own failure mode, and `refresh_load_list` already renders
any `Err` in `WorldListings` as one note line
(`load_screen.rs:225-231`, unchanged). Nothing in this function hides or
overwrites that error - the `if let Ok(rows) = ...` guard simply does not
match, and the fn proceeds to open the panel on it. Rendered by
`refresh_load_list`'s own `Err(refusal) => ...` arm, `load_screen.rs:245-247`,
unchanged.

**`SelectedWorldSlug`: not cleared, by choice - the task asked for this call
to be made explicit.** `on_load_screen` clears it unconditionally
(`load_screen.rs:81`, `selected.0 = None`) because a Load-button click is a
fresh browse: any previous selection might not even be in the new read.
`refuse_resumed_world` does not, because the two situations differ in what
the "previous" selection usually is:

- In the common path (Load screen -> pick a row -> Load -> refused), the
  slug that was `SelectedWorldSlug` when the player left `MainMenu` is
  exactly `refused.slug` - the row they picked is the row that got refused.
  Leaving it set means `refresh_load_details` (`load_screen.rs:341-365`,
  unchanged, keys off `selected.0`) shows the same `Unrestored` reason in the
  details pane, not only as a line under the row in the list. Clearing it
  would make `refresh_load_list`'s own repair logic
  (`load_screen.rs:257-263`, unchanged) default-select the first row by slug
  order instead - which, for any world whose slug sorts before the refused
  one, would silently move the player's view away from the one row that just
  grew a reason.
- In the Retry path (`leave.rs`'s `LeaveTarget::Retry`), `SelectedWorldSlug`
  was never written for this session at all (Retry never routes through
  `on_load_world_row_select`) - it is either `None` (nothing seen this
  session) or a stale slug from an earlier, unrelated Load-screen visit. In
  the `None` case, `refresh_load_list`'s repair logic still default-selects
  the first row regardless of what this fn does, so clearing changes
  nothing. In the stale-slug case, that old slug was never the refused one,
  so whether it is cleared or kept does not change whether the refusal is
  visible - it only changes which *unrelated* row stays selected, which this
  task has no basis to decide either way.

Net: clearing it can only ever lose information (hide the just-written
reason behind a different row) and never gains any, so the fn leaves it
alone. Flagging this as the explicit answer to the task's question, not an
oversight.

### 2. `WorldRefusal::Unrestored` (`load_screen.rs:207`)

One-line change: `Err(WorldRefusal::Io(refused.reason.clone()))` ->
`Err(WorldRefusal::Unrestored(refused.reason.clone()))`. The variant and its
unprefixed `Display` arm (`crates/nova_world_base/src/save/mod.rs:201`,
`Self::Io(reason) | Self::Unrestored(reason) => f.write_str(reason)`) were
already present on disk before this round started - added by the owner per
D-T8, outside this task's file ownership, so not touched here beyond using
it. This resolves Round 1's "borrowed variant" flag (SW-REFUSE.md:181-194)
exactly as Round 1 recommended against doing without owner sign-off.

### 3. `LoadingResumeLineMarker` (`crates/nova_core/src/loading_screen.rs`)

Added the private marker (`loading_screen.rs:135-140`):

```rust
/// The world-resume progress line ("RESTORING SECTORS `<live>` / `<desired>`"),
/// shown while [`WorldResumeProgress`] exists and cleared to empty text
/// otherwise. Always present, on both screens, so no spawn path needs to know
/// whether a resume is in progress.
#[derive(Component)]
struct LoadingResumeLineMarker;
```

and one always-present child in `loading_panel`'s column
(`loading_screen.rs:244-247`), between the sweep track and the field-note
slot, styled like every other panel line via the existing `loading_text`
builder:

```rust
(
    loading_text("", 14.0, LOADING_TEXT, &font),
    LoadingResumeLineMarker,
),
```

Chose the existing `animate_loading_screen` system over a sibling, per the
task's "whichever matches the file's pattern": every other dynamic marker in
this file (`LoadingCursorMarker`, `LoadingDotsMarker`, `LoadingSweepMarker`)
is driven from inside that one system, already chained before
`dismiss_scenario_load_screen`, and it already takes `Res<Time<Real>>` plus
one query per marker - adding a fifth param and a fifth per-frame block
matches the existing shape instead of introducing a second ungated `Update`
system for one more line. Added
`#[cfg(not(target_arch = "wasm32"))] world_resume: Option<Res<WorldResumeProgress>>`
as a new parameter (same cfg Round 1 used on `dismiss_scenario_load_screen`'s
own copy of this parameter, `loading_screen.rs:441`) and, at the end of the
function body (`loading_screen.rs:518-530`):

```rust
#[cfg(not(target_arch = "wasm32"))]
let resume_text = world_resume.map_or_else(String::new, |progress| {
    format!("RESTORING SECTORS {} / {}", progress.live, progress.desired)
});
#[cfg(target_arch = "wasm32")]
let resume_text = String::new();
for mut text in &mut q_resume_line {
    if text.0 != resume_text {
        text.0 = resume_text.clone();
    }
}
```

The `text.0 != resume_text` guard before the write matches the task's "write
the Text only when it changes" and mirrors `q_dots`'s own existing guard
three lines above it in the same function.

**Both screens get the line, not just the scenario one - this was a choice,
not an oversight.** `loading_panel` is the one builder both
`spawn_loading_screen` (boot) and `spawn_scenario_load_screen` (scenario
swap) call (`loading_screen.rs` module docs, confirmed unchanged at
`loading_screen.rs:186-240`'s surrounding comment, "One builder rather than
three, so the screens cannot drift apart"). Splitting the new child out to
only one caller would mean either a second builder (a new function - ruled
out) or an `Option`-gated child inside the one builder keyed on which screen
is calling it (a new parameter to `loading_panel` just to suppress a line
that is already a correct no-op everywhere `WorldResumeProgress` cannot
exist - the boot screen only ever runs before any world is opened, so
`world_resume` reads `None` there every frame and the line stays empty text,
indistinguishable from not being spawned at all). Kept the one builder, one
line, no branch.

**Query conflict check.** `q_resume_line: Query<&mut Text, With<LoadingResumeLineMarker>>`
sits alongside `q_dots: Query<&mut Text, With<LoadingDotsMarker>>` in the
same system - two mutable queries over the same component type, `Text`,
distinguished only by their `With` filter. This compiles (see Checks below)
because Bevy's system-param conflict check proves the two queries'
archetype sets are disjoint from the `With` filters themselves (no entity
spawned by `loading_panel` carries both marker components), the same
pre-existing relationship between `q_cursor`/`q_dots`/`q_sweep` already
relied on for three different component types - this is one more pair of
the same shape, not a new pattern.

### 4. Checks

```
$ nix develop --command cargo check -j 8 -p nova_menu -p nova_core --tests
    Checking nova_menu v0.15.0 (...)
    Checking nova_console v0.15.0 (...)
    Checking nova_core v0.15.0 (...)
    Finished `dev` profile [optimized + debuginfo] target(s) in 20.78s
```
Clean, both crates, before and after `cargo fmt -p nova_menu` /
`cargo fmt -p nova_core` (re-ran `check` after formatting - still clean, 0.72s
incremental). Only the pre-existing workspace-wide `proc-macro-error2`
future-incompat notice, unrelated.

```
$ nix develop --command cargo test -j 8 -p nova_menu --lib tests::load_screen
running 1 test
test tests::load_screen::load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 199 filtered out
```
1/1, run twice (before and after `cargo fmt`), same result both times.

```
$ nix develop --command cargo test -j 8 -p nova_menu --lib tests::leave
running 4 tests
test tests::leave::load_last_save_reopens_the_saved_world_without_writing ... ok
test tests::leave::a_failed_leave_save_holds_the_clock_and_leaves_only_on_consent ... ok
test tests::leave::leaving_a_saved_world_waits_for_the_leave_save_with_input_still_paused ... ok
test tests::leave::window_close_with_a_saved_world_waits_for_the_leave_save ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 196 filtered out
```
**4/4, run four times total** (three before `cargo fmt`, default thread
pool, plus one more after `cargo fmt`) - every run this round passed clean,
no flake reproduced. `tests/leave.rs` still has exactly 4 `#[test]` fns
(Round 1 already confirmed this with `grep -c '#\[test\]'`; unchanged this
round), so "4/4" is this file's full count, matching the task's "expect
4/4" - Round 1's 9/9 expectation mismatch was pre-existing and is not
something this round's diff could explain either way.

Round 1 traced its flake to a panic inside **`nova_world_base`'s**
`snapshot_world` ("Requested resource ... does not exist in the World"),
explicitly not touched by Round 1's own diff. This round's diff touches
`nova_world_base` even less (only reads a `pub` resource type and a `pub`
field already read by Round 1's `dismiss_scenario_load_screen` gate) and
does not reproduce the flake in four consecutive runs. Consistent with
Round 1's "unverified-but-likely pre-existing/environmental" label - not
claiming it fixed, since four green runs cannot prove a load-dependent flake
is gone, only that it did not reproduce this time.
