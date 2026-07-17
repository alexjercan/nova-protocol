# Rework broadside pacing: act-split retry and hardened cover ring

- STATUS: CLOSED
- PRIORITY: 52
- TAGS: spike,v0.7.0,scenario,content,balance

Goal: broadside is "hard but playable" - keep its good bones (light-turret
corvettes, 550u spawns, the gunship's 1177u approach as breathing room) and
fix the two unfair parts: dying to the act-2 gunship re-earns the whole
corvette fight, and the 24-rock cover ring is paper (health 100 =
0.25s of better-turret fire).

Direction notes:
- Act-split retry: corvette act and gunship act as separate hidden
  scenarios chained via NextScenario, so defeat retries the current act.
- Harden part of the cover ring to invulnerable: true so the gunship
  fight has persistent hard cover per attack bearing; keep some
  destructible rocks as chaff.
- Keep the gunship spawn distance and torpedo setpiece; tune only if the
  balance audit rig (tasks/20260717-112656) says the PDC-screen ask is
  beyond the intended skill bar.

Spike: tasks/20260717-111808/SPIKE.md (findings F4/F7)

Verified at plan time: base scenarios are BUILDER-GENERATED
(crates/nova_assets/src/scenario/broadside.rs is the source;
`cargo run -p nova_assets --bin gen_content` writes the RON; the
content_ron_parity test enforces byte parity and that base.bundle.ron
ships exactly the generated set) - all edits happen in the builder, RON
regenerates in the same commit. Broadside has ALREADY evolved past the
spike snapshot: corvettes carry light turrets with patrol + leash 420 at
~550u (good), hauler loss is a soft-fail objective not a defeat, and the
win (gunship down) ends the base story with no queued next. Remaining
spike problems = full-scenario restart on act-2 deaths (F7) and
all-destructible 100hp scatter cover (F4). Consumers to keep green:
crates/nova_assets/tests/broadside_assault.rs,
crates/nova_assets/tests/content_ron_parity.rs, examples/19_broadside.rs,
shakedown's chain-in (file A keeps the `broadside` id).

## Steps

- [x] Split the builder: `broadside()` keeps acts 0-1 (approach + corvette
  ambush) and on both-corvettes-down declares the Victory checkpoint beat
  + NextScenario("broadside_gunship", linger: true); defeat/soft-fail
  gates tighten from act < 3 to act < 2. New
  `broadside_gunship()` (hidden: true): OnStart spawns player, hauler,
  the same scatter + hard cover, and the gunship immediately (its ~1177u
  approach is the act's pacing); gunship down -> Victory, no queued next
  (end of base story); player death -> Defeat + retry broadside_gunship
  (THE checkpoint); hauler death stays a soft-fail objective. Shared
  helpers stay shared; ids/objectives unchanged where they carry over.
- [x] Harden cover: add 5 fixed invulnerable boulders (nominal r3.5-5,
  bodies 3.5x-6x nominal) to BOTH parts: 3 between the hauler and the
  corvette lane (z -520..-575), 2 on the gunship lane (z -700..-750,
  positions pre-derived so each threat corridor keeps >= 2 boulders within
  120u of the hauler->threat axis); keep the 24-rock destructible scatter
  as chaff. No overlap: worst-case 6x bodies clear each other, the
  hauler, spawns and the scatter box (box ends at z -430).
- [x] Register + regenerate: add broadside_gunship to build_scenarios()
  (lib.rs:83) and "scenarios/broadside_gunship.content.ron" to
  base.bundle.ron; run gen_content; parity + bundle-uniformity tests
  green.
- [x] Update crates/nova_assets/tests/broadside_assault.rs for the split:
  file A walk ends in Victory + queued broadside_gunship (was: gunship
  spawn), file B walk covers gunship victory with NO queued next, each
  part's player-death Defeat requeues ITSELF, post-win deaths declare
  nothing; add geometry pins for the new boulders (corridor >= 2 per
  threat lane, 6x no-overlap, station clearance) mirroring
  ledger_ch2_encounter.rs helpers.
- [x] Update examples/19_broadside.rs for the split (read it first; keep
  its walkthrough working against part A, extend or duplicate for part B
  as its structure suggests).
- [x] Docs: CHANGELOG (Scenarios & Objectives); grep the wiki for
  broadside scenario descriptions (scenarios.md) and sweep; NOTES.md
  design record with the boulder geometry numbers and the
  builder-generated workflow note.
- [x] Verify: gen_content idempotent (run twice, git diff clean);
  cargo test -p nova_assets --test content_ron_parity --test
  broadside_assault --test ledger_ch2_encounter (one filter each, separate
  runs); content_lint; the 19_broadside example compiles
  (cargo check --workspace --all-targets); cargo fmt. Full suite on CI.

## Close-out record

All seven steps landed; details, decisions and the verification runs are
in NOTES.md. Highlights: the chapter is two builder-generated scenarios
with the corvette win as checkpoint; five invulnerable boulders anchor
both threat lanes with computed geometry pins; the 19_broadside example
now rides Continue through the checkpoint; a stale infinite-ammo doc
comment was corrected in passing. Verification: gen_content idempotent,
content_ron_parity 2/2, broadside_assault 10/10, ledger_ch2_encounter
12/12, content_lint clean, workspace --all-targets green, fmt run last.
Full suite on CI per standing instruction.

Reflection: checking generated-vs-authored FIRST (the parity test names
the builder as the single source) prevented the classic hand-edit-the-RON
mistake before it happened; the ledger_ch2 cycle's geometry-pin pattern
transferred almost verbatim, which is what made this cycle fast.
