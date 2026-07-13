# Review: Flight feel polish: rotation slew, handling stats, camera weight, retune

- TASK: 20260709-095043
- BRANCH: feature/flight-feel-retune

## Round 1

- VERDICT: REQUEST_CHANGES

Independent fresh-eyes pass over the diff, the bcs PD/chase sources and avian's
force API. Verified clean: the bang-bang math (sqrt(pi*alpha)/2 is exactly the
average rate of a torque-limited 180); largest-principal-inertia is the right
conservative axis; per-frame offset writes compose with (not fight) bcs's
dt-corrected smoothing lerp; the two offset writers are chained and skip
coherently; the torpedo's own PD config is fully isolated from
FlightSettings/hull_turn_rate; 11_com_range's assertions are
spin-magnitude-independent and unperturbed; the three new tests are
deterministic; conventions and commit hygiene hold.

- [x] R1.1 (MAJOR) camera_controller.rs - CAMERA_SMOOTHING (and focus_offset)
  are applied only inside the mode-switch system, gated on `mode.is_changed()`.
  Player death removes `SpaceshipCameraController`/`ChaseCamera`; respawn
  re-inserts `ChaseCamera::default()` (smoothing 0.0) and the consumed
  change-flag means no re-application until the player toggles modes: the
  camera-weight feature works for the first life only. Suggested: the
  per-frame push system owns the whole rig (offset, focus_offset, smoothing).
  - Response: fixed - `update_camera_rig` (renamed from update_camera_burn_push)
    owns offset, focus_offset and smoothing every frame; the mode switch only
    juggles input markers. The burn-push test now asserts the full rig lands
    on a factory-fresh ChaseCamera with no mode change (the respawn case).
- [x] R1.2 (MAJOR) flight.rs:462 + docs - at torque 10 the PD can no longer
  out-torque off-center engines (break-even lever arm ~T/64 units per unit
  thruster magnitude), but the comment still says "the PD fights it". Editor
  builds with lateral engines and damage-shifted-COM hulls will pull or
  pinwheel under burn. Suggested: fix the comment, quantify the regime in the
  retune doc, cover with an off-axis physics test, and file thrust-balancing
  as a follow-up.
  - Response: fixed/filed - the stale 'PD fights it' comment is rewritten with
    the real break-even (~max_torque/64 lever units), the regime is tabled in
    the retune doc, `off_center_burn_pulls_but_a_centered_drive_is_held`
    pins both cases (centered held < 0.15 rad, 2-unit offset pulls > 0.4
    rad), and thrust balancing is filed as 20260709-155920.
