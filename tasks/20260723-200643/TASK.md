# ch5 raid tuning: shrink planetoid gravity/SOI, base holds station, fewer turrets, torpedo on R, unhide (content)

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.8.0, content, scenario, playtest

## Story

Umbrella 20260723-200636. Playtest tuning of `ledger_ch5_the_raid.content.ron`
(the reward raid from task 20260723-182855). Pure content. See GOAL.md for the
engine root-cause analysis (gravity SOI = 8*radius; only piloted ships feel
gravity; armed AI Engages by chasing).

## Steps

- [x] Shrunk the three planetoids so gravity is MILD and the field is calmer:
      planetoid_1 r22->14 g6->3 (-160,-40,-140); planetoid_2 r18->12 g5->3
      (150,50,-300); planetoid_3 r26->16 g7->3 (0,-70,-470). Verified base-to-
      planetoid: p1 416u (SOI 112, clear), p2 269u (SOI 96, clear), p3 99u (SOI
      128 - base in its MILD outer well, accel ~0.08 u/s^2 vs old ~0.35).
- [x] Base holds station via RCS + tight leash: added two `basic_thruster_section`
      (rcs_xp, rcs_xm on the x-arm tips), kept the controller core + AI, set
      `leash: Some(15.0)`. Sits at ~610u from the player in mild gravity - RCS
      holds it, the tether stops it chasing. Lint validated the thruster mounts.
- [x] Turrets 4 -> 2: kept turret_zp/turret_zm (spine top), removed the two
      x-arm turrets (their tips now carry the RCS thrusters instead).
- [x] Torpedoes rebind: the two tube cubes now `Keyboard(KeyR)` +
      `Gamepad(RightTrigger2)`; briefing text updated to "the R key". Lint clean
      - no flight-rig key conflict on R.
- [x] `hidden: true` -> `hidden: false` (with a RE-HIDE-before-release comment).
- [x] ch5 rig: the torpedo test now asserts the tubes bind R (round-trip via
      `BindingInput::try_from` -> `Keyboard(KeyCode::KeyR)`); added
      `the_base_holds_station_with_rcs_and_a_tight_leash` (2 turrets, >=2
      thrusters, leash == Some(15.0)) and `the_raid_is_launchable_for_testing`
      (not hidden); fixed the bundle-version pin 1.10.0 -> 1.11.0.
- [x] Docs: bundle 1.10.0 -> 1.11.0; CHANGELOG 1.11.0 entry; README + news
      torpedo-control "right"/"right mouse" -> "R key"; mod-guide walk -> 1.11.0.
- [x] Lint (`--target webmods/the-ledger`) 0 err/warn/finding; `webmods_validation`
      loads ch5; ledger_ch5_raid 11/11, ledger_ch4_ending 10/10; fmt clean.

## Definition of Done

- cmd: `cargo run -p nova_assets --bin content -- lint --target webmods/the-ledger`
  is clean (no key-conflict WARN, turret mounts still valid); `cargo test -p
  nova_assets --test webmods_validation` loads ch5.
- test: `cargo test -p nova_assets --test ledger_ch5_raid` green with the updated
  torpedo-key / turret-count / not-hidden assertions.
- cmd: `cargo fmt --check -p nova_assets` clean.
- manual: playtest ch5 - calmer gravity for the small ships, the base holds its
  position, torpedoes on R, 2 turrets on the base, launches from the picker.

## Notes

- Do NOT run the full local suite / clippy (memory: skip-local-tests-and-clippy).
- `Keyboard(KeyR)` RON: `BindingInput::Keyboard(KeyCode)`; KeyCode serializes by
  variant name, so `Keyboard(KeyR)`. If the lint/parse rejects that token, check
  the exact KeyCode RON repr and fix.
- Bump to 1.11.0 (1.10.0 was landed but each portal version is immutable; a tune
  is a new version).

## Outcome (2026-07-23)

CLOSED. Playtest tuning of ch5, pure content. Grounded every change in the
engine (gravity SOI = 8*radius, only piloted ships feel gravity, armed AI
Engages by chasing) rather than guessing.

**What changed and why.** (1) Planetoid gravity was far too strong and wide
(radius 26 + gravity 7 -> a 208u well the base sat 116u inside, with no thrusters
to resist -> dragged off). Shrank all three planetoids (radius/gravity down) so
the field is gentle for the small ships and the base sits in only a MILD residual
well. (2) The base now holds station via RCS + a tight leash - the user asked to
"place it such that RCS would work, safe distance." It gained two thruster
sections (RCS authority) and `leash: Some(15.0)`; at ~610u from the player in
mild gravity, RCS holds it and the AILeash tether stops it chasing when it
engages. (3) Turrets trimmed 4 -> 2. (4) Torpedoes moved to the R key
(`Keyboard(KeyR)`), freeing the mouse. (5) `hidden: false` for testing (flagged
to re-hide before release).

**Design note (why NOT thrusterless).** An earlier plan was thrusterless base +
out-of-gravity (so it can't chase and never drifts). The user preferred the RCS
approach, which is viable because `AILeash` centers on the spawn and breaks off
combat beyond its radius - a 15u leash bounds the engage-chase to a few units. So
the base gets real RCS station-keeping against real (mild) gravity, held near its
post by the tether. The RCS-hold vs actual-drift is a playtest question the user
will confirm; if it still drifts, the levers are: lower planetoid_3 gravity
further, tighten the leash, or move the base fully out of the well.

**Difficulties.** The lint (my `lint-is-the-fast-oracle` lesson) again earned its
keep: it validated the new thruster mounts and confirmed R is not a reserved
flight key in one pass. One in-file fixture pin broke - the ch5 rig asserted the
bundle version "1.10.0"; the bump to 1.11.0 tripped it (fixture-pin-in-same-file,
same class as the ch4-rig pin last cycle). Fixed to 1.11.0.

**Verification.** lint 0 err/warn/finding (6 scenarios balance-audited);
`webmods_validation` loads ch5; `ledger_ch5_raid` 11/11, `ledger_ch4_ending`
10/10; `cargo fmt --check -p nova_assets` clean.

**Self-reflection.** Investigating the engine's gravity SOI + AI-engage-chase
model BEFORE writing anything is what made this correct - the "add RCS thrusters"
instinct would have made an armed base fly at the player without the leash
insight. Worth remembering: when a fix touches AI + physics, read the behavior
state machine first. Next time, grep the rig for a version-string pin in the same
edit as any bundle bump (it has now bitten twice).

Manual playtest stays pending for the umbrella Finish (the user is set up to test
it directly now that it is un-hidden).
