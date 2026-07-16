# Review: Lock-on acquisition dwell (radar hold-to-lock)

- TASK: 20260708-165703
- BRANCH: feat/lock-dwell-mechanic

## Round 1

- VERDICT: APPROVE

Reviewed the diff against master plus an independent out-of-context correctness
pass (shared implementer/reviewer session, so the load-bearing logic was
re-derived from the full `update_radar_search`, not just the diff). Verified:
the dwell is the SOLE chokepoint for new-candidate slot writes (the `None` path
and the observers never commit; `update_contacts_and_locks` only holds/clears);
cancel + keep-last are clean (a candidate change/None resets the dwell and never
touches the committed slot); the acquire-once / retarget-on-change cue contracts
survive the move to dwell-completion; and all 5 new tests genuinely FAIL if the
gate is reverted to instant commit (the pure curve test defends the formula, not
the gate, by design). `cargo check -p nova_gameplay` + fmt clean; targeting 47
passed, lock_crosshairs 4 passed. Docs synced (targeting-radar, getting-started,
CHANGELOG). No BLOCKER/MAJOR.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/input/targeting.rs:900 -
  `raw.clamp(settings.lock_dwell_min, settings.lock_dwell_max)` panics per
  Rust's `f32::clamp` contract if a user configures `lock_dwell_min >
  lock_dwell_max` in the inspector. Internal playtest knobs with sane defaults,
  but a one-line harden removes the latent panic:
  `raw.clamp(min, min.max(max))`.
  - Response: fixed in the follow-up commit - `lock_dwell_secs` now clamps to
    `[min, min.max(max)]` so a misordered pair can never panic.
- [ ] R1.2 (NIT) crates/nova_gameplay/src/input/targeting.rs:793-796 - the
  `map_or(0.0, ...)` distance fallback is unreachable (the picked candidate
  always originates from `candidates`). Fine as defensive code; left as-is.
  - Response: intentional defensive fallback, kept.
