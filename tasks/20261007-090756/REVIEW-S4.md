# Review: Slice 4 (menu, leave, window close)

Scope: crates/nova_menu/src/{leave.rs, pause.rs, world_setup.rs, load_screen.rs,
save_status.rs, menu_ui.rs, lib.rs}, crates/nova_core/src/lib.rs,
crates/nova_world_base/src/save/session.rs, and the leave/load_screen/world_setup
tests. Read-only; no code was run or edited.

## Findings

### 1. MAJOR — "no stale Saved" correctness depends on an unstated, implicit system order
**Claim:** The invariant that `drive_pending_leave` never finishes (or keeps
waiting) on a status left over from the *previous* frame depends on
`request_world_save` -> `poll_world_writer` -> `snapshot_world` always having
already run earlier in the *same* frame. Nothing in the code enforces that
ordering between the two plugins; it holds only because `NovaWorldBasePlugin`
is added to the app before `NovaMenuPlugin`, and Bevy's scheduler uses
insertion order as the tie-break for systems that conflict on the same
resource but have no explicit `.after`/`.before`.

**Evidence:**
- `crates/nova_world_base/src/lib.rs:184-193` — `save::save_systems().after(NovaWorldSystems::Retire)`, no further `SystemSet` exported to order against.
- `crates/nova_world_base/src/save/mod.rs:33-36` — only `pub(crate) use session::{restore_resumed_world, save_systems};` is exported; no public ordering label.
- `crates/nova_menu/src/lib.rs` (diff, leave registration block) — `(leave::drive_pending_leave.run_if(...), leave::sync_leave_overlay).chain().run_if(in_state(GameStates::Playing))` has no `.after(...)` tying it to the save chain above.
- `crates/nova_world_base/src/save/session.rs:151-159` (`request_leave`) relies on `poll_world_writer`/`snapshot_world` having run before `drive_pending_leave` reads `status()` the same frame a crossing write finishes and the leave's own write starts (status must become `Writing` again before `drive_pending_leave` can read it).
- `crates/nova_core/src/lib.rs:421-430,469` confirms `NovaWorldBasePlugin` is added before `NovaMenuPlugin` — this is the sole reason the order is correct today.

**Consequence if this order ever flips** (e.g. a future refactor adds an
explicit `.after` elsewhere that reorders the ambiguity, or someone reorders
plugin registration): `drive_pending_leave` could read `WorldSaveStatus::Saved`
left over from the *previous* frame's crossing save (or a not-yet-updated
`is_idle()`), finish the leave, and remove the session/drop the lock before
the leave's own write has even started — exactly the "leave finishes on a
stale Saved" failure this review was asked to look for.

**Severity:** MAJOR. Not currently observed to fail (plugin order is correct
today and the tests in `tests/leave.rs` exercise the real scheduler), but the
correctness guarantee is implicit and unverified by any ordering assertion.
The project already has precedent for pinning such orders with
`ambiguity_detection: LogLevel::Error` (`crates/nova_ship/src/sections/turret_section/mod.rs:463-481`); nothing analogous exists here.
**Unverified:** did not run the schedule or tests; this is a static-reading
conclusion about Bevy's ambiguity tie-break, not an observed failure.

### 2. MINOR — wasm32 cfg gap: nothing answers `WindowCloseRequested` on web
**Claim:** `window_plugin()` sets `close_when_requested: false` unconditionally
(not cfg-gated), but the only system that ever reads `WindowCloseRequested`
(`leave::on_window_close_requested`) is registered only under
`#[cfg(not(target_arch = "wasm32"))]`.

**Evidence:**
- `crates/nova_core/src/lib.rs:744` — `close_when_requested: false` with no `#[cfg]`.
- `crates/nova_menu/src/lib.rs` (diff) — the whole block registering `on_window_close_requested`, `add_message::<WindowCloseRequested>()`, and the `OnExit(Playing)` `PendingLeave` cleanup is inside `#[cfg(not(target_arch = "wasm32"))]`.
- The no-menu fallback that restores default behavior (`crates/nova_core/src/lib.rs:467-476`) only fires when `!has_menu`; a wasm build with the menu has `has_menu == true`, so it does not get the fallback either.

**Consequence:** on a wasm build with the menu, if `WindowCloseRequested` is
ever emitted, nothing answers it: the window is never despawned, `AppExit` is
never written, and the session/lock are never released. In practice this is
likely inert (bevy_winit's wasm canvas backend has no native close button to
emit this from), but that is not verified in this review, and nothing in the
diff documents the assumption.
**Unverified:** could not confirm whether bevy_winit's wasm target ever emits `WindowCloseRequested`.

### 3. MINOR — dead branch: `on_leave_without_saving`'s `LeaveTarget::Retry` arm is unreachable
**Claim:** `on_leave_without_saving` (`crates/nova_menu/src/leave.rs:307-329`)
matches `LeaveTarget::Menu | LeaveTarget::Retry` together, but the overlay that
owns the only buttons wired to this handler never renders them while the
pending leave's target is `Retry`.

