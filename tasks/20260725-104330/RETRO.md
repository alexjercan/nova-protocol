# Retro: Epic - NOVA OS terminal drawer for v0.9.0

- TASK: 20260725-104330
- BRANCH: (epic container)
- CHILD TASKS: 7 (5 core + 2 stretch), all CLOSED and landed

This is the epic-level process retro; each child task has its own RETRO.md with
the detail. Process observations only - see SPIKE.md/DECISION.md for the design.

## What went well

- Spiking first paid off: the raw playtest feedback was a grab-bag ("terminal
  plus a second screen", comms, objectives, campaign polish). /spike narrowed it
  to one load-bearing decision (one monitor with app takeover, not two panels),
  captured in DECISION.md, and the whole epic built cleanly against that single
  shape without a mid-epic re-cut.
- The core/stretch split held exactly as planned: shell -> input -> PoC match ->
  output commands -> app runtime, then map + ship viewer as apps on top. Later
  polish (label tables, `map goto`) slotted in as separate small tasks rather
  than widening any child mid-flight.
- A standalone HTML PoC (`examples/ui/nova_os_terminal_poc.html`) as the visual
  target gave every UI child an unambiguous acceptance reference.

## What went wrong

- The epic container tracked the two stretch children as unchecked (`[ ]`) even
  after both had landed and been extended further, so the container looked
  in-progress while the work was fully done. Root cause: child close-out ticks
  the child's own STATUS but nothing walks back to tick the parent's list. Cost:
  a stale-looking epic until an explicit close pass.

## What to improve next time

- When a child task lands, tick its box in the parent epic's Child Tasks list in
  the same close-out (flow already says to; make it reflexive), so the container
  never lies about remaining work.

## Action items

- [x] Ledger: added `epic-parent-list-lags-child-close`.
- No follow-up code tasks: the v0.9.0 NOVA OS epic shipped its core + both
  stretch apps. Non-drawer feedback (campaign opening, per-scenario polish,
  map-boundary/radar follow-ups) was intentionally out of scope and remains as
  future context in TASK.md, not open work here.
