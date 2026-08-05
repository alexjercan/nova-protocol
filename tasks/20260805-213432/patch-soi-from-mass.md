# Prototype patch: gravity wells authored by mass

Sketch only - not applied, not compiled. Shows where the SOI is computed today,
what the change looks like, and how one scenario would author against it.

**The rule this patch enforces: mass is the only authored gravity quantity.**
Strength and reach both fall out of it. There is no SOI override and no
surface-gravity constructor - mass is the knob you cannot see in the game, which
is exactly why it is the one you tune until the SOI comes out where you want it.
A radius-derived SOI is not a simplification of reality, it is wrong: the real
sphere of influence (`r ~ a (m/M)^(2/5)`) never mentions a body's radius.

## Where it happens today

| File | Role |
| --- | --- |
| `crates/nova_scenario/src/objects/asteroid.rs:61` | `AsteroidConfig::surface_gravity: Option<f32>` - the authored knob |
| `crates/nova_scenario/src/objects/asteroid.rs:254-282` | `insert_asteroid_gravity_well` - decides which rocks get a well |
| `crates/nova_gameplay/src/gravity.rs:67-78` | `GravityWell::from_surface_gravity` - **the actual derivation** |

```rust
// crates/nova_gameplay/src/gravity.rs:67
pub fn from_surface_gravity(
    surface_gravity: f32,
    body_radius: f32,
    settings: &GravitySettings,
) -> Self {
    let g = surface_gravity.clamp(0.0, settings.max_surface_gravity);
    Self {
        mu: g * body_radius * body_radius,
        body_radius,
        soi_radius: settings.soi_factor * body_radius,   // <-- radius, all the way down
    }
}
```

### Why that is the problem

`body_radius` is not the authored radius. `insert_asteroid_gravity_well` passes
the **geometric** `BodyRadius` the collider observer derives from the noise mesh
(`asteroid.rs:242-250`), and that mesh reaches
`ASTEROID_GEOMETRIC_FACTOR_MIN..MAX` = **3.5-6.0x** the nominal radius depending
on the seed (`asteroid.rs:357-359`).

So for the shakedown planetoid (nominal 20, `surface_gravity: Some(6.0)`):

| Quantity | Formula | Range across seeds |
| --- | --- | --- |
| body radius | `20 * [3.5, 6.0]` | 70 - 120 u |
| `mu` | `6.0 * body^2` | 29 400 - 86 400 (**2.9x**) |
| `soi_radius` | `8.0 * body` | 560 - 960 u (**1.7x**) |

Both strength and reach are a per-seed lottery, which is why
`shakedown/tests/pins.rs:414` has to assert every layout fact against the whole
range at once.

## The change

### 1. Settings: a cutoff replaces the factor, a default mass replaces the default surface gravity

```rust
// crates/nova_gameplay/src/gravity.rs, in GravitySettings

    /// Acceleration (u/s^2) below which a well is treated as having no reach:
    /// the SOI is the distance at which `mu / r^2` decays to this. This is the
    /// ONE global knob trading how far wells reach against how strong they are
    /// at a given reach - see the note under the value below.
    pub soi_cutoff_accel: f32,

    /// Mass parameter a designated body gets when the scenario does not author
    /// one. Fixed rather than radius-scaled: a body whose pull matters is a
    /// body worth authoring.
    pub default_mass: f32,

    // REMOVED: pub soi_factor: f32,
    // REMOVED: pub default_surface_gravity: f32,
```

```rust
            soi_cutoff_accel: 0.5,
            default_mass: 22_000.0,
```

**Why 0.5.** With no override, the layout has to come out of the physics, so the
cutoff is chosen to make the mass that feels right also produce the SOI the level
wants. The sweep:

| cutoff | mass for a 300u SOI | surface accel (body 70-120u) | orbit lap at r=150 | shipped 20u planetoid's SOI becomes |
| --- | --- | --- | --- | --- |
| 0.09 | 8 100 | 1.65 - 0.56 | 128 s | 572 - 980 u |
| 0.25 | 22 500 | 4.59 - 1.56 | 77 s | 343 - 588 u |
| **0.50** | **45 000** | **9.18 - 3.12** | **54 s** | **242 - 416 u** |
| 1.00 | 90 000 | 18.4 - 6.25 (clamped at the small end) | 38 s | 171 - 294 u |

0.5 is the row where a 300u SOI comes with a surface pull and an orbit lap in the
same range the game already ships (`surface_gravity: 6.0`, ~45 s lap). It is also
0.5 against a ship's ~21 u/s^2 of thrust - about 2.4%, a defensible "below this,
nobody can feel it".