**Evidence:**
- `crates/nova_menu/src/leave.rs:190-195` (`sync_leave_overlay`) — the `(LeaveTarget::Retry, _)` match arm always returns `failed = false`.
- `crates/nova_menu/src/leave.rs:279-290` — "Try again"/"Leave without saving" buttons (the only callers of `on_leave_try_again`/`on_leave_without_saving`) are spawned only `if failed`.
- `crates/nova_menu/src/leave.rs:324` — the `LeaveTarget::Retry` arm in `on_leave_without_saving`'s match can therefore never execute.

**Consequence:** none today (Retry's own reopen path in `drive_pending_leave`
handles its own completion independently), but it is dead code that
contradicts the "no dual paths/leftovers" standard — worth removing the
`Retry` arm or documenting why it is kept as a safety net.

### 4. Informational / low confidence — `drive_pending_leave`'s session-vanished branch silently drops the leave
**Claim:** `crates/nova_menu/src/leave.rs:98-101` — if `PendingLeave` exists
but `WorldSaveSession` does not, the system just removes `PendingLeave` and
returns: no state transition, no `AppExit`, no feedback.

**Evidence:** grep of the whole tree shows `WorldSaveSession` is removed only
in three places (`leave.rs:112`, the Retry closure at `leave.rs:137`,
`leave.rs:318`), and all three remove `PendingLeave` in the same step. So this
branch looks unreachable with the current callers.

**Consequence if ever reached:** the player is not stranded — once
`PendingLeave` is gone, `reconcile_pause_overlay`'s `taken` flag
(`crates/nova_menu/src/pause.rs` diff) stops treating the leave as "taken" and
the ordinary pause overlay reappears (since `PauseStates` is still `Paused`)
— but the player's chosen leave/exit is silently dropped with no error shown.
**Unverified / low severity:** no current code path reaches this branch; flagging as a gap in case a future caller (e.g. a death/disarm path) ever removes `WorldSaveSession` independently of `leave.rs`.

## Verified as correct (explicitly checked against the review's focus list)
- `begin_leave`/`drive_pending_leave`/`on_leave_without_saving` never write a
  double `AppExit`; a second `WindowCloseRequested` while `PendingLeave`
  exists is a no-op (`leave.rs:344`).
- The Retry reopen closure drops the session (and its lock) with an explicit
  `drop(session)` before calling `open_world` on the same slug
  (`leave.rs:137-142`), and re-checks `is_writing` when the command applies.
- `resume_world` removes `WorldConfig` first (`session.rs:215`), matching the
  stated contract.
- `FreezeOwner::PauseMenu` hold/release is balanced across the leave wait,
  the failure path, and Try again/Leave without saving — traced through
  `pause.rs`'s `OnEnter`/`OnExit(PauseStates::Paused)` hooks and `leave.rs`'s
  per-frame hold/release; no leak found.
- Window close behavior for the menu, a non-open-world scenario (no
  `WorldSaveSession`), the editor, and a no-menu app all resolve to the
  original behavior (`leave.rs:335-356`, `nova_core/src/lib.rs:467-476`), as
  the matrix requires.
- `create_world`'s `NameTaken` refusal in `on_create_world` starts nothing and
  leaves the existing folder untouched; the taken-name field error is not
  auto-cleared without a retyped value, which matches the design's statement
  that the taken-name check "cannot go stale" and is Create's job, not
  `read_world_setup`'s.
- `on_load_world`'s per-row refusal mutates `WorldListings` through `ResMut`,
  correctly re-triggering both `refresh_load_list` and `refresh_load_details`
  via `resource_changed`.

## Skipped checks
- Did not run `cargo check`/`cargo test`/Clippy (per instructions; the main
  worker owns builds).
- Did not run the game or any GPU/rendered measurement.
- Did not review `nova_assets::storage::worlds_root()`,
  `create_world`/`open_world`/`write_world`/`list_worlds` internals (Slice 2,
  out of this slice's file list) beyond their call sites in the reviewed files.
- Did not review death/disarm interaction with a mid-leave `WorldSaveSession`
  (Slice 3 territory) beyond confirming no current code removes the session
  outside `leave.rs`.
- Did not verify bevy_winit's actual wasm `WindowCloseRequested` emission
  behavior (finding 2).

## Disposition (main worker)

1. Rejected with evidence. `begin_leave` calls `request_leave`, which sets
   `wanted = Some(Leave)` at once (`save/session.rs:152-159`), and
   `is_idle` needs `wanted.is_none()` (`save/session.rs:179-181`). The wanted
   leave save is consumed only when its own writer starts, so a Saved left by
   an earlier write can never meet an idle session. The system order between
   the plugins does not change this. No set added.
2. Fixed. `nova_core` `window_plugin` sets
   `close_when_requested: cfg!(target_arch = "wasm32")`, and the no-menu
   `close_when_requested` system is native-only. The web build keeps bevy's
   own answer.
3. Fixed, fail loud. `on_leave_without_saving` matches `Menu` alone, and
   `Retry` is `unreachable!` with the reason.
4. Fixed, fail loud. `drive_pending_leave` panics with "a pending leave
   outlived its world session" instead of dropping the leave.

Verified: 42 focused nova_menu tests pass; wasm32 check of nova_core and
nova_menu clean.
