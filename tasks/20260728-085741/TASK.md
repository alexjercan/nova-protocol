# NOVA OS: persistent header + main + footer layout

- PRIORITY: 47
- TAGS: v0.9.0, ui, hud, nova_os, refactor
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

Refactor the NOVA OS screen into a persistent header + main + footer layout
(HTML `<header>/<main>/<footer>` model). Today the "NOVA OS ... / COCKPIT LINK"
top bar lives INSIDE the terminal content node, so opening an app hides the
whole terminal (header included) and the app draws its own chrome bar instead.
The header must instead be a persistent, fixed-height sibling that is ALWAYS on
screen; only the middle "main" region swaps between the terminal surface and an
app body. The footer (keybind hints) is already a persistent sibling and stays
as-is (keybinds only, fixed height).

## Problem

`spawn_nova_os_terminal_content` (nova_os.rs) builds:

```
NovaOsTerminalContent (flex_grow 1)   <- hidden wholesale when an app opens
  |- NovaOsTopbar   = the "NOVA OS ... / COCKPIT LINK" | SHIP/LINK/FPS header
  |- NovaOsTerminalSurface (scrollback + prompt)
```

and `spawn_nova_os_footer` adds a sibling footer. When `sync_nova_os_app_ui`
enters App mode it sets `NovaOsTerminalContent` Visibility::Hidden and
`spawn_nova_os_app` draws a full-screen app root with its OWN chrome bar
(app.title() + a clickable `[ESC] CLOSE`). Result: the NOVA OS branding/header
disappears while an app is open, and there are two parallel header
implementations (topbar vs app chrome) that must be kept in visual sync.

## Target model (confirmed with owner)

Three persistent siblings under the content root (rtt.content_root / screen):

```
content_root (safe-area padding, flex column)
  |- NovaOsHeader   (persistent, FIXED height)   = <header>
  |    |- left:  brand lamp + "NOVA OS <ver> // <breadcrumb>"
  |    |- right: [ ESC ] close (app only) + "SHIP: .. LINK: LOCAL FPS: .."
  |- NovaOsMain     (persistent, flex_grow 1)     = <main>
  |    |- NovaOsTerminalSurface  (shown in Prompt mode)
  |    |- NovaOsApp body         (shown in App mode)
  |- NovaOsFooter   (persistent, FIXED height)    = <footer>  (unchanged: keybinds)
```

Confirmed decisions (see DECISION.md):

1. Breadcrumb: terminal surface reads `NOVA OS <ver> // SHELL`; an open app
   reads `NOVA OS <ver> // APPS / <APP>` (e.g. `APPS / MAP`). The `<ver>` keeps
   using `nova_os_version_label()`. The app segment uses a SHORT breadcrumb
   label (the command/app id upper-cased, e.g. `MAP`), NOT `app.title()` -
   the map's title is "MAP / LOCAL SPACE" which would render as
   "APPS / MAP / LOCAL SPACE".
2. Drop the "COCKPIT LINK" subtitle from the header prefix.
3. Right side stays `SHIP: .. LINK: LOCAL FPS: ..` (unchanged text/`nova_os_status_text`).
4. App close affordance: keep a clickable `[ ESC ]` control, but rehome it to
   the header's right side (left of the SHIP/LINK/FPS status), shown ONLY while
   an app is open. The old per-app chrome bar (title + close) is DELETED.
5. Header and footer are FIXED height (constant), so switching terminal<->app
   never reflows them. Main flex-grows between them.

## Approach

1. Extract the header into its own persistent spawn (e.g.
   `spawn_nova_os_header`) added as a sibling of main + footer in BOTH the
   render-capable branch (rtt.content_root, ~line 3286) and the headless
   fallback branch (screen, ~line 3269). Give it a fixed height
   (`NOVA_OS_HEADER_HEIGHT_PX`) matching today's topbar box (~32px content +
   its 10px bottom pad + 1px border) so the visual size is preserved.
   - Left cluster: keep `NovaOsLampMarker` + a `NovaOsBrandMarker` text node.
     Seed with the terminal breadcrumb `NOVA OS <ver> // SHELL`.
   - Right cluster: a row holding (a) a `NovaOsAppCloseMarker` `[ ESC ]` button
     (start Hidden), then (b) the existing `NovaOsStatusMarker` status text.
2. Move the terminal surface (scrollback + prompt) into a persistent
   `NovaOsMain` container (flex_grow 1). The terminal surface is shown/hidden by
   toggling visibility of the terminal-surface subtree (NOT a wrapper that also
   holds the header, since the header no longer lives with it).
   - Rename/repurpose `NovaOsTerminalContentMarker`: it should now tag ONLY the
     terminal surface (the thing hidden in App mode), not the header. Update
     `sync_nova_os_app_ui`'s `q_content` accordingly so it hides only the
     terminal surface, and spawn the app body INTO `NovaOsMain` (absolute-fill
     over the hidden terminal surface, as today) so the shared CRT overlay and
     footer reserve still work.
3. `spawn_nova_os_app`: DELETE the chrome bar (title + `[ESC] CLOSE` button).
   The app root becomes just the body (keep the absolute fill + footer-reserve
   margin + safe-area handling). `NovaOsAppCloseMarker` + `on_nova_os_app_close`
   observer move onto the header's close control.
