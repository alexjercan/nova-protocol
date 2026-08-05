# Prototype patch: SOI from mass, with an override

Sketch only - not applied, not compiled. Shows where the SOI is computed today,
what the change looks like, and how one scenario would author against it.

## Where it happens today

Three files, in this order at runtime:

| File | Role |
| --- | --- |
| `crates/nova_scenario/src/objects/asteroid.rs:61` | `AsteroidConfig::surface_gravity: Option<f32>` - the authored knob |
| `crates/nova_scenario/src/objects/asteroid.rs:254-282` | `insert_asteroid_gravity_well` - decides which rocks get a well and calls the constructor |
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

Author the **mass parameter** (`mu`) instead of a surface acceleration, and
derive the SOI from it. Radius keeps exactly one job: the surface clamp.

### 1. Gravity settings gain a cutoff, lose the factor

```rust
// crates/nova_gameplay/src/gravity.rs, in GravitySettings
    /// Acceleration (u/s^2) below which a well is considered to have no reach:
    /// the derived SOI is the distance at which `mu / r^2` falls to this.
    /// 0.09 keeps today's shipped 20u planetoid at roughly its current SOI, so
    /// existing scenarios move as little as possible.
    pub soi_cutoff_accel: f32,

    // REMOVED: pub soi_factor: f32,
```

```rust
    soi_cutoff_accel: 0.09,
```

### 2. The constructor takes mass and an optional override

```rust
// crates/nova_gameplay/src/gravity.rs
impl GravityWell {
    /// Build a well from an authored mass parameter (`mu`, u^3/s^2) and the
    /// body's geometric radius.
    ///
    /// `soi_override` is the authored sphere of influence: `None` derives it
    /// from the mass - the distance at which the pull decays to
    /// [`GravitySettings::soi_cutoff_accel`] - and `Some(r)` keeps `r` exactly
    /// as authored, for scenarios whose LAYOUT needs a specific reach.
    ///
    /// The escapability guardrail still applies, now at the surface: `mu` is
    /// clamped so the acceleration at `body_radius` never exceeds
    /// [`GravitySettings::max_surface_gravity`].
    pub fn from_mass(
        mu: f32,
        body_radius: f32,
        soi_override: Option<f32>,
        settings: &GravitySettings,
    ) -> Self {
        let mu = mu.clamp(0.0, settings.max_surface_gravity * body_radius * body_radius);
        let derived = (mu / settings.soi_cutoff_accel.max(1e-6)).sqrt();
        Self {
            mu,
            body_radius,
            soi_radius: soi_override.unwrap_or(derived).max(body_radius),
        }
    }

    /// Build a well from a surface acceleration at `body_radius`. Kept as the
    /// ergonomic form for small bodies where "how hard does it pull at the
    /// surface" is the natural question; it forwards to [`Self::from_mass`],
    /// so the SOI still derives from mass rather than from radius.
    pub fn from_surface_gravity(
        surface_gravity: f32,
        body_radius: f32,
        settings: &GravitySettings,
    ) -> Self {
        Self::from_mass(
            surface_gravity * body_radius * body_radius,
            body_radius,
            None,
            settings,
        )
    }
}
```

`from_surface_gravity` stays because ~12 call sites (flight tests, HUD tests,
radar tests, guidance tests) use it as a fixture and do not care about the SOI.
They keep compiling unchanged; only the derived SOI value shifts.

### 3. The config gains two optional fields

```rust
// crates/nova_scenario/src/objects/asteroid.rs, in AsteroidConfig
    /// Authored mass parameter (`mu`, u^3/s^2): `a = mu / r^2`. When set it
    /// wins over `surface_gravity` and the body's radius plays no part in how
    /// hard or how far the well pulls - the point of authoring mass rather
    /// than a surface value, since the geometric radius varies 3.5-6.0x with
    /// the mesh seed.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub mass: Option<f32>,

    /// Authored sphere of influence, world units. `None` derives it from the
    /// mass; `Some(r)` pins it, for bodies whose reach is a LAYOUT decision
    /// rather than a physical one.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub soi_radius: Option<f32>,
```

Both are `Option` + `serde(default)` + `skip_serializing_if`, so all six shipped
`assets/base/scenarios/*.content.ron` files that author `surface_gravity` stay
byte-identical until someone opts in.

They ride to the entity as components alongside the existing
`AsteroidSurfaceGravity`:

```rust
// asteroid.rs, in the asteroid bundle (next to line 95)
        AsteroidSurfaceGravity(config.surface_gravity),
        AsteroidMass(config.mass),
        AsteroidSoiRadius(config.soi_radius),
```

### 4. The well-insertion observer picks the authored form

