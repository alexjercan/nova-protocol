# NOTES

## Landed: problem 1, engagement range (2026-08-15)

Scale: 1 unit = 10 m.

| Constant | Where | Was | Now | km |
|---|---|---|---|---|
| `projectile_lifetime`, 100 u/s guns | `sections/standard.rs`, `ships/shared.rs`, `turret_section/config.rs` default | 5.0 s | **2.0 s** | reach 200 u / 2.0 |
| `projectile_lifetime`, 60 u/s guns | `sections/standard.rs`, `ships/shared.rs` | 5.0 s | **3.0 s** | reach 180 u / 1.8 |
| DERIVED fire gate, 100 u/s | `guns.rs` | 450 u | **180 u** | 1.8 |
| DERIVED fire gate, 60 u/s | `guns.rs` | 270 u | **162 u** | 1.62 |
| `AI_STANDOFF_RANGE` | `maneuver.rs` | 250 u | **100 u** | 1.0 |
| `AI_STANDOFF_BAND` | `maneuver.rs` | 60 u | **25 u** | 0.25 |
| `AI_POINT_DEFENSE_RANGE` | `acquisition.rs` | 400 u | **150 u** | 1.5 |
| `AI_ENGAGE_RANGE` | `behavior.rs` | 800 u | **400 u** | 4.0 |
| `AI_THREAT_AIM_RANGE` | `threat.rs` | 500 u | **200 u** | 2.0 |
| `AI_FIRE_RANGE_FACTOR` | `guns.rs` | 0.9 | 0.9 (kept) | - |
| `muzzle_speed` | everywhere | 100 / 60 | unchanged | - |
| `AI_TARGET_MAX_RANGE`, `AI_TORPEDO_MAX_RANGE` | `acquisition.rs`, `torpedo.rs` | 2000 / 1000 | unchanged | 20 / 10 |

`(standoff + band) / gate = 125 / 180 = 0.69`, the ratio the spike said to
preserve. The band's outer edge (125 u) sits inside the WEAKEST shipped gun's
gate (162 u) with 37 u to spare.

Why these and not the spike's exact numbers, plus the live measurement, are in
`SPIKE.md` section 14. Two things worth repeating here:

- The 60 u/s guns got a LONGER lifetime rather than the shorter reach the
  spike leaned toward. Most shipped hostiles fly them, and at 2.0 s their gate
  (108 u) falls inside the global standoff band - they would have flown the
  fight correctly and never fired.
- New guard: `AI_STANDOFF_OUTER_EDGE` and `AI_FIRE_RANGE_FACTOR` are exported
  from `nova_ship`, `nova_authoring::balance::EFFECTIVE_RANGE_MARGIN` now
  ALIASES the engine constant instead of copying it, and
  `every_authored_turret_reaches_past_the_standoff_band` fails the build for
  any authored gun that cannot reach the band its own AI flies.

Checks: `cargo check --workspace`, `cargo fmt`, `cargo test --lib -p nova_ship
input::ai` (117) and `turret` (54), `cargo test --lib -p nova_authoring` (53),
`cargo test -p nova_authoring --test balance_audit_gate` (pass, NO new
acknowledgements needed - the one shipped ack rides the unchanged 1000 u
torpedo envelope), `content lint` (0 errors, 0 warnings), `content -- gen`.

## Not started

Problems 2 (finite ammunition), 3 (ordnance survivability) and 4 (round
models). The range change makes 2 cheaper to reason about: at `pd_range` 150 u
an intercept costs ~111 rounds instead of ~296.
