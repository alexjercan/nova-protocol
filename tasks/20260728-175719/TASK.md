# Epic: UI rework - NOVA OS look everywhere, quieter contextual HUD, metric units

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.9.0, epic, ui, hud, feedback

## Context

Owner direction (2026-07-28, /flow): extend the proven NOVA OS visual language
from the Tab monitor to the ENTIRE UI - "button vibes and basic light 3D
vibes", covering the main menu and parts of the in-game UI - while cutting the
amount of text on screen during gameplay and switching the displayed distance
unit from `1 u` to `10 meters`. The owner wants the directions explored through
HTML demos first, the same method that produced
`examples/ui/nova_os_terminal_poc.html` for the NOVA OS monitor.

This is the deferred follow-up the NOVA OS spike named explicitly
(tasks/20260725-104330/SPIKE.md): "the main menu and other UI can follow later
once the drawer proves the style". The drawer shipped in v0.9.0 and proved it.
Owner decision 2026-07-28: this epic EXTENDS v0.9.0 rather than opening
v0.10.0; v0.9.0 tags only after this epic closes.

## Current state (understood 2026-07-28)

- NOVA OS monitor: green phosphor on near-black screen inside a dark blue-black
  moulded casing with light-3D physical controls (BackgroundGradient +
  BoxShadow bevels), Iosevka Term font, amber/orange accents. Canonical
  reference: `examples/ui/nova_os_terminal_poc.html` +
  `crates/nova_gameplay/src/hud/nova_os.rs`.
- Everything else: flat navy/cyan/amber `nova_ui::theme` mirrored from
  `web/src/style.css`; zero gradients/shadows outside the monitor. Palette
  changes propagate automatically (menus/editor/HUD chrome consume the theme);
  widget SHAPES are per-site. `nova_menu` duplicates its own MenuButton color
  system next to `nova_ui`'s ThemedButton observers.
- HUD: ~50 elements across 3 tiers (Instrument/Chrome/Status) x 3 levels
  (All/Minimal/None on `~`). Much contextual show/hide already exists (locks,
  AP chips, availability cues). Biggest text block: the 7-row keybind hint
  cluster. Units are INCONSISTENT: `u`/`u/s` in the speed chip, closing speed,
  orbit spoke and the map app; bare `m` at an implicit 1:1 scale in combat DST,
  destination readout, edge arrows, objective/beacon chips, radar label.
- Bevy 0.19 BackgroundGradient + BoxShadow are already in use by the monitor
  casing, so the light-3D widget treatment needs no new engine machinery.

## Epic

Spread the NOVA OS visual language across menus, editor chrome and flight HUD
(palette, typography, light-3D "physical control" widgets); reduce on-screen
text during gameplay by making the HUD contextual (elements appear when
relevant and grow while in direct use, detail lives in the NOVA OS computer);
unify all player-facing distances/speeds at a 1 u = 10 m display scale
(meters/km, m/s). Directions are explored and accepted through HTML demos
before Bevy work starts.

## Done Means

1. Both HTML demos exist under `examples/ui/` and the spike's SPIKE.md records
   the owner-accepted directions (cmd: `ls examples/ui/*poc*.html`).
2. All player-facing distances/speeds display meters/kilometers (m/s for
   speed) at 1 u = 10 m; the unit `u` is retired from HUD, NOVA OS output and
   the wiki (cmd + test: recorded in the units child task).
3. Menus, editor chrome and HUD chrome render the accepted NOVA OS-derived
   language: shared palette + light-3D widget treatment; no screen still shows
   the old flat navy/cyan theme (manual: owner eyeballs each screen;
   screenshot examples updated).
4. In-game text density drops per the accepted spike decisions (keybind hints,
   chips, comms); the NOVA OS computer carries the detail (manual: owner
   playtest verdict).
5. HUD elements show/emphasize contextually per the accepted spike design
   (test: harness coverage of the visibility/emphasis rules; manual:
   playtest).

## Child Tasks

- [x] 20260728-175726 (p44) Spike: HTML demos - menu widget language + contextual HUD behavior
      landed cc5be9bd; 1 review round (APPROVE, out-of-context); owner accepted both
      PoCs. Directions in tasks/20260728-175726/SPIKE.md + DECISION.md (D1-D6);
      children 175734/38/42/47 refined; follow-ups 185730 (web egg), 214929 (glyphs).
