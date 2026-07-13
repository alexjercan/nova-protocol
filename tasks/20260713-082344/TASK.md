# Reconcile targeting docs with the deliberate-radar model

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.5.0, docs, targeting, spike

## Goal

After the radar family lands (20260713-082324/-082330/-082337), sweep the
targeting docs so no doc asserts the dead models as current: the aim-assist
auto-acquisition and sticky-held docs (docs/2026-07-10-signature-lock.md and
friends, the component-lock and inset docs where they describe acquisition),
the CTRL free-aim and CTRL+scroll claims (CHANGELOG "Unreleased" entries may
need a rewrite rather than stacked contradictions), and supersession banners
on anything still recommending scroll cycling. Update the keybind-hints docs
and the spike fix records (20260713-082207 + the superseded 222610/215256
already carry banners).

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md.
- Depends on: 20260713-082337 (family complete).
- Replaces the dead docs task 20260712-223345 (closed wontdo); its file list
  is a starting inventory.