The cost is honest and global: every existing well's reach shrinks roughly 2.4x,
back near the `soi_factor: 4.0` era that playtest raised to 8.0 ("had to be
almost near it to experience the pull"). Wells now announce themselves later
unless their mass goes up. Re-tuning the shipped bodies is part of the work, not
a follow-up.

### 2. One constructor

```rust
// crates/nova_gameplay/src/gravity.rs
impl GravityWell {
    /// Build a well from the body's authored mass parameter (`mu`, u^3/s^2) and
    /// its geometric radius.
    ///
    /// Mass is the only authored gravity quantity: strength is `a = mu / r^2`
    /// and the SOI is the distance at which that decays to
    /// [`GravitySettings::soi_cutoff_accel`]. The radius is used for one thing
    /// only - the surface clamp that stops a close pass being a singularity
    /// slingshot - and never for how hard or how far the body pulls.
    ///
    /// `mu` is clamped so the acceleration at the surface cannot exceed
    /// [`GravitySettings::max_surface_gravity`]: the guardrail that keeps every
    /// well escapable under main drive.
    pub fn from_mass(mu: f32, body_radius: f32, settings: &GravitySettings) -> Self {
        let mu = mu.clamp(0.0, settings.max_surface_gravity * body_radius * body_radius);
        Self {
            mu,
            body_radius,
            soi_radius: (mu / settings.soi_cutoff_accel.max(1e-6)).sqrt().max(body_radius),
        }
    }
}
```

`from_surface_gravity` is **deleted**. Its ~12 call sites are all test fixtures
(flight, guidance, radar, HUD); each becomes a `from_mass` call with the mass the
old arguments implied - `from_surface_gravity(3.0, 20.0, &s)` becomes
`from_mass(1200.0, 20.0, &s)`. Mechanical, and it makes each fixture state the
quantity that actually drives the system.

### 3. The config authors mass

```rust
// crates/nova_scenario/src/objects/asteroid.rs, in AsteroidConfig

    /// Mass parameter (`mu`, u^3/s^2) making this body a gravity well:
    /// `a = mu / r^2`, and the SOI is where that decays to
    /// [`GravitySettings::soi_cutoff_accel`]. `None` leaves small rocks in flat
    /// space and gives bodies at or above [`GravitySettings::min_well_radius`]
    /// the default mass.
    ///
    /// Tune this by the SOI you want, not by a number that means anything on
    /// its own: mass is invisible in game, the sphere of influence is not.
    /// `mu = soi_cutoff_accel * soi^2`.
    pub mass: Option<f32>,

    // REMOVED: pub surface_gravity: Option<f32>,
```

`surface_gravity` goes with the constructor. Six shipped
`assets/base/scenarios/*.content.ron` files author it
(`shakedown_run`, `final_tally`, `asteroid_field`, `menu_ambience`,
`menu_scrapyard`, `menu_waystation`); they are generated, so the Rust builders
change and `content -- gen` rewrites them.

The component rename follows: `AsteroidSurfaceGravity(Option<f32>)` ->
`AsteroidMass(Option<f32>)`.

### 4. The well-insertion observer

```rust
// crates/nova_scenario/src/objects/asteroid.rs:254
fn insert_asteroid_gravity_well(
    add: On<Add, BodyRadius>,
    mut commands: Commands,
    settings: Res<GravitySettings>,
    q_asteroid: Query<(&AsteroidRadius, &BodyRadius, &AsteroidMass), With<AsteroidMarker>>,
) {
    let entity = add.entity;
    let Ok((radius, body_radius, authored)) = q_asteroid.get(entity) else {
        return;
    };

    let mu = match **authored {
        Some(mass) => mass,
        None if **radius >= settings.min_well_radius => settings.default_mass,
        None => return,
    };

    commands.entity(entity).insert((
        GravityWell::from_mass(mu, **body_radius, &settings),
        RigidBody::Static,
    ));
}
```

Qualification still keys on the **nominal** radius - that is the designation
intent and it is seed-independent. Only the well's numbers stop caring about
radius.

## What one scenario looks like after

The shakedown planetoid, which is the whole reason for the change. Today:

```rust
// crates/nova_assets/src/scenario/shakedown/mod.rs:576
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: PLANETOID_NOMINAL_RADIUS,          // 20
            surface_gravity: Some(6.0),                // mu = 29 400 .. 86 400, SOI 560 .. 960
            health: 2000.0,
            invulnerable: true,
            ..
        }),
```

After - the body moves in to `(520, -40, -520)` and its mass is tuned to the SOI
the layout needs:

```rust
/// The planetoid's mass parameter (mu, u^3/s^2). Tuned to the SOI, which is the
/// thing the layout cares about and the thing a player can feel:
/// `mu = soi_cutoff_accel * soi^2` = 0.5 * 300^2. Authored as mass because the
/// geometric radius swings 3.5-6.0x with the mesh seed, so anything derived from
/// radius was a different well on every load.
const PLANETOID_MASS: f32 = 45_000.0;
/// What that mass buys, and what the layout below is authored against. Not a
/// setting - a consequence. Recompute it if the mass or the cutoff moves.
const PLANETOID_SOI: f32 = 300.0;
const PLANETOID_POS: Vec3 = Vec3::new(520.0, -40.0, -520.0);
```

```rust
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: PLANETOID_NOMINAL_RADIUS,
            mass: Some(PLANETOID_MASS),
            health: 2000.0,
            invulnerable: true,
            ..
        }),
```

What the player gets, across the whole seed range:

| | Value |
| --- | --- |
| SOI | 300 u, every seed |
| surface pull | 3.1 - 9.2 u/s^2 (under the 10 u/s^2 escapability cap on every seed) |
| circular orbit at the 150u ring | 17.3 u/s, ~54 s lap |

And the layout constants that hung off the seed-varying SOI become plain numbers:

```rust
// beacon 4: inside the SOI so the ORBIT hint lights on arrival
const BEACON_4_POS: Vec3 = Vec3::new(660.0, 20.0, -380.0);   // 207u from the body
// the coast ring: outside the widest orbit ring (182u), inside the SOI
const COAST_RING_RADIUS: f32 = 220.0;
```

## What the change does to the tests

`crates/nova_assets/src/scenario/shakedown/tests/pins.rs:414`,
`beat4_geometry_holds_across_the_derived_radius_range`, exists only to sweep the
3.5-6.0x range. With a mass-derived SOI it collapses:

```rust
#[test]
fn beat4_geometry_holds_against_the_planetoid_soi() {
    // The SOI is now a property of the mass alone.
    assert_eq!(
        PLANETOID_SOI,
        (PLANETOID_MASS / GravitySettings::default().soi_cutoff_accel).sqrt(),
        "PLANETOID_SOI must stay the SOI the authored mass actually produces"
    );

    // The orbit ring still rides the geometric radius - it is a distance from a
    // surface, not a property of the well.
    let widest_orbit = ORBIT_RING_FACTOR
        * (PLANETOID_NOMINAL_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MAX + SURFACE_MARGIN);

    assert!(BEACON_4_POS.distance(PLANETOID_POS) < PLANETOID_SOI * 0.75);
    assert!(COAST_RING_RADIUS > widest_orbit);
    assert!(COAST_RING_RADIUS < PLANETOID_SOI - 50.0);

    for (name, pos) in gravity_free_positions() {
        assert!(
            pos.distance(PLANETOID_POS) > PLANETOID_SOI + 40.0,
            "{name} must stay outside the planetoid's SOI"
        );
    }
}
```

Verified against the proposed coordinates (distance to the body at
`(520,-40,-520)`):

| Must stay gravity-free | Distance | vs SOI + 40 = 340 |
| --- | --- | --- |
| player spawn | 736 | clear |
| beacon 1 | 549 | clear |
| beacon 2 | 417 | clear |
| debris cluster | 403 | clear |
| crate 1 / 2 / 3 | 380 / 410 / 435 | clear |
| derelict | 602 | clear |
| beacon 3 | 658 | clear |

| Must sit inside | Value | Bound |
| --- | --- | --- |
| beacon 4 | 207 u | < 225 (`SOI * 0.75`) |
| coast ring | 220 u | 182 < r < 250 |

## Blast radius

- `soi_factor` and `default_surface_gravity` leave `GravitySettings`;
  `soi_cutoff_accel` and `default_mass` arrive. Both reflected, so the inspector
  follows automatically.
- `from_surface_gravity` deleted: ~12 fixture call sites across
  `flight/tests/*`, `flight/guidance.rs`, `input/targeting/radar.rs`,
  `hud/velocity.rs`, `hud/maneuver_instruments.rs` become `from_mass`.
- `AsteroidConfig::surface_gravity` deleted: six generated content files plus
  three examples (`screenshot_scene`, `screenshot_flight`, `turret_section`)
  author it. Rust builders change, `content -- gen` rewrites the RON. This is a
  **format break** - the changelog line says so, and any mod authoring
  `surface_gravity` breaks.
- Every shipped well's reach shrinks ~2.4x at `soi_cutoff_accel: 0.5`. Each body
  needs its mass re-tuned to the SOI it should have, and the orbit beat needs a
  playtest, not just a passing test.

## Open

- Is `mass` the right name for `mu`? It is a gravitational parameter, not a mass,
  and the module docs are firm that strength is a designer stat rather than
  physics. `mass` is what a designer will reach for; `gravity_mu` is accurate.
- `default_mass: 22_000` is a placeholder - it wants the same sweep the cutoff
  got, across the bodies that currently rely on the default.
- `max_surface_gravity` still expresses its guardrail as a surface acceleration
  even though nothing authors one any more. It reads fine as a clamp ("no well
  out-muscles a live drive"), but it is now the only surface-gravity number left
  in the system.
