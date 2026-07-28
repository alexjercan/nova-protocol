# Review: web easter egg - menu -> HUD -> NOVA OS CRT chain

- TASK: 20260728-185730
- BRANCH: feat/web-easter-egg

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Out-of-context reviewer (fresh subagent, no sight of the implementing session)
ran the DoD proofs (`npm test` pass; `npm run build` emits all three routes +
the 99 HUD glyphs), rendered `/nova-menu/` and `/nova-hud/` in chromium, and
audited the chain wiring, the guarded CRT change, route detection, localStorage
skin and rewrite order. In-session re-verified two load-bearing claims: the CRT
guard (`eggReturnHref()` returns null unless `?back=` is exactly
`nova-hud`/`nova-menu`, and `closeDrawer` only navigates when non-null - so the
paramless standalone/`/nova-os/` link is behaviour-unchanged) and finding #1
(grep confirms `hud_rework_poc.html` and `nova_os_terminal_poc.html` carry zero
skin machinery).

Verified clean: all navigation is relative (`../nova-hud/`, `../nova-os/?back=`,
`../nova-menu/`, `../`) and resolves under both `/` and the `/nova-protocol/`
base; immersive route detection hides the menu topbar and collapses the app grid
(fixing the empty-`auto`-track pitfall) while the `file://` review copies keep
full chrome; the skin setter syncs both controls under try/catch; the dev-only
`historyApiFallback` fires only on 404 HTML and does not shadow the real
`/nova-hud/assets/*` files.

- [x] R1.1 (MINOR) tasks/20260728-185730/TASK.md - Step 7 / Verification implied
  the UI-skin choice "survives the menu -> HUD -> CRT -> back hops", but only the
  menu PoC reads `novaSkin`; the HUD and CRT are single-look demos, so flipping to
  Hardware themes nothing downstream. Honesty/wording gap, not a wiring bug.
  - Response: Fixed - reworded Step 7 to state the setting is persisted and
    re-applied on return to the menu and themes the MENU surface only.
- [ ] R1.2 (NIT) examples/ui/nova_ui_rework_poc.html:590 - Sandbox maps to
  `data-nav="play"` (-> HUD), same as New Game; not spec'd but reasonable.
  - Response: Left as-is - Sandbox is a plausible "start play" entry; harmless.

### Pending manual items (owner, cleared at deploy/playtest)

1. DoD #3: live `webpack serve` playtest of the full chain (5x -> menu -> New
   Game -> HUD -> NOVA OS -> CRT -> close returns to HUD) + menu overlays in place.
2. DoD #4: not pushed/deployed - owner drives the web deploy via `/release`.
