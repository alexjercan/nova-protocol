# Review: RCS HUD indication (active palette)

- TASK: 20260718-122923
- BRANCH: feat/rcs-hud

## Round 1

- VERDICT: APPROVE

Reviewed commit 2da30475 vs master (e1c254b7). The diff delivers the Goal's
REQUIRED item - a distinct RCS-active palette on the velocity sphere - by
mirroring the proven `sync_engaged_palette` autopilot-presence switch. Small,
idiomatic, fully contained.

Independently verified (shared-session blind-spot guard):
- No other callers of `desired_velocity_palette` or `sync_engaged_palette` exist
  outside velocity.rs (grep across crates/), so the arity change is fully
  contained - nothing else breaks.
- Precedence is `rcs_active > engaged > manual`, matching the doc and covered by
  the pure test for all four input combinations.
- `velocity_palette_follows_rcs_active` is non-vacuous: it would fail if the
  `rcs_active` branch were deleted (RcsActive would map to default, not
  RCS_ACTIVE), and it pins the deliberate contract that an engaged autopilot
  WITHOUT RcsActive still reads ENGAGED - protecting the later autopilot-drives-
  RcsIntent behavior.
- The existing `velocity_palette_follows_the_autopilot` test is unchanged and
  passed in the 8-test `hud::velocity` run, confirming the change did not weaken
  the autopilot path.

The cap-ring split is legitimate, not a shortfall: the Goal marks the ring
"Optionally render", the sphere is genuinely fixed-radius (speed is a shader
magnitude, not the physical radius) so "a ring at the cap" is a real visual-
design question, and it needs a playtest this headless flow cannot do. It is
seeded as tatr 20260718-144939 with that rationale. Delivering the required
palette and deferring the optional ring is correct scope management.

No BLOCKER/MAJOR. Nits only, left to discretion:

- [x] R1.1 (NIT) crates/nova_gameplay/src/hud/velocity.rs:371
  (`sync_engaged_palette`) - the name now undersells the function: it picks the
  RCS palette too, not just "engaged". `sync_velocity_palette` would read truer.
  Left un-renamed to keep the diff minimal (rename touches the registration at
  velocity.rs:172-181 and the tests); fine to leave or rename in passing.
  - Response: Renamed `sync_engaged_palette` -> `sync_velocity_palette` (all 9
    occurrences in velocity.rs: def, registration, and 7 test call sites).
- [ ] R1.2 (NIT) crates/nova_gameplay/src/hud/velocity.rs:78 (RCS_ACTIVE colors)
  - the violet values are an unverified "starting point for the by-eye pass"
  (same caveat the ENGAGED palette carries). Confirm the exact hue in a playtest
  alongside the deferred cap ring; no code concern.
  - Response: Left as-is (NIT) - a documented by-eye/playtest call, same as the
    ENGAGED palette. Routed to the cap-ring follow-up (20260718-144939) which
    already requires a playtest.

### Round 1 resolution

- VERDICT: APPROVE

R1.1 renamed; R1.2 is a documented playtest call. No open BLOCKER/MAJOR. Clean,
contained diff mirroring the proven autopilot-palette pattern.