- [x] 20260728-175731 (p42) Units: display 1 u = 10 m everywhere (m/km, m/s)
      landed 93032f53; formatter nova_ui::units + 11 sites + docs sweep; sweep
      record in the task (DoD 3+4).
- [x] 20260728-233707 (p41) Relocate input-prompt key glyphs to assets/ (Alt only) + credits
      landed; 1 review round (APPROVE, out-of-context). 99 Alt glyphs moved to
      assets/input-prompts/keyboard/Alt/, Dark/White dropped (2.4M->~800K),
      license absorbed into credits/CREDITS.md + credits/licenses/, webpack +
      HUD PoC repointed. Unblocks the HUD icon dock (175742).
- [x] 20260729-000956 (p50) Preload static assets via bevy_asset_loader + phosphor boot loading screen
      landed; bevy_asset_loader collections + phosphor boot screen; also fixed
      the probe FPS regression (d96a9b54 closes it).
- [x] 20260728-175734 (p40) nova_ui theme + widgets: NOVA OS palette + skin-aware widget set
      landed 634eb3e7 (+ aab05fe8 widget_zoo duplicate-Node/text-ghost fix);
      nova_menu's button system folded into nova_ui.
- [x] 20260728-175738 (p38) Menus + editor adopt the reworked widget language
      landed 6ef67424 + 87dc6ba8; UI skin setting and the scenarios scroll
      driver came with it.
- [x] 20260728-175742 (p36) HUD restyle + on-screen text reduction (icon dock)
      landed 89dd435b (legacy navy/cyan retired from HUD + theme) + 5b19d08a
      (phosphor chip family + contextual keycap icon dock).
- [x] 20260728-175747 (p34) Contextual HUD: show-by-relevance + grow-in-use + On/Cinematic
      landed e109b5cf + 92c153a8; objective read-notification stack replaces
      the status-bar hint and the objective chip is the posting itself.

Owner playtest feedback wave 2026-07-30 (/flow) - direct verdicts on this
epic's DoD 3/4/5, planned as three more children:

- [x] 20260730-122843 (p50) Keybind dock shows only currently-available verbs again
      landed 1f1a5c40; available-only dock with the scenario-spotlight override
      from its DECISION.md.
- [x] 20260730-122909 (p48) Floating chip background covers only a corner of its label
      landed d1460fc5; world-anchored chips back their whole label.
- [x] 20260730-122940 (p46) Wide keycaps (Tab/Shift/Space) render at their art aspect
      landed 69dc505c; keycaps size from their art's proportions.

Two more items from the same playtest are NOT part of this epic (they touch
targeting and the NOVA OS map, neither in the epic's Done Means) and run as
standalone v0.9.0 tasks: 20260730-123009 (combat lock lets go) and
20260730-123039 (map app clicks miss).

Re-planned 2026-07-28 (post-spike, owner /flow directive): all remaining
children now carry concrete Steps + DoD grounded in the implemented PoCs and
the 2026-07-28 code maps (nova_ui/nova_menu/nova_editor/HUD/asset pipeline).
Child 233707 was added for the owner's asset-structure directive (glyphs out
of examples/, Alt only, license in credits/). Notable corrections baked in:
the hardware skin is the case-gradient look, NOT the old navy/cyan (which
retires entirely); the dock uses the real 7-verb set + live bindings; the
scenarios scroll bug is a missing driver system for ScenariosList.

## Decisions

- 2026-07-28 owner: extend v0.9.0 with this epic instead of opening v0.10.0;
  v0.9.0 tags only after the epic closes. (Recorded here + grooming note in
  tracker 20260724-083631.)
- Load-bearing style forks (full CRT-terminal menus vs casing-hardware look,
  contextual HUD ruleset, keybind-hint shape, m/km threshold) are DECIDED BY
  THE SPIKE demos with the owner and recorded in the spike's
  SPIKE.md/DECISION.md before implementation starts on the affected children.
- 20260730-122843 DECISION.md: an unavailable dock chip is hidden EXCEPT when a
  scenario spotlights that verb, which forces it visible in the dim pulse band -
  strict hiding and the tutorial spotlight cannot both hold. (ACCEPTED)
