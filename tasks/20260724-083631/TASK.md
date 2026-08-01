# v0.9.0 release tracker: Cockpit & Command - ship-computer drawer, combat readability, scenario browsing

- STATUS: IN_PROGRESS
- PRIORITY: 1
- TAGS: v0.9.0, release, meta
- KIND: TASK
- FLOW STEP: REVIEWING
- PLAN STATUS: APPROVED

Release-level tracker for v0.9.0. Per-strand work lives in its own tatr task
(tagged `v0.9.0`); this task carries the release theme, strand map, out-of-scope
decisions, and the grooming history - the same shape as the v0.8.0 tracker
(20260720-142428).

- DATE: 2026-07-24
- BASE: master at v0.8.0 (head 70587839)
- THEME: **Cockpit & Command.** Turn the cockpit into a real ship-computer and
  make combat legible. The headline is now the one-screen **NOVA OS** Tab drawer:
  an inset cockpit monitor where commands either print inline terminal output
  (`help`, `log`, `objectives`, `ship`) or launch apps (`map`, later
  `ship viewer`) that take over the same monitor until exited. The drawer also
  becomes the first pass at the stronger HUD visual language: darker Nova
  blue-black casing, green phosphor screen, orange/yellow accents, CRT scanlines
  and diagnostic FPS/version chrome. This sits alongside at-a-glance combat
  readability (allegiance markers over ships) and better campaign/scenario
  browsing. Unlike v0.8.0 (pure debt paydown, no new features), v0.9.0 is a
  features release.

## Why this scope

v0.8.0 paid down docs/tooling debt and lengthened the campaigns. The game now
has content worth reading clearly, but the in-flight information surface is thin:
objectives and comms are minimal, you cannot tell friend from foe at a glance in
a busy scene, and the scenario picker is a flat list. This release invests in the
cockpit as an information system and in combat readability, on top of the stable
v0.8.0 base.

The owner's stated focus for this release, in preference order:

1. **Goal C - the cockpit ship-computer drawer (100% in, the headline).** The
   Tab drawer becomes NOVA OS: one terminal monitor, command output, launchable
   apps, and the new CRT/green-phosphor visual direction. The feedback spike
   `20260725-104330` is now the planning source for the remaining drawer work.
2. **Goal D - improved scenario picker only (100% in).** Collapsible campaign
   headers + campaign->scenario mapping. NOT the per-scenario thumbnail art
   (stays backlog, 20260715-220011).
3. **Goal A - the allegiance marker over ships (in).** The rest of the old
   ch5/gravity Goal-A cluster is deferred/dropped (see out of scope).
4. **Goal B - the kill/critical-damage model (STRETCH).** Nice to have; cut
   first if Goal C runs long.

## In scope, in execution order

IDs are tatr task IDs under `tasks/`. Priorities encode order (higher = earlier /
more important). SIZE is a rough effort estimate (S/M/L), not a commitment. All
are tagged `v0.9.0`. Per-task DoD and Steps are defined in the v0.9.0 planning
pass (see "Planning - next step"), NOT in this tracker.

### Strand C - Cockpit ship-computer drawer (headline)

