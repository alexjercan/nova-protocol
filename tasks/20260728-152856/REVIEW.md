# REVIEW: ship view label table

Out-of-context reviewer, round 1, `feat/ship-view-label-table` vs `master`.

## Round 1

- NIT - sort-by-code is lexical (`HULL-1`, `HULL-10`, `HULL-2`), matching the
  prior lexical name sort; not requested, no regression. No action.
- NIT - the TASK.md Design note said to `use super::nova_os_ship::SectionCode`,
  but `SectionCode` already resolves via the prelude glob, so no import was
  added. The Design note is stale; the code is correct. Left as append-only
  history (noted in RETRO instead of rewriting the record).

Verified: padding math handles the header/fallback being the widest cell;
`{x:<w$}` width syntax correct; both query tuples (snapshot fn param + caller
system) are byte-for-byte identical and the sole caller matches; the fallback
is panic-free; the live test genuinely proves ECS->scrollback threading (a
`SectionCode("THR-1")` asserted in the row - would read `THRUSTER` via fallback
if unthreaded) plus the codeless-turret fallback; empty-ship branch and row
colour-coding unregressed; `cargo check -p nova_gameplay` clean.

- VERDICT: APPROVE
