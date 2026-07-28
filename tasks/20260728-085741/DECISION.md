# DECISION: NOVA OS header/main/footer chrome

- STATUS: ACCEPTED

Load-bearing build-shape choices for the persistent header + main + footer
refactor, confirmed with the owner before build.

## Context

The "NOVA OS ... / COCKPIT LINK" top bar currently lives inside the terminal
content node, so opening an app hides it and the app draws its own chrome bar
(title + clickable `[ESC] CLOSE`). Owner feedback: the header and footer are
like HTML `<header>`/`<footer>` - always on screen at constant size - and only
the middle `<main>` swaps between the terminal and an app. That fixes the
placement, but three concrete artifact/shape forks had mutually-exclusive
constraints and went to the owner.

## Decisions (ACCEPTED)

1. **Terminal breadcrumb wording.** The terminal surface reads
   `NOVA OS <ver> // SHELL` (owner picked SHELL over TERMINAL/CONSOLE/COMMS).
   An open app reads `NOVA OS <ver> // APPS / <APP>`.

2. **Subtitle dropped.** The old "COCKPIT LINK" subtitle is removed from the
   header prefix; the breadcrumb tail carries the context instead.

3. **App close affordance -> header, not footer.** Constraint: the owner's model
   says the header right side "stays with FPS and ship" and the footer "just
   contains keybinds", so neither slot has room for a clickable close button,
   yet the owner also wanted to keep a mouse affordance. Resolution: rehome a
   single `[ ESC ]` clickable control to the header's RIGHT cluster, left of the
   SHIP/LINK/FPS status, shown ONLY while an app is open (Hidden at the prompt).
   This is a deliberate, minimal bend of "right side is just FPS/ship" that the
   owner chose explicitly over dropping the click target. The per-app chrome bar
   (title + close) is deleted; the app title is now carried by the header
   breadcrumb.

4. **App breadcrumb label = command/app id, not `app.title()`.** The breadcrumb
   uses the short upper-cased id (`MAP`), because the map app's `title()` is
   "MAP / LOCAL SPACE" and would render as "APPS / MAP / LOCAL SPACE".

5. **Header + footer are fixed height.** Constant `NOVA_OS_HEADER_HEIGHT_PX` /
   `NOVA_OS_FOOTER_HEIGHT_PX` so switching terminal<->app never reflows the bars.

## Alternatives considered

- Breadcrumb word CLI/TERMINAL/CONSOLE/COMMS: owner wanted "more nova-ish than
  CLI"; SHELL chosen.
- Close via ESC keybind only (footer already lists "ESC: BACK"): rejected by the
  owner in favor of keeping the clickable control (moved to the header).