1. **20260721-211512** (p80, SIZE M, spike/ui/hud) Spike: the Tab ship-computer
   drawer. **CLOSED 2026-07-24** - SPIKE.md RECOMMENDED. Fixed the v0.9.0
   contents to **objectives + comms log** (deferred the 3D minimap and
   ship-status to backlog per the "cannot balloon" directive) and the
   interaction model (Tab keybind, new `PauseStates::Drawer` sim-gate variant,
   slide-in via bcs TweenPlugin, tab-handle anchor as 211520's tuck-target).
   Seeded the shell + log tasks below. Depended on 20260721-211500 (CLOSED).
2. **20260724-102304** (p72, SIZE M, spike/feature/ui/hud) Drawer shell +
   interaction model + objectives section. **CLOSED / LANDED 2026-07-24**
   (c13143d4; 2 review rounds - out-of-context R1.1 caught an audio-loop freeze
   gap). The headline foundation - Tab keybind, `PauseStates::Drawer` (third
   variant on one freeze axis, DECISION.md), `Time<Real>` slide (bcs Tween is
   virtual-clocked), `DrawerTabAnchor` for 211520, expanded objectives section.
   Now UNBLOCKS 211520 and the comms-log section (102309).
   - Manual acceptance (batched to owner): open with Tab in a real run - slides
     in from the right, game pauses, cursor appears, objectives expand, tab
     handle visible when closed, Tab+ESC close, slide reads well, and the
     thruster/RCS loops go silent while open.
   - **Playtest VERDICT (owner, 2026-07-24):** looks fine for now; LIKES the
     transparency effect + slide animation (keep them). One REQUEST: the open
     drawer must render ON TOP of the flight HUD (the compact objectives text
     currently draws over it) - filed as 20260724-121541. Owner wants to see the
     whole drawer family before deeper feedback.
2b. **20260724-121541** (p68, SIZE S, bug/ui/hud) Tab drawer must render on top
   of the flight HUD (z-order). **CLOSED / LANDED 2026-07-24** (b8de3a30; 1
   in-session review round, trivial diff). Backdrop `GlobalZIndex(10)` + panel
   `GlobalZIndex(11)` (the pause overlay's modal tier); tab handle stays at HUD z.
   - Manual acceptance (batched to owner): the open drawer panel sits ON TOP of
     the compact objectives panel + the rest of the flight HUD.
3. **20260721-211520** (p60, SIZE M, feature/hud/ui) Diegetic objective
   presentation: big on the cockpit HUD, then tucks into the right tab. **CLOSED /
   LANDED 2026-07-24** (e6dbf78d; 1 review round, out-of-context APPROVE). A new
   objective's big rotated card grows in, holds ~2s, tucks into the shell's
   `DrawerTabAnchor` and despawns; replaced the gold posting ghost (completions
   keep green). Placement via the `screen_indicator` px pattern; default clock
   (plays unpaused). Paired with 20260721-211506 (CLOSED).
   - Manual acceptance (batched to owner): a new objective appears large + slightly
     rotated, holds ~2-3s, tucks into the drawer tab handle and vanishes; reads
     well and lands into the tab.

**Playtest REWORK (owner, 2026-07-24)** - after seeing shell + reveal + z-order
in a real run, the owner reshaped the drawer family. Verdicts + the new/rescoped
tasks (see the grooming-history block below for the full feedback):

4. **20260724-134312** (p66, feature/ui/hud) Flight objective HUD rework.
   **CLOSED / LANDED 2026-07-24** (c4693c3a; 1 review round, out-of-context
   APPROVE, zero findings). Removed the compact objectives panel + the drawer
   tab-handle square; new minimalist top-right hint (glyph + count + TAB), which
   now publishes `DrawerTabAnchor`; reveal retuned smaller (tucks up-and-right);
   gamepad open on `RightThumb`. Covers playtest feedback (1)-(4).
   - Manual acceptance (batched to owner): the top-right hint reads minimal (glyph
     + count + TAB) and the old compact panel/square are gone; the reveal is
     smaller and slides up-and-right into the hint; the right-stick click opens
     the drawer.
5. **20260724-134335** (p62, feature/ui/hud) Drawer open: HIDE the flight HUD +
   BLUR the gray background (a present-but-dimmed HUD hurts readability); keep the
   top status bar + the lower-left keybind hints visible; both side panels slide
   in (right + left). **CLOSED / LANDED 2026-07-25.** This was the first
   post-playtest readability reshape. Later feedback supersedes the permanent
   two-panel model with the one-screen NOVA OS monitor; keep its useful behavior
   (ordinary flight HUD yields to the drawer) but do not plan new work around
   left/right/center drawer slots.
6. **20260721-211526** (p55, feature/hud/ui) Comms messages: stacking, skip,
   speaker icons, dismiss. Grows 20260717-163033 (CLOSED). Its icons + StoryFeed
   feed the left panel below.
7. **20260724-134350** (p54, feature/ui/hud) Drawer RIGHT panel: objectives as a
   styled LIST (not the plain text placeholder the shell shipped). NEW.
8. **20260724-102309** (p50, spike/feature/ui/hud) Drawer LEFT panel: comms/chat
   history + a curated flight-log EVENTS journal (nova_probe-style but in-game,
   important events only); slides from the left; must not overlap the lower-left
   keys. **CLOSED / LANDED 2026-07-25.** The data model remains useful, but the
   terminal rework should render this as `log` command output in NOVA OS rather
   than as a permanent left panel.
9. **20260725-104330** (p0, feedback/ui/ux) Terminal drawer feedback epic +
   SPIKE.md. **OPEN.** Supersedes the remaining drawer layout direction after
   playtest: one inset NOVA OS monitor, terminal scrollback, app takeover,
   CRT/green-phosphor visual target, and standalone visual PoC at
   `examples/ui/nova_os_terminal_poc.html`. This is the planning source for the
   next drawer chores/tasks.
10. **20260724-102320** (p30, STRETCH, spike/feature/ui/hud) NOVA OS `map` app:
   the 3D minimap still belongs in v0.9.0 as a stretch placeholder, but it is now
   launched by the terminal and takes over the same monitor. No center panel and
   no multiple drawers. **Cut first if Strand C runs long.** Depends on the NOVA
   OS shell/runtime planning that follows 20260725-104330.

**Strand C fidelity passes (added 2026-07-26, PoC fidelity review)** - the core
terminal (shell 115320, input 115324, output 115330, app runtime 115334, HTML
fidelity 180807, scanline realism 193155) is CLOSED; these close the remaining
gap to `examples/ui/nova_os_terminal_poc.html`:

11. **20260726-193219** (p45, feature/ui/hud) CRT casing + glass depth pass -
   rounded corners, glass sheen, bevels, screws/vents, PLUS the chin bar with
   the recessed NovaCRT 9000 brand plate bottom-left (scope extended from the
   PoC review). Was an unslotted p0 spike option; now a slotted release task.
   Pure chrome outside the tube - no interaction with the RTT pipeline.
12. **20260726-193233** (p44, feature/ui/hud) CRT render-to-texture pipeline:
   real text bloom + crisp tube curvature + the power-on/off, degauss and
   micro-effect inventory. The load-bearing architecture change; it SUPERSEDES
   the overlay CRT shader, so it runs BEFORE the chin controls wire any knob
   to a shader uniform (re-slot rationale in the 2026-07-26 grooming entries).
13. **20260726-214617** (p43, feature/ui/hud) Chin controls: working
   BRIGHT/SCAN knobs + SND/PWR buttons. Depends on 193219 for geometry and
   193233 for the sampling shader the BRIGHT/SCAN uniforms live on.
14. **20260726-214639** (p42, feature/ui/hud/audio) NOVA OS sound: terminal
   SFX (keys, enter, error, beeps, degauss, power sweeps) + the ambient CRT
   bed, through the existing `UiSfx`/`NovaAudioPlugin` conventions.
   Independent - can run in parallel with any of the above.
15. **20260726-214708** (p41, feature/ui/hud/input) Terminal UX parity:
   staggered boot banner, unread-events line, Tab match cycling,
   PageUp/PageDown paging, block caret, contextual footer hints, app-exit
   chords (Ctrl+C / Shift+Esc), and parser support for arguments + multi-word
   launch words (unblocks `repair <part>` / `ship view`). Independent -
   content-side shell work, unaffected by where the content renders.

### Strand A - Combat readability

4. **20260723-233446** (p70, SIZE S, hud/gameplay) HUD allegiance marker over
   ships: a small friendly/enemy triangle/chevron above each entity. Independent
   - can start immediately in parallel with the C spike. From the ch5 playtest
   20260723-182855 (CLOSED). Watch fps (cross-refs the deferred perf task
   20260723-233453).

### Strand D - Scenario browsing

5. **20260723-095951** (p65, SIZE M, menu/scenario/ui/modding/feature) Scenarios
   tab: collapsible campaign headers + campaign->scenario mapping
   (replayability). Independent of the cockpit work. Supersedes the interim
   inline-prefix style (20260723-095930, CLOSED). Step 1 wants a small
   DECISION.md.

### Strand B - Kill / critical-damage model (STRETCH)

6. **20260722-092320** (p40, SIZE M-L, gameplay/feature) Critical-damage state:
   a ship is combat-dead when its weapons + thrusters are destroyed (hull
   notwithstanding), for AI and the player. Now also owns the kill-condition
   rethink merged from 20260722-092326 (CLOSED). STRETCH - cut first if Strand C
   runs long. Integrates with the outcome system; watch
   `outcome-is-last-write-wins-close-the-act` (LESSONS).

## Out of scope (backlog / deferred / dropped)

- **20260715-220011** per-scenario thumbnail art - Goal D is "just the improved
  picker"; art stays backlog.
- **The rest of the ch5/gravity Goal-A cluster:** AI gravity-well handling
  (20260723-224003) CLOSED as wontdo; **20260723-233500** (restore bigger
  planetoid wells) CLOSED - premise removed, its intent folds into a future
  campaign-polish pass once the AI is improved; ch5 perf profiling
  (20260723-233453) can wait (backlog).
- The modding/content-kind spikes (20260714-081703 in-editor scenario builder,
  20260714-134115 ship-prototype content kind, 20260708-162010 piccolo VM),
  input/UX work (20260710-231927 keybind icons, 20260714-001140 gamepad/mobile),
  HUD polish (20260709-164608 widget promotion, 20260717-003620 hull-integrity
  chip), 20260714-214329 web fonts, tooling (20260719-004908 CI nightly pin,
  20260714-081710 bevy_capture) - all stay backlog; no pull this release.
- **20260724-082856** frontend app image refresh - web-content, not a v0.9.0
  game feature; backlog (consolidated the closed devlog-thumbnail and wiki-shot
  tasks).
- **Drawer ship-status/damage section (20260724-102332)** - deferred to backlog
  by the Tab spike (20260721-211512): it overlaps STRETCH Strand B
  (critical-damage, 20260722-092320) and would risk double-work. Slots into the
  drawer's section framework whenever pulled. (The 3D minimap, also "core" in the
  2026-07-21 questionnaire, was pulled INTO v0.9.0 as a stretch item - see Strand
  C item 6.)

## Planning - gate package (pending owner OK)

This tracker started as the 2026-07-24 grooming deliverable, but the drawer
direction changed after playtest and the feedback spike in `20260725-104330`.
The flow planning split now exists and is waiting at the owner gate:

- Core `20260726-115320`: one inset NOVA OS monitor shell and visual treatment.
- Core `20260726-115324`: terminal input, prompt editing, autocomplete, history
  and command registry plumbing.
- Core `20260726-115330`: useful read-only terminal output commands.
- Core `20260726-115334`: app runtime for terminal-launched GUI surfaces.
- Stretch `20260724-102320`: `map` app. Cut first if the core needs to tighten.
- Stretch `20260726-115339`: `ship viewer` app and safe section actions. Cut
  before `map` if only one stretch app fits.

No implementation work should start until `20260725-104330` is approved at the
flow gate.

Note: this project's release convention is a `v0.9.0, release, meta` tracker
task (this file), NOT flow's GOAL.md - the v0.8.0 tracker set the precedent.

## Release cut plan - v0.9.0

Date decision: cut metadata uses `2026-08-01`, the current release date. The
`DATE: 2026-07-24` tracker field remains the original scope/grooming date.

Branch decision: release edits happen on `master`, matching
`web/src/wiki/dev/development.md`. Do not use a sprout worktree for the final
version/tag cut. Confirm with `git branch --show-current` immediately before
staging or tagging.

Tag boundary: create the local `v0.9.0` tag only after the release commit if
the user asks to proceed past planning. Do not push `master` or the tag in this
flow unless the user explicitly asks later.

## Steps

1. Confirm release preconditions:
   - `git status --short --branch`
   - `git branch --show-current`
   - `find docs -maxdepth 2 -type f | sort` shows only `docs/README.md`
   - `scripts/check-docs-clean.sh`
   - confirm no non-DONE `v0.9.0` story blocks the cut, ignoring backlog tasks
     already recorded as out of scope
2. Update versions:
   - edit root `Cargo.toml` `[workspace.package] version` from `0.8.1` to
     `0.9.0`
   - run `nix develop --command cargo metadata --format-version 1 >/dev/null`
     to refresh `Cargo.lock`
   - grep exact old release strings after the bump:
     `rg -n '"0\.8\.1"|v0\.8\.1|0\.8\.1' Cargo.toml Cargo.lock crates web/src CHANGELOG.md`
     and keep intentional dated/history hits only
3. Update `CHANGELOG.md`:
   - leave a fresh empty `## [Unreleased]`
   - promote current Unreleased content to `## [0.9.0] - 2026-08-01`
   - keep subsystem headings concise and merge duplicate headings if any exist
   - update compare links:
     `[unreleased]` -> `v0.9.0...HEAD`
     add `[0.9.0]` -> `v0.8.1...v0.9.0`
4. Add the v0.9.0 News post:
   - create `web/src/news/0.9.0.md`
   - source narrative from the final `CHANGELOG.md` section and release tracker,
     not intent
   - cover the headline NOVA OS drawer/terminal, contextual phosphor HUD,
     scenario campaign headers, neutralized ships, allegiance markers, web/site
     skin sync, preload/assets/licensing, and probe/profile sandboxing
   - include `.figure` placeholders for screenshots to capture later
   - include closing `## Point releases`
5. Wire the News post:
   - add newest-first `NEWS_POSTS` entry in `web/webpack.config.js`
   - add newest-first card in `web/src/news.html`
   - update `scripts/shoot-web-pages.sh` `news-post|/news/0.8.0/` to
     `news-post|/news/0.9.0/` so the representative news-post screenshot covers
     the current release
6. Re-read edited artifacts:
   - `Cargo.toml`
   - `CHANGELOG.md`
   - `web/src/news/0.9.0.md`
   - `web/webpack.config.js`
   - `web/src/news.html`
   - `scripts/shoot-web-pages.sh`
7. Verify:
   - `nix develop --command cargo fmt --check`
   - `nix develop --command cargo check`
   - `nix develop --command cargo run -p nova_assets --bin content -- lint`
   - `cd web && npm run ci` inside the Nix dev shell
   - optional visual proof if time permits: `nix develop --command scripts/shoot-web-pages.sh target/web-shots-v0.9.0`, then open the news-index and news-post screenshots
   - do not rerun full `cargo test` unless requested; user reported all tests
     green on master before this release cut
8. Record and commit:
   - update this task Notes with exact proof commands and outcomes
   - run `tatr check --ledger LESSONS.md`
   - stage explicit paths only:
     `Cargo.toml Cargo.lock CHANGELOG.md tasks/20260724-083631/TASK.md web/webpack.config.js web/src/news.html web/src/news/0.9.0.md scripts/shoot-web-pages.sh`
   - commit with `chore(release): v0.9.0`
   - if proceeding to local tag, run `git tag v0.9.0`; do not push it

## Definition of Done

- Workspace and lockfile carry version `0.9.0`. (cmd:
  `rg -n '^version = "0\.9\.0"$' Cargo.toml` and
  `rg -n 'name = "nova-protocol"|version = "0\.9\.0"' Cargo.lock`)
- Changelog has a fresh empty Unreleased section, a dated `0.9.0` section, and
  correct compare links. (cmd:
  `rg -n '## \[Unreleased\]|## \[0\.9\.0\] - 2026-08-01|\[unreleased\]: .*v0\.9\.0\.\.\.HEAD|\[0\.9\.0\]: .*v0\.8\.1\.\.\.v0\.9\.0' CHANGELOG.md`)
- News has a v0.9.0 post, webpack registration, news index card, and screenshot
  route. (cmd:
  `test -f web/src/news/0.9.0.md && rg -n 'slug: "0\.9\.0"|news/0\.9\.0/|news-post\|/news/0\.9\.0/' web/webpack.config.js web/src/news.html scripts/shoot-web-pages.sh`)
- `docs/` remains release-clean. (cmd: `scripts/check-docs-clean.sh`)
- Notes record every verification command from Step 7 with pass/fail output.
  (manual: inspect `## Notes`)
- Release commit exists on `master`. (cmd: `git log -1 --oneline --decorate`)
- If the user approves local tagging, `v0.9.0` exists and is unpushed.
  (manual: `git tag --list v0.9.0` shows `v0.9.0` and no push is performed)

## Notes

2026-08-01 release cut:

- Changed workspace version to `0.9.0`; refreshed `Cargo.lock` with `cargo metadata`.
- Promoted `CHANGELOG.md` Unreleased content to `0.9.0` dated `2026-08-01`.
- Added `web/src/news/0.9.0.md`; wired `NEWS_POSTS`, the news index card and
  the representative screenshot route.
- Expanded the v0.9.0 feature post from 1,192 to 3,342 words after comparison
  with v0.8.0's 2,751-word post. Added shipped detail for the terminal command
  tree, map and ship apps, CRT interaction, contextual HUD, combat outcomes,
  campaign content, interface skins, startup, web verification, asset loading
  and probe isolation; increased screenshot placeholders from three to five.
- `git status --short --branch`: clean before edits except `master` ahead of
  `origin/master` by one existing commit.
- `git branch --show-current`: `master`.
- `find docs -maxdepth 2 -type f | sort`: only `docs/README.md`.
- `scripts/check-docs-clean.sh`: pass, `docs/` is release-clean.
- `tatr ls -f '(:tags contains v0.9.0) and (:status eq OPEN)' --sort priority`:
  only this release tracker remains open.
- `nix develop --command cargo metadata --format-version 1 >/dev/null`: pass.
- `rg -n '"0\.8\.1"|v0\.8\.1|0\.8\.1' Cargo.toml Cargo.lock crates web/src CHANGELOG.md`:
  no stale workspace crate versions; remaining hits are dated changelog/history
  links, the v0.8.1 point-release news section and unrelated dependency
  versions in `Cargo.lock`.
- `rg -n '^version = "0\.9\.0"$' Cargo.toml`: pass.
- `rg -n 'name = "nova-protocol"|version = "0\.9\.0"' Cargo.lock`: pass for
  workspace packages.
- `test -f web/src/news/0.9.0.md && rg -n 'slug: "0\.9\.0"|news/0\.9\.0/|news-post\|/news/0\.9\.0/' web/webpack.config.js web/src/news.html scripts/shoot-web-pages.sh`:
  pass.
- `nix develop --command cargo fmt --check`: pass.
- `nix develop --command cargo check`: pass; existing future-incompat warnings
  remain in `nova_gameplay` map/ship prelude exports and `proc-macro-error2`.
- `nix develop --command cargo run -p nova_assets --bin content -- lint`: pass,
  `0 error(s), 0 warning(s), 0 finding(s), 14 scenario(s) balance-audited, 1 acked`.
- `nix develop --command bash -lc 'cd web && npm run ci'`: pass; format,
  eslint, site/theme tests and webpack build succeeded.
- `nix develop --command scripts/shoot-web-pages.sh target/web-shots-v0.9.0`:
  pass; captured 12 screenshots. Manual inspection of news index and v0.9.0
  post desktop/mobile shots: coherent layout, top card/post rendered.
- After the expanded news rewrite, reran
  `nix develop --command bash -lc 'cd web && npm run ci'`: pass; format,
  eslint, site/theme tests and webpack build succeeded.
- `nix develop --command scripts/shoot-web-pages.sh target/web-shots-v0.9.0-news-expanded`:
  pass; captured 12 screenshots. Re-inspected the news index and expanded
  v0.9.0 post at desktop/mobile widths: coherent layout, no overlap or clipped
  text, and figure placeholders render correctly.
- `tatr check --ledger LESSONS.md`: blocked by five existing
  `promotion-awaiting-decision` findings: `split-tests-hoist-the-shared-fixture`,
  `split-must-re-export-not-repoint`, `conserve-on-regroup`,
  `comment-pass-as-asserted-replacements` and
  `visibility-sweep-narrows-back`. Owner chose `DEFER` for all five during the
  release close; recorded with `tatr ledger` and reran the check successfully.
- Release child `20260729-211150` has `STATUS: CLOSED` but remains at
  `FLOW STEP: COMPOUNDING`; all other v0.9.0 children are `DONE`.
- Full `cargo test` skipped per release plan; user had reported all tests green
  on master before the cut.
- Local `v0.9.0` tag not created; task requires a later explicit user approval.

## Grooming history

- **2026-07-26 (RTT re-slot):** owner questioned the fidelity-pass ordering:
  does the render-to-texture pipeline (193233) need to come FIRST to make the
  PoC's CSS/HTML effects possible? Assessment: mostly no - casing/sound/UX are
  UI-node and logic work independent of where the content renders - but 193233
  SUPERSEDES the overlay CRT shader, so wiring the chin knobs (BRIGHT/SCAN) to
  overlay uniforms first would be throwaway by design, and a true >1.0 BRIGHT
  multiply is only exact against a sampled texture. Per the no-time-pressure
  technical-decision rule, the well-designed order wins: 193233 re-slotted
  p41 -> p44 (right after the casing chrome, before any shader-touching
  control work); chin 214617 -> p43 (now also depends on 193233), sound
  214639 -> p42, shell UX 214708 -> p41. Strand C items 11-15 renumbered.
- **2026-07-26 (PoC fidelity review):** owner asked for a review of the shipped
  NOVA OS (`crates/nova_gameplay/src/hud/drawer.rs`) against
  `examples/ui/nova_os_terminal_poc.html` and a task restructure so the PoC
  actually makes it INTO the game. Gap found: the terminal core (commands,
  input, palette, shader scanlines) is at parity, but the PHYSICAL layer is
  not - no rounded/moulded casing or glass, no chin bar with the NovaCRT 9000
  brand plate (logo bottom-left), no working BRIGHT/SCAN/SND/PWR controls, no
  sound at all, and the shell lacks the PoC's feel details (staggered boot,
  unread-events line, Tab match cycling, paging, block caret, contextual
  hints, app-exit chords, argument parsing). Actions: re-slotted the two
  unslotted p0 CRT passes (193219 -> p45 with chin/plate scope extension,
  193233 -> p41 with the micro-effect inventory); NEW tasks 20260726-214617
  (chin controls, p44), 20260726-214639 (sound, p43), 20260726-214708
  (terminal UX parity, p42) - Strand C items 11-15 above; enriched the two
  stretch apps (102320 map, 115339 ship viewer) with PoC-derived requirement
  sections (contact readout, severity styling, disabled-action explanations,
  number-key actions, applast chrome line, launch-word decision).
