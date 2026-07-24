# v0.9.0 release tracker: Cockpit & Command - ship-computer drawer, combat readability, scenario browsing

- STATUS: OPEN
- PRIORITY: 1
- TAGS: v0.9.0,release,meta

Release-level tracker for v0.9.0. Per-strand work lives in its own tatr task
(tagged `v0.9.0`); this task carries the release theme, strand map, out-of-scope
decisions, and the grooming history - the same shape as the v0.8.0 tracker
(20260720-142428).

- DATE: 2026-07-24
- BASE: master at v0.8.0 (head 70587839)
- THEME: **Cockpit & Command.** Turn the cockpit into a real ship-computer and
  make combat legible. The headline is the Tab ship-computer drawer (objectives,
  comms log, and whatever the spike settles) with its two diegetic feeders -
  on-cockpit objectives and stacking comms - plus at-a-glance combat readability
  (allegiance markers over ships) and better campaign/scenario browsing. Unlike
  v0.8.0 (pure debt paydown, no new features), v0.9.0 is a features release.

## Why this scope

v0.8.0 paid down docs/tooling debt and lengthened the campaigns. The game now
has content worth reading clearly, but the in-flight information surface is thin:
objectives and comms are minimal, you cannot tell friend from foe at a glance in
a busy scene, and the scenario picker is a flat list. This release invests in the
cockpit as an information system and in combat readability, on top of the stable
v0.8.0 base.

The owner's stated focus for this release, in preference order:

1. **Goal C - the cockpit ship-computer drawer (100% in, the headline).** The
   Tab drawer and its two diegetic feeders. A spike fixes its contents first.
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

4. **20260724-134312** (p66, feature/ui/hud) Flight objective HUD: remove the
   always-on compact objectives panel AND the drawer tab-handle square; replace
   with a MINIMALIST top-right status-bar notification (hints Tab + gamepad);
   repoint `DrawerTabAnchor` to it; retune the reveal SMALLER + vanish toward the
   right. NEW. Covers feedback (1) old objective text still there, (2) dislike the
   drawer square, (3) prefer a minimalist status-bar notification, (4) reveal too
   big/centered.
5. **20260724-134335** (p62, feature/ui/hud) Drawer open: HIDE the flight HUD +
   BLUR the gray background (a present-but-dimmed HUD hurts readability); keep the
   top status bar + the lower-left keybind hints visible; both side panels slide
   in (right + left). NEW. The core interaction reshape.
6. **20260721-211526** (p55, feature/hud/ui) Comms messages: stacking, skip,
   speaker icons, dismiss. Grows 20260717-163033 (CLOSED). Its icons + StoryFeed
   feed the left panel below.
7. **20260724-134350** (p54, feature/ui/hud) Drawer RIGHT panel: objectives as a
   styled LIST (not the plain text placeholder the shell shipped). NEW.
8. **20260724-102309** (p50, spike/feature/ui/hud) Drawer LEFT panel: comms/chat
   history + a curated flight-log EVENTS journal (nova_probe-style but in-game,
   important events only); slides from the left; must not overlap the lower-left
   keys. RESCOPED from "comms-log section".
9. **20260724-102320** (p30, STRETCH, spike/feature/ui/hud) Drawer CENTER 3D
   minimap - THIS SPRINT a placeholder: a downsized 3D map view (render-to-texture)
   with WASD camera + placeholder asteroid/ship/enemy markers; zoom + flight
   planning are LATER. RESCOPED to the center placeholder. **Cut first if Strand C
   runs long.** Depends on the shell (102304) + the drawer-open rework (134335).

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

## Planning - next step (pending owner OK)

This tracker + the tagged/estimated task set is the deliverable of the
2026-07-24 grooming session. Still TODO, on the owner's go-ahead:

- **Spike Goal C** (20260721-211512): settle the drawer's v0.9.0 contents and
  interaction model. This is the gate for 211520 + 211526.
- **Define per-task DoD + Steps** for every strand above via `/plan`, each DoD
  item naming its proof (`test:` / `cmd:` / `manual:`), per repo AGENTS.md.
- Decide Strand B's in/out call once Strand C's real size is known.
- Then the flow gate: present the full package for an explicit "build this"
  before any worktree is cut.

Note: this project's release convention is a `v0.9.0, release, meta` tracker
task (this file), NOT flow's GOAL.md - the v0.8.0 tracker set the precedent.

## Definition of done (release-level; filled at planning)

To be authored in the planning pass. Skeleton:

- The cockpit Tab drawer exists and shows the contents the spike fixed;
  objectives present diegetically then tuck into the tab; comms stack, skip and
  dismiss. (proofs per task DoD)
- Friendly/enemy allegiance is readable at a glance over ships in a busy scene.
- The Scenarios tab groups scenarios under collapsible campaign headers.
- (stretch) A ship with no weapons + no thrusters counts as combat-dead.
- Overall: the full check suite passes; gameplay-touching strands probed.

## Grooming history

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
