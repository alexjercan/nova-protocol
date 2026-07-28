# DECISION: web easter-egg chain shape

- DATE: 20260728-000000
- STATUS: ACCEPTED
- TASK: 20260728-185730
- TAGS: decision, web, ui, easter-egg

Owner-confirmed at the flow plan gate, 2026-07-28.

## D1 - Chain shape: menu -> HUD -> CRT (mirrors the game)

STATUS: ACCEPTED. The 5x brand-click opens the reworked main menu; New Game
goes to the flight HUD PoC (`/nova-hud/`); a NOVA OS button on the HUD opens the
CRT terminal (`/nova-os/`). Chosen over the earlier two-step (menu -> CRT
directly) because the game flow is menu -> play -> Tab-opens-computer.

## D2 - HUD page keeps its situation controls

STATUS: ACCEPTED. On the deployed `/nova-hud/` route the situation toggles stay
(POC tag hidden, chrome trimmed) so visitors can drive the HUD through
combat/AP/objective states - the interactive fun. Chosen over a clean static
idle HUD.

## D3 - CRT return is a guarded navigation

STATUS: ACCEPTED. The HUD's NOVA OS button links to `/nova-os/` with a return
hint (e.g. `?back=nova-hud`); `nova_os_terminal_poc.html` only reads that hint to
make its exit/sleep navigate back to the HUD, so its DEFAULT (paramless) use -
the standalone review copy and any existing link - is byte-for-byte unchanged in
behaviour. Chosen over forward-only (browser-back) because "Esc resumes flight"
is part of feeling like the game, and over an unguarded edit that would change
the canonical reference's default.
