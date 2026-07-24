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
   drawer - objectives, comms log, 3D minimap, what else. GATES the two features
   below; must FIX the drawer's v0.9.0 contents (recommend: objectives + comms
   log in, defer the 3D minimap) so the release cannot balloon. Depends on the
   cursor machinery from 20260721-211500 (CLOSED).
2. **20260721-211520** (p60, SIZE M, feature/hud/ui) Diegetic objective
   presentation: big on the cockpit HUD, then tucks into the right tab. Depends
   on the spike (211512). Pairs with 20260721-211506 (CLOSED).
3. **20260721-211526** (p55, SIZE M, feature/hud/ui) Comms messages:
   notification-style stacking, skip control, speaker icons, dismiss. Depends on
   the spike (211512) for the log view. Grows 20260717-163033 (CLOSED).

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
