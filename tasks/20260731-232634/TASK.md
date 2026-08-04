# Holo trajectory ribbon doc may be stale: arrival solve gravity-awareness

- PRIORITY: 0
- TAGS: backlog, hud, docs
- ACTIVITY: -
- GATES: -
- RESOLUTION: WONTDO

## Goal

`crates/nova_gameplay/src/hud/holo_instruments.rs` module doc says the ribbon
is "deliberately the straight-line plan the computer actually flies today;
when the arrival solve becomes gravity-aware a curved prediction can replace
it".

The gravity-aware arrival task (20260710-193500) is CLOSED, so that
precondition may already be met and nobody would notice - the pointer was
dropped in the KISS comment pass (20260731-170335) precisely because a
provenance ID is not a tracking mechanism.

Requires: check whether the arrival solve is now gravity-aware. If it is,
either update the ribbon to a curved prediction or rewrite the doc to say why
the straight-line ribbon is still the honest instrument. If it is not, leave
the doc alone.

## Done Means

- manual: the doc claim matches what the autopilot actually solves today.


## Dropped

- REASON: old