4. Breadcrumb + close-button reconciliation: add a system (or extend the
   existing footer-hint rebuild, which already fires on `active_mode` change via
   a `Local`) that, on active-surface change, updates:
   - `NovaOsBrandMarker` text: Prompt -> `NOVA OS <ver> // SHELL`;
     App{id} -> `NOVA OS <ver> // APPS / <ID-UPPER>`.
   - `NovaOsAppCloseMarker` visibility: Visible in App mode, Hidden in Prompt.
   Keying on active_mode (like `rebuild_nova_os_footer_hints`) avoids thrashing
   on ordinary prompt edits.
5. Keep `drive_nova_os_topbar_fps` working: it targets `NovaOsStatusMarker`,
   which is unchanged and now lives in the header's right cluster.
6. Fixed sizes: header gets `NOVA_OS_HEADER_HEIGHT_PX`; footer already has a
   small min_height - promote it to a fixed `NOVA_OS_FOOTER_HEIGHT_PX` so both
   bars are constant. Verify `NOVA_OS_FOOTER_RESERVE_PX` still clears the footer
   for app bodies.

## Steps

- [x] Add `NOVA_OS_HEADER_HEIGHT_PX` (+ `NOVA_OS_FOOTER_HEIGHT_PX`) constants and
      a `NovaOsBrandMarker`; add a `nova_os_header_breadcrumb(mode)` helper that
      returns `NOVA OS <ver> // SHELL` / `... // APPS / <ID>`.
- [x] Write `spawn_nova_os_header` (persistent, fixed height): left lamp+brand,
      right `[ ESC ]` close button (Hidden) + `NovaOsStatusMarker` status.
      Carry the `NovaOsAppCloseMarker` + `on_nova_os_app_close` observer here.
- [x] Wrap the terminal surface in a persistent `NovaOsMain` (flex_grow 1) and
      spawn header/main/footer as three siblings via `spawn_nova_os_chrome` in
      both the rtt and headless branches. Remove the header from
      `spawn_nova_os_terminal_content`.
- [x] Repurpose `NovaOsTerminalContentMarker` to tag only the terminal surface;
      update `sync_nova_os_app_ui` to hide only that surface and spawn the app
      body into `NovaOsMain` (single `q_main` target for rtt + headless).
- [x] Delete the app chrome bar from `spawn_nova_os_app` (body only; no safe-area
      pad or footer-reserve margin - `<main>` already handles both).
- [x] Add `reconcile_nova_os_header` (mode-keyed, mirrors the footer rebuild) to
      update the brand breadcrumb text and toggle the header close button on
      active-surface change.
- [x] Promote the footer to fixed height; app bodies fill `<main>` above the
      footer, so the old reserve margin was removed (no longer needed).
- [x] Update module docs / comments (app.rs trait doc + nova_os.rs fn/marker
      docs) that described the old topbar-in-terminal and per-app chrome bar.
      History under tasks/ is not edited.

## Implementation notes

- New structure under the content root (rtt content root or the headless screen):
  three siblings `NovaOsTopbarMarker` (header, fixed height) / `NovaOsMainMarker`
  (`<main>`, flex_grow) / `NovaOsFooterHintsMarker` (footer, fixed height). The
  app root spawns as an absolute-fill child of `<main>` (which is
  `PositionType::Relative` to be its containing block), so it renders through the
  same CRT pass and never covers header/footer - the footer-reserve margin and
  the app's own safe-area padding both became unnecessary and were dropped.
- The `[ ESC ]` close (`NovaOsAppCloseMarker`) + its `on_nova_os_app_close`
  observer now live once in the header, start `Visibility::Hidden`, and are shown
  only in App mode by `reconcile_nova_os_header`.
- `NovaOsTopbarMarker`/`NovaOsLampMarker`/`NovaOsStatusMarker` kept so the PoC
  structure test and `drive_nova_os_topbar_fps` need no change; the brand text
  node gained `NovaOsBrandMarker` for the breadcrumb rewrite.
- Verified end to end with `screenshot_nova_os` (see `shots/`): `nova-os-welcome`
  + `nova-os-active` show `// SHELL` with no close button; `nova-os-map` shows
  `// APPS / MAP`, the `[ ESC ]` control, the unchanged right-side status, and
  the map keybind footer - header + footer identical across all three.

## Definition of Done

1. The `NOVA OS <ver> // ...` header and the keybind footer are visible in BOTH
   the terminal and while the map app is open, at a constant height (no reflow
   on open/close). (manual: open NOVA OS with Tab, run `map`, confirm header +
   footer stay put; ESC back, confirm same.)
2. Header left shows `NOVA OS <ver> // SHELL` at the prompt and
   `NOVA OS <ver> // APPS / MAP` while the map app is open. (manual)
3. Header right always shows `SHIP: .. LINK: LOCAL FPS: ..` with a live FPS, in
   both surfaces. (manual)
4. A clickable `[ ESC ]` control appears on the header right ONLY while an app
   is open and closes the app back to the prompt; no per-app chrome bar remains.
   (manual: click it, confirm returns to SHELL.)
5. `cargo check -p nova_gameplay -p nova_os` is green; `cargo fmt` clean.
   (cmd: `cargo check -p nova_gameplay -p nova_os`)
6. Any new/changed unit test for `nova_os_header_breadcrumb` passes.
   (cmd: `cargo test -p nova_gameplay nova_os_header`)