- **2026-07-24 (drawer-family playtest REWORK):** owner played shell (102304) +
  reveal (211520) + z-order (121541) together and reshaped the drawer. Verdicts &
  requests, filed as tasks (Strand C items 4-9 above):
  1. The old always-on compact objectives text (top-right) is still there -> REMOVE
     it (objectives live in the drawer + reveal now). (134312)
  2. Dislikes the "drawer square" (tab handle) on the right during play -> remove,
     or at most a tiny "Tab" hint. (134312)
  3. PREFERRED: a minimalist top-right status-bar notification ("objectives" etc.),
     terse, hinting Tab + a gamepad alternative. (134312)
  4. The reveal is too big + too centered -> a bit SMALLER, and the vanish should
     translate toward the RIGHT (into that notification). (134312)
  5. Drawer opens as TWO side panels: RIGHT = objectives as a prettier LIST (rework
     from plain text) sliding from the right (134350); LEFT = chat history + events
     "flight-log journal" (nova_probe-style, in-game, important events) sliding from
     the left (102309).
  6. Background: keep the gray transparent dim (liked) and ADD blur; HIDE the
     flight UI in drawer mode so the old UI does not fight readability. (134335)
  7. The top STATUS BAR (readout strip) stays visible - reserve space, no drawer UI
     on it (like a WM status bar). The lower-left keybind buttons must NOT be
     overlapped by the left panel - keep keys visible. (134335)
  8. CENTER of the drawer = the 3D minimap; this sprint a placeholder: downsized 3D
     map view with WASD + placeholder markers; zoom/planning later. (102320,
     rescoped)
