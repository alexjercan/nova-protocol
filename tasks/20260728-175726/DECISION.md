# DECISION: UI-rework load-bearing forks

- DATE: 20260728-000000
- STATUS: ACCEPTED
- TASK: 20260728-175726
- TAGS: decision, ui, hud, menu

All six forks ACCEPTED by the owner on 2026-07-28 through live review of the two
HTML PoCs (`examples/ui/nova_ui_rework_poc.html`,
`examples/ui/hud_rework_poc.html`). These are the forks the epic
(20260728-175719) deferred to the spike; they gate implementation on the
affected children. Rationale detail lives in SPIKE.md.

## D1 - Primary skin: phosphor terminal, CLI-rendered widgets

STATUS: ACCEPTED. All player UI uses the NOVA OS phosphor terminal look as the
primary skin; the hardware-casing light-3D look is a secondary alternative. In
phosphor mode every widget re-renders as a CLI element (flat phosphor borders,
inverted selection, ASCII-meter sliders, bracketed tags), not a 3D control on
glass. Chosen over: hardware casing primary; phosphor-as-background-only.

## D2 - Main menu: corner panel over the live scene

STATUS: ACCEPTED. The main menu is a compact bottom-right panel over the live
`menu_backdrop` scene (Factorio-style, scene is the focus), matching the shipped
layout. Chosen over: a full centered menu panel that covers the scene.

## D3 - Contextual HUD ruleset

STATUS: ACCEPTED. Show-by-relevance (elements appear when their situation is
live) + grow-in-use-then-settle emphasis. KEEP the velocity-direction shader
(always on) and the top-right locked-target zoom PiP. Ammo shows one group per
weapon. Detail delegated to NOVA OS. Chosen over: always-on full HUD; a HUD with
no persistent instruments.

## D4 - Keybind hint shape: icon-chip dock with real key glyphs

STATUS: ACCEPTED. The 7-row `[KEY] VERB` text cluster becomes a contextual
icon-chip dock using FREE Input Prompts (CC0) **Alt**-style keycap glyphs (Dark/
White secondary). Folds backlog 20260710-231927. Chosen over: keeping the text
rows; a bespoke drawn keycap.

## D5 - `~` HUD levels: On / Cinematic

STATUS: ACCEPTED. Two levels only - On (full auto-contextual) and Cinematic
(clean screen). Chosen over: keeping the All / Minimal / None triple (auto-hide
already covers Minimal).

## D6 - Units: 1 u = 10 m, m/km threshold 1000 m, m/s

STATUS: ACCEPTED. Player-facing distance/speed display at 1 u = 10 m: metres
below 1000 m, kilometres (2 decimals) at/above; speed m/s; closing speed signed
m/s. The unit `u`/`u/s` retires from the player surface and the wiki glossary.
Display-only; physics/content/AI untouched. Detailed in child 20260728-175731.
