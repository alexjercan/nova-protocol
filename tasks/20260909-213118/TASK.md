# Gameplay feedback ledger: play, record, fix or file

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.14.0, playtest, balance, polish

## Goal

One place for gameplay feedback across the cycle, and the rule for what
happens to each item. Owner (2026-09-09): this release is "more code review
rather than tasks in particular and also a lot of gameplay feedback ... fix
any obvious bugs" and balance.

## The ledger

`FEEDBACK.md` in this task, one row per observation:

| date | who | where (scenario, hull) | what was seen | kind (bug / feel / balance / clarity) | disposition |

Sources: the owner's own play sessions, the three-person pitch test from the
store presence task (`20260909-212954`), the gamepad playthrough
(`20260714-001140`), and every review lane's feel findings.

## Disposition rules

- A bug with a reproducible trigger gets a `bug_` or `system_` range in
  `examples/systems/` that fails before the fix and passes after, then the
  fix, one commit. The range is the proof; the row links the commit.
- A feel or clarity item that is cheap (a constant, a line of copy, a
  colour, a gap in the pacing table) is fixed in-session and recorded.
- A balance item is measured before it is tuned: name the number, the hull,
  the scenario, and the target feel. Tune one knob per commit. The verified
  statistics wiki task (`20260907-173050`, backlog) is where the tuned
  figures would be published if it is scheduled.
- Anything larger becomes its own task on the v0.14.0 board only by owner
  decision; otherwise it goes to the backlog at priority 0 with the row as
  its body.

## Seeds

Rows already known on 2026-09-09:

- The velocity and gravity spheres clip a big hull: `20260909-212917`.
- On a gamepad, L2 raises weapons AND fires the torpedo tubes (review
  `20260905-231735`, group I): folded into `20260714-001140`.
- The wiki calls 55.2 m "its own 55.2 m hull" for a hull 85 m long
  (`web/src/wiki/flight-autopilot.md`, review `20260908-004345`): clarity.

## Done when

Every row has a disposition and the release meta task can point at this
file as the record of what the playtests found.
