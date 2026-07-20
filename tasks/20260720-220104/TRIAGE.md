# OPEN-task triage (2026-07-21, task 20260720-220104)

24 OPEN tasks (excluding this task and the goal umbrella). Every one carries an
intentional scheduling tag and a priority - none are untagged or orphaned. No
unilateral closes were made: close/defer of product/feature work is the user's
call. This is the triage assessment + a shortlist to surface.

## By scheduling state

- **Scheduled for v0.8.0 (9)**: active release work, priorities 20-50.
  20260718-152313 (base campaign polish, p50), 20260718-152320 (Ledger mod
  polish, p48), 20260719-002512-seeded 20260721-000229 (sccache, p44),
  20260721-000249 (crate-scoped tests fix, p42), 20260525-133030 (doc
  nova_gameplay, p40), 20260716-174729 (gauntlet run timer, p36), 20260525-133032
  (inline plugin docs, p36), 20260718-004856 (broadside hitch, p24), 20260715-231500
  (HUD/radar screenshot callouts, p20). KEEP - all current release scope.
- **Backlog / deferred (15)**: priority 0, tag `backlog` - explicitly parked.
  Real-timer thumbnails, piccolo VM, web fonts, screen-indicator->bcs, keybind
  hints, gamepad/mobile, nova_probe golden compare, ship-prototype kind,
  bevy_capture eval, devlog thumbs, in-editor scenario builder, hull-integrity
  chip, doc bcs API, CI clippy -D warnings decision, alt-fire modes. KEEP as
  backlog - deferral IS their intentional disposition.

## Surfaced for a user close/defer decision

These are valid but have sat idle; worth a keep-or-close ruling:

- **20260525-133030 / -133032 (v0.8.0 docs, ~8 weeks idle)**: "document
  nova_gameplay public API" and "inline doc comments on all public plugin
  structs". Still-valid work never picked up; decide whether they stay in v0.8.0
  or drop to backlog.
- **20260525-133031 (backlog): "Write documentation for bevy_common_systems
  public API"** - this is a NOVA task about documenting a DIFFERENT repo (bcs).
  bcs has its own docs task; this one likely belongs there or is redundant.
  Candidate to close.

## Supersession checks (no action taken)

- 20260709-164608 (promote screen-indicator to bcs): NOT yet in bcs src - still
  valid, not superseded.
- 20260714-134115 (ship-prototype kind, "folds 113414"): "folds" is a plan note
  (it absorbs the scope of an earlier id), not a duplicate - keep.

## Recommendation

Backlog is healthy and needs no structural change. The only concrete cleanup is
the 3 May-25 doc tasks (esp. 133031, the cross-repo bcs one) - surfaced above for
a user ruling rather than closed unilaterally.