- [x] R1.3 (MAJOR) sections.rs / docs / TASK.md - the tuning rationale ("stock
  3-section ship, I ~2.3, flips in ~1.7s") describes the flight-test rig, not
  shipped content: the asteroid_field flagship is 5 sections at I ~10.8, so
  torque 10 HALVED its turn rate (90 -> ~44 deg/s) unacknowledged;
  10_gameplay's ship lands at ~60. Suggested: per-shipped-ship numbers in the
  docs and a max_torque default that is deliberate about the flagship.
  - Response: fixed - max_torque retuned 10 -> 40: the asteroid_field flagship
    keeps its familiar ~88 deg/s command rate (vs the old fixed 90) while a
    remnant pins the 240 ceiling; per-ship table added to the docs;
    sections.rs comment, TASK.md, and CHANGELOG corrected.
- [x] R1.4 (MINOR) sections.rs/flight.rs/docs - the quoted flip times are the
  un-scaled bang-bang optima, not delivered behavior (command slews at 0.9x,
  and the PD tracks a ramp with ~0.5*w rad steady-state lag, adding ~30%);
  and "at 100 ... flipped a 180 in ~0.5s" is wrong - the old fixed 90 deg/s
  slew bound at ~2s. Label numbers as optima or use measured values.
  - Response: fixed - all quoted times are labeled bang-bang optima with the
    ~25-30% PD-ramp-lag caveat, and the false 'flipped in ~0.5s at 100'
    claim is replaced by the true old behavior (slew-bound at 90 deg/s).
- [x] R1.5 (MINOR) flight.rs:319 - `f32::clamp` panics when min > max, and
  both bounds are inspector-editable on the reflected FlightSettings; a live
  edit can panic every frame. Defensive ordering of the bounds.
  - Response: fixed - bounds ordered defensively before the clamp (hi =
    max(hi, lo)); a live inspector edit can no longer panic.
- [x] R1.6 (MINOR) input/player.rs - with zero live computers the command
  still slews at the floor rate toward the camera; a disabled-in-place
  controller keeps its (drifting) command and would snap the hull on
  re-activation. Freezing the command (early return) is the cleaner
  semantic. Related pre-existing hole, not this PR:
  `sync_controller_section_forces` has no inactive filter, so a
  disabled-but-present controller still torques toward its frozen input.
  - Response: fixed/filed - the player command now FREEZES when no live computer
    exists (early return; nothing consumes a dead helm, and drift would snap
    the hull on re-activation). The pre-existing disabled-controller-still-
    torques hole is filed as 20260709-155922.
- [x] R1.7 (MINOR) flight.rs tests - the physics-test rigs keep max_torque
  100 (derived rate pins at the 240 ceiling - a different regime from
  shipped ships), and the scratch test's comment claims "real config values
  from nova_assets/sections.rs", which is now false. Align the scratch test
  with the shipped value; keep or justify the generic rig's 100.
  - Response: fixed - the scratch test runs the shipped 40.0 (its 'real config
    values' comment is true again); the generic rig keeps 100 with a comment
    justifying it (outcome tests deliberately pinned at the rate ceiling; the
    shipped regime is covered by the scratch and off-axis tests).
- [x] R1.8 (MINOR) input/ai.rs - AI ships inherit the lower torque with an
  unslewed, every-frame rotation command - the exact saturation regime the
  player path was fixed for (plus the pre-existing delta-into-absolute-input
  bug at ai.rs:92). No shipped scenario spawns an AI controller today, so
  exposure is editor-only. Suggested: follow-up task for the AI adopting
  slew_rotation/hull_turn_rate; a blast-radius line in the docs.
  - Response: filed as 20260709-155921 (AI adopts slew_rotation/hull_turn_rate,
    plus the pre-existing delta-into-absolute-input bug), and the docs'
    blast-radius section notes the editor-only exposure.
- [x] R1.9 (NIT) player.rs/flight.rs - PD outputs stack additively across
  computers, so multi-computer authority is the SUM of caps; "strongest live
  computer" (max) is safely conservative but the wording deserves a comment.
  - Response: fixed - both q_computer sites carry the 'PD outputs stack; max is
    deliberately conservative' comment.
- [x] R1.10 (NIT) camera_controller.rs - the mode-switch offset write is dead
  (the chained push system overwrites it the same frame); the burn push in
  FreeLook/Turret is a view-axis dolly rather than a hull-frame lean (doc
  wording); heat = max over thrusters means one lit gnat engine gives full
  push (authority-weighted would track acceleration better - playtest knob).
  - Response: fixed - the dead mode-switch write is gone (the rig system owns the
    fields), the doc describes FreeLook/Turret as a dolly-out, and heat=max
    got a rationale comment (playtest owns authority-weighting).
- [x] R1.11 (NIT) player.rs command-lag test - passes for any rate in a wide
  band; asserting the actual expected per-frame step would pin the
  derivation. Also `mod command_lag_tests` beside `mod tests` is odd.
  - Response: fixed - the lag test asserts one frame advances exactly one
    hull_turn_rate step (15% tolerance), pinning the derivation; the module
    keeps its own harness with a comment on why it sits beside `tests`.
- [x] R1.12 (NIT) tasks/20260709-121842/RETRO.md -
  the "[ ] 20260709-095043 ... est_turn_rate_deg" action item is now
  deliverable; annotate it.
  - Response: fixed - the multi-thruster retro action item is ticked with a
    delivery note.

## Round 2

- VERDICT: APPROVE

Verified every fix on the updated branch: the per-frame rig system applies
offset/focus/smoothing to a factory-fresh ChaseCamera (test covers the respawn
case) and the mode switch no longer touches ChaseCamera at all; the retuned 40
keeps the flagship at ~88 deg/s with the per-ship table in the docs and all
times labeled as optima; the off-axis test pins the torque-cut regime exactly
as computed in round 1 (centered < 0.15 rad, offset > 0.4 rad); the clamp is
panic-proof; the no-computer path freezes; the strengthened lag test pins the
derived step. Follow-ups 20260709-155920/155921/155922 cover thrust balancing,
the AI path, and the disabled-controller hole. 111 unit tests, both headless
smokes, fmt and workspace check green. No new findings.