```rust
// crates/nova_scenario/src/objects/asteroid.rs:254
fn insert_asteroid_gravity_well(
    add: On<Add, BodyRadius>,
    mut commands: Commands,
    settings: Res<GravitySettings>,
    q_asteroid: Query<
        (
            &AsteroidRadius,
            &BodyRadius,
            &AsteroidSurfaceGravity,
            &AsteroidMass,
            &AsteroidSoiRadius,
        ),
        With<AsteroidMarker>,
    >,
) {
    let entity = add.entity;
    let Ok((radius, body_radius, authored_g, authored_mass, authored_soi)) =
        q_asteroid.get(entity)
    else {
        return;
    };

    // Mass wins; surface gravity is the fallback knob; a big undesignated rock
    // still gets the default well; small field rocks stay flat space.
    let mu = match (**authored_mass, **authored_g) {
        (Some(mass), _) => mass,
        (None, Some(g)) => g * **body_radius * **body_radius,
        (None, None) if **radius >= settings.min_well_radius => {
            settings.default_surface_gravity * **body_radius * **body_radius
        }
        (None, None) => return,
    };

    commands.entity(entity).insert((
        GravityWell::from_mass(mu, **body_radius, **authored_soi, &settings),
        RigidBody::Static,
    ));
}
```

## What one scenario looks like after

The shakedown planetoid, which is the whole reason for the change. Today:

```rust
// crates/nova_assets/src/scenario/shakedown/mod.rs:576
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: PLANETOID_NOMINAL_RADIUS,          // 20
            surface_gravity: Some(6.0),                // mu = 29 400 .. 86 400
            health: 2000.0,
            invulnerable: true,
            ..
        }),
```

After - the body moves in to `(520, -40, -520)` and authors its own reach:

```rust
/// The planetoid's mass parameter (mu, u^3/s^2). Authored rather than derived
/// from a surface gravity: the geometric radius swings 3.5-6.0x with the mesh
/// seed, so a surface-authored well was a different well on every load.
/// 48 000 is the midpoint of what `surface_gravity: 6.0` used to produce, so
/// the orbit beat feels the same as it did in playtest.
const PLANETOID_MASS: f32 = 48_000.0;
/// Authored SOI. The derived value (~730u at the default cutoff) would reach
/// the salvage crates; the belt layout wants a well that stops well short of
/// them, and 300u is a LAYOUT decision, so it is authored rather than tuned
/// into the mass.
const PLANETOID_SOI: f32 = 300.0;
const PLANETOID_POS: Vec3 = Vec3::new(520.0, -40.0, -520.0);
```

```rust
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: PLANETOID_NOMINAL_RADIUS,
            mass: Some(PLANETOID_MASS),
            soi_radius: Some(PLANETOID_SOI),
            surface_gravity: None,
            health: 2000.0,
            invulnerable: true,
            ..
        }),
```

And the layout constants that hung off the seed-varying SOI become plain
numbers:

```rust
// beacon 4: inside the SOI so the ORBIT hint lights on arrival
const BEACON_4_POS: Vec3 = Vec3::new(660.0, 20.0, -380.0);   // 207u from the body
// the coast ring: outside the widest orbit ring (182u), inside the SOI
const COAST_RING_RADIUS: f32 = 220.0;
```

## What the change does to the tests

`crates/nova_assets/src/scenario/shakedown/tests/pins.rs:414`,
`beat4_geometry_holds_across_the_derived_radius_range`, exists only to sweep the
3.5-6.0x range. With an authored SOI it collapses:

```rust
#[test]
fn beat4_geometry_holds_against_the_authored_soi() {
    // The orbit ring still rides the geometric radius - only the SOI is authored.
    let widest_orbit = ORBIT_RING_FACTOR
        * (PLANETOID_NOMINAL_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MAX + SURFACE_MARGIN);

    assert!(BEACON_4_POS.distance(PLANETOID_POS) < PLANETOID_SOI * 0.75);
    assert!(COAST_RING_RADIUS > widest_orbit);
    assert!(COAST_RING_RADIUS < PLANETOID_SOI - 50.0);

    for (name, pos) in gravity_free_positions() {
        assert!(
            pos.distance(PLANETOID_POS) > PLANETOID_SOI + 40.0,
            "{name} must stay outside the planetoid's authored SOI"
        );
    }
}
```

Verified against the proposed coordinates (units, distance to the body at
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

- `soi_factor` disappears from `GravitySettings`. Grep hits: the field itself,
  its default, `from_surface_gravity`, and `pins.rs`. The inspector picks up the
  replacement automatically (reflected resource).
- Six shipped content files author `surface_gravity`
  (`final_tally`, `menu_ambience`, `asteroid_field`, `menu_scrapyard`,
  `shakedown_run`, `menu_waystation`) plus three examples
  (`screenshot_scene`, `screenshot_flight`, `turret_section`). None change
  syntactically; each needs its SOI re-checked once, since the derived value
  moves from `8 * body` to `sqrt(mu / cutoff)`.
- `cutoff = 0.09` is chosen so a 20u nominal body lands near its current SOI.
  Worth a sweep across the shipped bodies before committing to the number.

## Open

- Is `mass` the right name for `mu`? It is a gravitational parameter, not a
  mass, and the module docs are emphatic that strength is a designer stat, not
  physics. `gravity_mu` is accurate; `mass` is what a designer will look for.
- `from_surface_gravity` staying as a forwarding constructor is a convenience
  for ~12 test fixtures. If it reads as two ways to say the same thing, the
  alternative is one constructor and a mechanical fixture update.
