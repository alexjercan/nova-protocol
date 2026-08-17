# Retro

## Bug and fix

- A continuous invariant must reconcile both sides. The WFC guard continuously paused clocks for interactive screens but relied on a one-time exit hook to resume them.
- Make the active match state explicit: frozen screen -> paused clocks; no frozen screen -> running clocks.

## Next time

- Test both entry and exit when adding a continuous state ownership guard.