- 20260729-000956 DECISION.md: static assets preload via bevy_asset_loader
  collections + a phosphor boot loading screen. Boot-state font gating (D1),
  single-face .ttf extraction (D2), indeterminate CRT animation (D3), UI-SFX
  load-gating without a bevy_common_systems release (D4), DoD-1 test scoped to
  the chaining mechanism (D5). (ACCEPTED)

## Manual Acceptance

- (done 2026-07-28) owner reviewed both HTML demos and accepted the directions
  (spike 20260728-175726; phosphor-primary CLI widgets, corner menu, contextual
  HUD ruleset, icon dock, On/Cinematic, 1u=10m).
- (2026-07-30, interim) owner playtested the reworked HUD. Verdicts: the dock's
  dim-chip rule is REJECTED (wants available-only back, 122843); the world-
  anchored chip backgrounds are BROKEN (122909); the wide keycaps are
  ILLEGIBLE (122940). No verdict yet on overall text density - it is re-asked
  once these three land.
- (done 2026-07-31) all three feedback-wave fixes landed (1f1a5c40, d1460fc5,
  69dc505c) and the owner signed the epic off at the /flow Finish gate: DoD 3
  (every screen carries the NOVA OS language), DoD 4 (text density) and DoD 5
  (contextual show/emphasis) ACCEPTED. No further verdicts outstanding.

## Notes

Finish gate 2026-07-31 (/flow), verified on master at 48ae5962:

- DoD 1: `ls examples/ui/*poc*.html` -> nova_os_terminal_poc.html,
  nova_ui_rework_poc.html, hud_rework_poc.html. Spike 20260728-175726's
  SPIKE.md + DECISION.md carry the accepted directions.
- DoD 2: `crates/nova_ui/src/units.rs` (`distance`/`speed`/`closing_speed`) is
  the single formatter; a repo-wide grep finds no `u`/`u/s` in HUD, NOVA OS or
  wiki output. Remaining `u/s` hits are internal world-unit comments and the
  archived 0.7.0/0.8.0 news posts, which are historical and stay as written.
- DoD 3-5: children 175734/175738/175742/175747 + the 2026-07-30 feedback wave
  (122843/122909/122940) all CLOSED; owner accepted at this gate (see Manual
  Acceptance).
- Checks: `cargo check --workspace` clean, `cargo fmt --check` clean,
  `tatr check` clean. `tatr check --ledger LESSONS.md` reports only pending
  promotions awaiting the owner's decision plus one malformed disposition
  (`reuse-known-good-stack` carries `positive`, not a
  PROMOTE|DEFER|RETIRE|ABSORBED token) - both are ledger-level, not epic work.
- Not epic blockers, left open on the backlog: 20260729-140945 and
  20260730-161545 are two records of the SAME red shakedown test
  (`an_early_derelict_kill_skips_to_the_fight`), confirmed inherited rather
  than caused by this epic. They should be merged into one task.

Extra (owner request 2026-07-28, during the spike): 20260728-185730 LANDED
(80427d2d) - wires the reworked PoCs into the web app as a hidden easter egg
chain (5 brand-clicks -> /nova-menu/ -> New Game -> /nova-hud/ -> NOVA OS button
-> /nova-os/ CRT, close returns to the HUD). Wire-only, clean-immersive with a
Phosphor/Hardware skin switch in the PoC Settings; owner deploys via /release.
NOT part of this epic's Done Means.

Related pre-existing tasks: backlog 20260710-231927 (keybind hint icons) folds
its ICON half into the HUD text-reduction child 175742 (remapping + gamepad
half stays in 231927); backlog 20260714-214329 (ship Rajdhani/Inter/JetBrains
web fonts) is superseded by the accepted mono-first direction - 175734 routes
Iosevka Term as the UI typeface - PROPOSED CLOSE at the 2026-07-28 re-plan
gate (owner call); backlog 20260728-214929 (glyph adoption) had its canonical-
home question answered by 233707's DECISION.md and keeps only the remaining
surfaces (web key-UI, NOVA OS help, editor chips, gamepad); backlog
20260726-193040 (NOVA OS CRT look spike) stays separate (monitor-internal
polish). Wontdo 20260714-225524 (web cyan/amber alignment) is superseded by
this direction change.
