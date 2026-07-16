# Retro: More menu backdrops

- TASK: 20260716-180352
- BRANCH: content/menu-backdrop-pack (landed 29493272)
- REVIEW ROUNDS: 1

## What went well

- The whole 155816-155849 tooling chain compounded: builders -> one
  gen_content command -> parity bundle-set guard forcing the wiring ->
  fixture-based menu tests unaffected by real content counts. A content
  task touched zero test logic.
- The eyeball step was cheap because example 14 already captures the
  menu: 8 automated runs produced screenshots of every rotation member,
  and 6 example-12 boots proved the menu survives each one.
- Constraint-driven design (only proven primitives; menu_ambience's
  documented safety envelope) meant zero physics surprises.

## What went wrong

- A cleanup `pkill -f 'Xvfb :99'` matched the invoking shell's OWN
  command line and killed the whole chain (exit 144) - and blind
  pattern-kills are doubly dangerous here because one running Xvfb may
  be the user's real display. Cost: one re-run.

## What to improve next time

- Never pkill by a pattern that appears in your own command line; if a
  helper process must die, record its PID at spawn and kill that PID -
  or just leave session-scoped helpers to die with the session.

## Action items

- [x] Ledger: new pkill-pattern-matches-own-shell (x1).
