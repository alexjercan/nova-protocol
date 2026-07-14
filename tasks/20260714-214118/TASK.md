# Centralize gameplay HUD palette into nova_ui; align chrome, preserve semantic hues

- STATUS: OPEN
- PRIORITY: 50
- TAGS: ui,v0.6.0

Umbrella: task 20260714-212139. Depends on: 20260714-214111 (nova_ui).

## Goal

Bring the gameplay HUD onto ONE palette without breaking gameplay legibility.
The HUD's ~31 ad-hoc colour consts across ~14 files are largely SEMANTIC (threat
= red, ally = green, nav = cyan, objective = gold, neutral = steel, damage-type
hues), so this is NOT a recolor - it is: (1) route the shared/neutral CHROME
(panel + readout backdrops, borders, neutral/steel text, backgrounds, fonts)
through `nova_ui::theme`, and (2) define the SEMANTIC accents ONCE in `nova_ui`
(nav-cyan, objective/ammo-amber, threat-red, ally-green, neutral-steel) and have
the HUD reference those, so the palette has a single source of truth while every
meaningful hue is preserved. The result is consistency with the web app's
cyan/amber chrome + a de-duplicated palette, with combat readability unchanged.

Done = HUD chrome colours come from `nova_ui`; the per-file semantic consts
reference shared `nova_ui` semantic accents instead of raw literals; no meaningful
hue changes (hostile still red, ally still green, nav still cyan, objective still
gold); gameplay HUD tests + the relevant example autopilots (e.g. `10_playable`,
`11_hud_range`) stay green and look correct.

## Steps

- [ ] Add `nova_ui = { path = "../nova_ui" }` to `crates/nova_gameplay/Cargo.toml`.
- [ ] Add a `semantic` block to `nova_ui::theme` (or a `theme::semantic` module):
  `NAV` (= CYAN), `OBJECTIVE`/`AMMO` (= AMBER), `THREAT` (combat red), `ALLY`
  (green), `NEUTRAL`/`STEEL` (light gray), plus neutral `BACKDROP` (the recurring
  `srgba(0.15,0.15,0.15,0.8)`) and `OUTLINE`. Pick the canonical hues from the
  current HUD consts so nothing shifts visibly. All `pub`.
- [ ] Chrome pass - replace neutral/backdrop/border/text literals in the HUD with
  `nova_ui::theme` refs, one file at a time, verifying nothing shifts:
  - `hud/mod.rs` objectives panel (280px, 13px, gold accent) - backdrop/text ->
    theme, `OBJECTIVE_GOLD` -> `theme::semantic::OBJECTIVE`.
  - `hud/torpedo_target.rs`, `hud/ammo_readout.rs`, `hud/target_inset.rs`,
    `hud/lock_crosshairs.rs`, `hud/component_lock.rs`, `hud/keybind_hints.rs`,
    `hud/turret_lead.rs`, `hud/edge_indicators.rs`, `hud/item_highlights.rs`,
    `hud/objective_feedback.rs`, `hud/flight_status.rs` - the backdrops
    (`0.15,0.15,0.15,0.8`), outlines, steel/neutral tones, and nav-cyan/gold
    accents move to shared consts; keep the file's own const NAME as a
    `pub(crate)` alias of the shared value where that reads clearer locally.
- [ ] Semantic pass - point the threat/ally/neutral/faction consts
  (`target_inset.rs` FACTION_*, the combat reds in `edge_indicators.rs`,
  `lock_crosshairs.rs`, `torpedo_target.rs`, `component_lock.rs`) at the shared
  `theme::semantic` accents WITHOUT changing the hue. `damage.rs::damage_type_color`
  stays a semantic function; if its hues match shared accents, reference them,
  else leave a comment that damage-type colours are their own semantic set.
- [ ] Do NOT touch diegetic/world colours: section materials
  (`sections/*_section.rs`), the velocity sphere gizmo (`velocity.rs`), juice
  particles (`juice.rs`) - these are 3D, not UI chrome.
- [ ] `cargo check --workspace --all-targets --features debug` clean; `cargo fmt`;
  `cargo test -p nova_gameplay`; run `10_playable` and/or `11_hud_range`
  headless (BCS_AUTOPILOT) and eyeball a `BCS_SHOT` capture to confirm the HUD
  reads the same (combat red/ally green/nav cyan intact).

## Notes

- Relevant files (from the UI inventory): `crates/nova_gameplay/src/hud/*` (~14
  files, 31 colour consts) and `crates/nova_gameplay/src/damage.rs`
  (`damage_type_color`, lines ~139-150). Recurring neutral backdrop is
  `srgba(0.15,0.15,0.15,0.8)`; nav-cyan `srgba(0.3,0.9,1.0,0.9)` (mod.rs:108);
  objective-gold `srgba(1.0,0.85,0.3,0.95)` (mod.rs:114).
- This is the SUBTLE, higher-risk task: the HUD colours carry meaning, so the win
  is a single palette source + web-app-consistent chrome, NOT a visual recolor.
  If a colour cannot be shared without changing meaning, leave it and note why.
- A deeper HUD visual redesign (layout, new chrome language) is OUT of scope and
  would want its own spike; this task only centralizes + aligns.
- Verify by eyeball (BCS_SHOT) as well as tests - HUD legibility is visual.
- Depends on: 20260714-214111. Sibling: 20260714-214115 (menu).