- **2026-07-26 (terminal drawer feedback spike, 20260725-104330):** after seeing
  the drawer family in game, owner re-cut the remaining direction. Supersedes
  the permanent two-panel/center-slot model with one NOVA OS monitor. Tab opens
  the drawer from flight, then becomes autocomplete inside the terminal; Escape
  closes the drawer; apps use their own exit control/chord. Visual target is the
  standalone PoC `examples/ui/nova_os_terminal_poc.html`: full-main monitor fill,
  dark blue-black casing, green phosphor screen, orange/yellow accents, scanlines
  and diagnostic FPS/version. The 3D minimap remains stretch as the `map` app,
  launched from the terminal.
- **2026-07-24 (Tab drawer spike, 20260721-211512 CLOSED):** SPIKE.md fixed the
  drawer's v0.9.0 contents to objectives + comms log (both render data that
  already exists: `GameObjectives`, `StoryFeed`), deferring the 3D minimap
  (20260724-102320) and ship-status (20260724-102332) to backlog per the
  "cannot balloon" directive. Interaction model settled: Tab keybind, a new
  `PauseStates::Drawer` sim-gate variant reusing the 211500 cursor/freeze hooks,
  slide-in via bcs TweenPlugin, tab-handle screen anchor as 211520's diegetic
  tuck-target. Seeded the shell (20260724-102304, gate) and comms-log
  (20260724-102309) tasks; Strand C reordered above.
