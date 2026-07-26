# DECISION: how the NOVA OS easter egg is shipped and triggered

- STATUS: ACCEPTED
- DATE: 2026-07-26

## Context

Goal: surface `examples/ui/nova_os_terminal_poc.html` in the marketing web app
as an easter egg, "opened from a secret location", while keeping the examples
copy as the single source of truth. The goal deliberately left the mechanism
open ("somehow"), and the plausible shapes are not interchangeable, so the fork
was taken to the user.

## Options considered (trigger)

1. Konami code on the landing page -> navigate to the hidden route.
2. Secret click hotspot: click the site brand/logo N times -> hidden route.
3. Unlinked secret URL only (no trigger; reachable by typing the address).

## Decision

- Route: `/nova-os/` (thematic, unlinked from all nav).
- Trigger: option 2, the brand/logo clicked 5 times rapidly.

The brand is a home link, so the hotspot only arms when the brand resolves to
the current path (you are on the landing page, where re-clicking home is an
otherwise-useless self-reload). There the handler swallows the clicks, counts
them in a rolling time window, and on the 5th navigates to the basePath-aware
`/nova-os/`. On every other page the brand keeps its normal "go home" behavior,
so nothing regresses.

## How source truth is preserved

The web build copies `../examples/ui/nova_os_terminal_poc.html` into
`dist/nova-os/index.html` via a CopyPlugin pattern at build time. No second copy
is committed under `web/src/`. The PoC is fully self-contained (inline CSS + one
inline `<script>`, no external/relative asset refs), so it renders correctly at
any deploy subpath (`/` locally, `/nova-protocol/` on GitHub Pages).
