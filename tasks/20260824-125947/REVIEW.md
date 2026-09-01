# Review: Spinal railgun lance

- TASK: 20260824-125947
- RANGE: d6d311e3..68a2cb38 (5 commits)
- BRANCH: master

## Round 1

- REVIEWER: craft, performance, correctness, contracts, red team, feel
- VERDICT: REQUEST CHANGES (six findings), all fixed in 59048768, 04c8b653
  and c07eaa1e.

The lance was wired correctly into its OWN systems and missing from every
shared one. Five of the six findings are the same shape: a query, a match or
a table that enumerated turret and torpedo sections and was not widened when
a third weapon kind shipped.

Findings:

- [x] R1.1 (BLOCKER) `crates/nova_ship/src/input/targeting/safety.rs` - the weapons-safety
  interrupt zeroed `TurretSectionInput` and `TorpedoSectionInput` and left
  `RailgunSectionInput` latched. A held R survived the whole cold window and
  committed an unabortable shot on the first hot frame. Fixed in 59048768.
- [x] R1.2 (MAJOR) `crates/nova_gameplay/src/integrity/neutralize.rs` - a hull that kept a working lance was called
  disarmed and stood down, and a lance-only hull could never be neutralized at
  all. The editor sandbox's picket carries one. Fixed in 59048768.
- [x] R1.3 (MAJOR) `crates/nova_ship/src/input/ai/railgun.rs` - the AI commit
  was a one-frame `Update` pulse read a schedule later in `FixedUpdate`. Above
  the 64 Hz fixed rate most commits were cleared before any step saw them, so
  each burned the full fourteen-second cadence for a shot that never left. It
  holds the trigger now and burns the cadence on the shot, which is the bay's
  rule. Fixed in 59048768.
- [x] R1.4 (MAJOR) `crates/nova_channel/src/apply.rs`, `crates/nova_os_ui/src/ship/rebind.rs`, `crates/nova_menu/src/settings.rs` - the settings conflict guard, the NOVA OS readout
  and rebind, and the channel resolver all stopped at three section kinds, so a
  flight verb could be bound onto a live lance's trigger. Fixed in 59048768.
- [x] R1.5 (MAJOR) `crates/nova_ship/src/input/targeting/contacts.rs` - the
  combat-lock decay counted a held trigger on turret and torpedo sections only.
  A ship whose only gun is the lance read as idle while it was firing; thirty
  seconds in the lock let go and weapons safety came down mid-fight, with
  nothing on screen to say why. That is the exact bug the held-trigger case was
  widened to fix, reintroduced by a weapon the query did not know about. Fixed
  in 04c8b653 with `a_committed_lance_resets_the_decay_the_same_way_a_held_trigger_does`.
- [x] R1.6 (MINOR) `crates/nova_ship/src/sections/railgun_section/render.rs` - a lance disabled mid-charge kept its bore lit
  forever, and the slug carried no tracer. Fixed in 59048768.
- [x] R1.7 (MINOR) documentation - `docs/sections.md` had no `Railgun` row, the
  creator reference said five section kinds and had no railgun chapter, the
  editor palette table had no `railgun_lance_section` row, and the closed
  animation-cue set omitted `Charge`. Fixed in c07eaa1e.

Considered and NOT raised:

- The 24 sequential raycasts in `sync_bore_sight` break on the first miss and
  the loop documents why. Not a finding.
- `terminal/input.rs` looked like a fourth missing-kind site, but
  `SectionClass::Railgun` exists and short-circuits the marker fallback.

Verified (not taken on trust):

- `cargo check -p nova_ship -p nova_menu -p nova_editor -p nova_scenario
  -p nova_assets -p nova_authoring --all-targets --features debug`: clean.
- `cargo fmt --all -- --check`: clean.
- The R1.5 fix proved by its own test, which fails on the pre-fix query.
- `content lint`: 0 errors, 0 warnings, 13 scenarios balance-audited, 1 acked.
- NOT verified locally: the workspace test suite and Clippy (standing
  instruction: CI only; the full suite OOMs this box).

## Balance

Recorded here because it is the range's open question, not a defect. See the
session report for the working. The lance's `slug_power` of 1800, priced at a
closing-speed multiplier of 3.0, buys 27 layers of 200 hp reinforced hull, and
no shipped craft is more than about six cells deep along a line of fire - so
roughly 85 percent of every shot leaves through the far side. That, and not
the per-shot damage, is why it reads weak beside PDC spam.