- **2026-07-24 (drawer shell planned, 20260724-102304):** /plan authored Steps +
  DoD. DECISION.md tasks/20260724-102304/DECISION.md (ACCEPTED): the Tab drawer
  is a THIRD `PauseStates` variant (not a separate freeze state) - keeps one
  clock-freeze axis and avoids the `unpause_clocks` stomp. Audit surfaced 19
  observer `== Paused` self-guards to widen to `!= Unpaused` (set-gates-miss-
  observers); that guard sweep is the bulk of the task. Awaiting the flow gate.
- **2026-07-24 (owner contents call, post-spike):** owner pulled the 3D minimap
  (20260724-102320) back INTO v0.9.0 as a STRETCH item - wants it but at the end,
  after the core sections, "curious how it will look"; cut first if Strand C runs
  long (p30, Strand C item 6). Ship-status (20260724-102332) stays deferred to
  backlog (overlaps stretch Strand B).
- **2026-07-24 (planning triage + v0.9.0 groom):** triaged the 28-item backlog.
  Closed as wontdo: 20260723-224003 (AI gravity wells). Consolidated the two
  frontend-image tasks (20260715-092658, 20260715-231500) into new
  20260724-082856. Merged the kill-condition pair (092326 -> 092320). Assembled
  this v0.9.0 set from Goals C/D/A + B-stretch, retagged and prioritized.
  Also closed (owner call, same session): 20260712-133356 (alt-fire, not
  pursuing), 20260719-112245 (golden-timeline compare, superseded by invariant
  assertions 20260719-114931), 20260525-133031 (bcs public-API docs, wrong
  repo), and 20260723-233500 (restore bigger wells, premise removed - folds
  into a future campaign-polish pass).
- **2026-07-28 (owner scope call, /flow):** v0.9.0 EXTENDED with the UI-rework
  epic 20260728-175719 (Strand E: NOVA OS look everywhere - menus/editor/HUD
  restyle with light-3D widgets, quieter contextual HUD, 1 u = 10 m metric
  display; HTML-demo spike first). Owner chose extending this release over
  opening v0.10.0; v0.9.0 tags only after the epic closes. Children:
  20260728-175726 (spike, p44), 20260728-175731 (units, p42), 20260728-175734
  (theme/widgets, p40), 20260728-175738 (menus+editor, p38), 20260728-175742
  (HUD restyle/text, p36), 20260728-175747 (contextual HUD, p34). The epic
  container holds the strand map and manual-acceptance batch.
- **2026-07-31 (/flow Finish):** epic 20260728-175719 is CLOSED - all 11
  children landed (including the 2026-07-30 feedback wave 122843/122909/122940)
  and the owner accepted DoD 3/4/5 at the Finish gate. The condition gating the
  v0.9.0 tag is met; the release is ready to cut.
