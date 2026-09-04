// Interactive doc widgets: small in-page simulators for the wiki pages.
//
// Authoring model: a doc .md carries a tight raw-HTML block like
//
//   <div class="widget" data-widget="round-travel">
//   <p>...static fallback prose for no-JS readers and search engines...</p>
//   </div>
//
// and this module hydrates it by `data-widget` key: the fallback is replaced
// with live controls built from the DOM, no dependencies. A key nobody
// registered (or JS disabled) leaves the fallback prose in place, styled by
// the .widget panel rules in style.css.
//
// Every game number in here is lifted from the Rust source, not tuned for the
// page - the file:line each constant was verified against is on its comment.
// The widgets simulate the same rules the game runs; where a fixture is
// invented for legibility (a stack of identical sections) it uses real
// catalog values and says so in its caption.
//
// UNITS. Nova authors and reasons in METERS (crates/nova_events/src/units.rs),
// so every constant lifted from a typed quantity (`Meters`, `MetersPerSecond`,
// `MetersPerSecondSquared`) or from authored content is a metric number here
// and needs no conversion to print. Engine world units survive where Nova
// meets Bevy, avian, a mesh or the build grid - gravity's `mu` and cutoffs,
// `FlightSettings`, the attitude arm, thruster impulse, the damage curves'
// reference closing speed, ship part boxes - and those constants keep the
// engine number with a comment saying so. One world unit is METERS_PER_UNIT
// meters, and the `engine*` formatters below are the ONLY place a world-unit
// figure crosses into a player-facing reading.

// ---- constants verified against the game source ---------------------------

// Turret aim servo (crates/nova_ship/src/sections/turret_section/aim.rs).
// The servo closes `1 - exp(-rate * dt)` of the aim error per frame - the
// same fraction per unit TIME at any frame rate (aim.rs:400). The rate is
// derived from the old flat 0.35-of-the-error-per-frame gain at 60 fps
// (aim.rs:402-406), which is the "old servo" curve shown for contrast.
const AIM_CORRECTION_RATE = 25.847; // aim.rs:407
const OLD_PER_FRAME_GAIN = 0.35; // aim.rs:403 (historical, documented)

// The fire gate: a round fired `e` radians off misses laterally by
// `range * sin(e)`, so the widest usable error is hull half-beam over range.
// Both sides are engine geometry and stay in world units (aim.rs:25,30) - a
// 1.6 u half-beam over a 100 u range, which is 16 m over 1 km - and the
// RATIO they make is the dimensionless gate: 0.016 rad = 0.92 deg (aim.rs:53).
const FIRE_GATE_RAD = 1.6 / 100;
const FIRE_GATE_DEG = FIRE_GATE_RAD * (180 / Math.PI);

// Damage travel rules (crates/nova_gameplay/src/damage.rs). Both bullet curves
// are ratios against a reference closing speed, and that speed is still an
// engine `f32` in WORLD UNITS per second: 100 u/s, which is the same speed as
// the PDC's authored 1 000 m/s muzzle speed. Everything the widget feeds it is
// therefore in world units too, and only the readout converts.
const REFERENCE_CLOSING_SPEED = 100; // damage.rs:183 (world units/s)
const KINETIC_DAMAGE_FLOOR = 0.25; // damage.rs:189
const KINETIC_DAMAGE_CEILING = 2.0; // damage.rs:195
const PIERCE_POWER_FLOOR = 0.5; // damage.rs:199
const PIERCE_POWER_CEILING = 3.0; // damage.rs:204
const PIERCE_BASE_POWER = 300; // damage.rs:215
const MAX_PIERCE_LAYERS = 6; // damage.rs:223
const EXPLOSIVE_SECTION_TRANSMISSION = 0.65; // damage.rs:523
// Blast free pressure falls off linearly to zero at the radius
// (damage.rs:565-571); each destroyed structural layer transmits 65%, a
// surviving layer stops the wave (damage.rs:573-576; ray walk 620-680).

// Gravity wells (crates/nova_gameplay/src/gravity.rs). Mass (`mu`) is the ONLY
// authored gravity quantity: both the pull `a = mu / r^2` and the reach (the
// SOI, where that pull decays to the cutoff) fall out of it (gravity.rs:96-108).
// The whole well model is ENGINE UNITS and stays that way: `mu` is u^3/s^2, so
// an SI one would be a thousand times this, and the cutoff below is u/s^2.
const SOI_CUTOFF_ACCEL = 0.25; // gravity.rs:211 (world units/s^2)
const GRAVITY_FADE_FRACTION = 0.15; // gravity.rs:212
const GRAVITY_SURFACE_MARGIN = 1.0; // gravity.rs:213 (world units)
const WELL_SWITCH_HYSTERESIS = 1.1; // gravity.rs:214
// ORBIT's trusted band (crates/nova_ship/src/flight/state.rs).
const ORBIT_CLEARANCE_FACTOR = 1.5; // state.rs:392
const ORBIT_BAND_SAFETY = 0.9; // state.rs:393
// Shipped fixtures: the two campaign planetoids (crates/nova_authoring/src/
// base_content/scenarios/nova_protocol/stage.rs). A scenario's `mass`
// is still an `Option<f32>` in u^3/s^2 (nova_scenario/src/objects/asteroid.rs:92).
const INSPECTION_PLANETOID_MU = 27000; // stage.rs:29 (u^3/s^2)
const CONCEALMENT_PLANETOID_MU = 20000; // stage.rs:39 (u^3/s^2)

// Radar locking (crates/nova_ship/src/input/targeting/). The dwell curve's
// reference range is still an engine `f32` in world units (state.rs:54-57), so
// the trainer's contact distances are world units too.
const RADAR_TAP_SECS = 0.25; // gesture.rs:18
const TARGETING_CONE_HALF_ANGLE_DEG = 18.0; // radar.rs:20
const LOCK_DWELL_BASE = 0.6; // state.rs:73
const LOCK_DWELL_RANGE_FACTOR = 1.5; // state.rs:74
const LOCK_DWELL_REFERENCE_RANGE = 2000; // state.rs:75 (world units)
const LOCK_DWELL_MIN = 0.25; // state.rs:76
const LOCK_DWELL_MAX = 2.5; // state.rs:77
const COMBAT_DECAY_SECS = 30; // contacts.rs:24

// GOTO flight controller (crates/nova_ship/src/flight/state.rs defaults;
// the speed-envelope and flip rules are ported from flight/guidance.rs).
// `FlightSettings` is engine tuning and every length and speed on it is still
// WORLD UNITS - the doc comments on the fields say so - so the GOTO scope
// models in world units and converts only where it prints.
// Cited by FIELD, not by line: the file moves and a stale line number reads
// like a checked fact.
const ARRIVAL_STANDOFF = 50; // FlightSettings::arrival_standoff (world units)
const DECEL_MARGIN = 0.85; // FlightSettings::decel_margin
const MIN_APPROACH_SPEED = 1.5; // FlightSettings::min_approach_speed (u/s)
const ARRIVAL_SPOOL_PAD = 0.5; // FlightSettings::arrival_spool_pad
const STOP_SPEED_EPSILON = 0.2; // FlightSettings::stop_speed_epsilon (u/s)
const TURN_RATE_SCALE = 0.9; // FlightSettings::turn_rate_scale
const TURN_RATE_MIN_DEG = 10; // FlightSettings::turn_rate_min_deg
const TURN_RATE_MAX_DEG = 240; // FlightSettings::turn_rate_max_deg
const RCS_ACCEL = 1.5; // FlightSettings::rcs_accel (world units/s^2)
const RCS_SPEED_CAP = 2.0; // FlightSettings::rcs_speed_cap (world units/s)

// ---- units -----------------------------------------------------------------
//
// Readings follow the HUD's policy: whole meters below a kilometer, kilometers
// above it, speeds in m/s. The four plain formatters take SI, because that is
// what the authored catalog now holds. The four `engine*` ones take a world-
// unit figure and are the ONLY place the engine scale is applied - they exist
// for the scopes that model an engine quantity (gravity, the flight settings,
// the attitude arm, the damage curves' closing speed, the build lattice).

/**
 * Meters in one engine world unit (crates/nova_events/src/units.rs:32).
 *
 * Still needed because several scopes model engine quantities directly: the
 * gravity well (`mu` in u^3/s^2 and its u/s^2 cutoff), `FlightSettings`
 * (standoff, approach floor, RCS), the structural arm read off collider
 * boxes, the damage curves' reference closing speed, and the build lattice a
 * railgun corridor is counted in - a cell is one world unit, so 10 m.
 */
export const METERS_PER_UNIT = 10;

function numText(n: number, decimals: number): string {
    return n.toLocaleString("en-US", { maximumFractionDigits: decimals });
}

/** A length in meters, or in kilometers from one kilometer up. */
export function meters(m: number, decimals = 0): string {
    if (Math.abs(m) >= 1000) return `${numText(m / 1000, 2)} km`;
    return `${numText(m, decimals)} m`;
}

/** A length in meters, always as kilometers. */
export function kilometers(m: number, decimals = 2): string {
    return `${numText(m / 1000, decimals)} km`;
}

/** A speed in meters per second. */
export function metersPerSec(mps: number, decimals = 0): string {
    return `${numText(mps, decimals)} m/s`;
}

/** An acceleration in meters per second squared. */
export function metersPerSec2(mps2: number, decimals = 1): string {
    return `${numText(mps2, decimals)} m/s^2`;
}

/** An engine world-unit length, read out in meters. */
export function engineMeters(units: number, decimals = 0): string {
    return meters(units * METERS_PER_UNIT, decimals);
}

/** An engine world-unit length, read out in kilometers. */
export function engineKilometers(units: number, decimals = 2): string {
    return kilometers(units * METERS_PER_UNIT, decimals);
}

/** An engine world-unit speed, read out in meters per second. */
export function engineMetersPerSec(unitsPerSec: number, decimals = 0): string {
    return metersPerSec(unitsPerSec * METERS_PER_UNIT, decimals);
}

/** An engine world-unit acceleration, read out in m/s^2. */
export function engineMetersPerSec2(
    unitsPerSec2: number,
    decimals = 1
): string {
    return metersPerSec2(unitsPerSec2 * METERS_PER_UNIT, decimals);
}

// The attitude envelope (crates/nova_ship/src/physics/attitude.rs:69-91): a
// hull's turn ceiling is the lower of its computers' torque over its inertia
// and the structural load limit over its arm. Nothing authors the ceiling.
// The load limit is SI (`MetersPerSecondSquared`); the arm arrives from
// collider geometry in world units and crosses in `structuralCeiling` below,
// exactly as `AttitudeEnvelope::new` takes `Meters::from_engine(arm)`
// (controller_section.rs:487-489).
const LOAD_LIMIT = 8 * 9.81; // m/s^2, scale.rs:17 (MetersPerSecondSquared)
const CONTROLLER_MAX_TORQUE = 1501; // standard.rs:726
// The shipped corvette's structural arm: centre of mass to the outer FACE of
// its furthest section (attitude.rs:136-149). WORLD UNITS - it is measured off
// avian collider boxes, and the ship part tables below are the same geometry.
// The GOTO widget flies this hull. `hullState(CARGOA_PARTS)` re-derives it from
// the craft's own boxes and agrees, which is the check that the assembly model
// below is the game's.
const CORVETTE_ARM_U = 2.76;

// Thrust is authored as an IMPULSE PER FIXED TICK and handed to avian with no
// `dt` factor, so a hull's acceleration is its summed magnitude times the tick
// RATE over its mass (thruster_section.rs:474-481,:569-575) - and that
// acceleration comes out in WORLD UNITS per second squared. Nothing configures
// `Time<Fixed>`, so the rate is Bevy's own.
const FIXED_TICK_HZ = 64; // thruster_section.rs:489
const THRUSTER_MAGNITUDE = 1.0; // standard.rs:650, ships/shared.rs:285

// Catalog fixtures (crates/nova_authoring/src/base_content/sections/standard.rs).
// Authored content is METERS now, so the blast radius is the metric number.
const LIGHT_HULL_HP = 60; // standard.rs:758 (light_hull_section)
const TORPEDO_BLAST_DAMAGE = 750; // standard.rs:1207 (Serpent/Lance warhead)
const TORPEDO_BLAST_RADIUS = 300; // standard.rs:1199 (Meters)

// The shared turret mount (standard.rs `turret_joint_tree` :206-318). Traverse
// is unbounded (:279-282) and elevation runs from the depression floor to
// straight up (:298-299), so what a mount cannot see is a cone under its own
// keel and nothing else. Every hinge slews at the same rate (:279,:291).
const TURRET_DEPRESSION_DEG = -10; // standard.rs:104,298 (PI / 18)
const TURRET_ELEVATION_DEG = 90; // standard.rs:299 (FRAC_PI_2)
const TURRET_SLEW_DEG_S = 180; // standard.rs:281,291 (PI rad/s)
// A turret has no range field: muzzle speed times projectile lifetime IS its
// reach (config.rs:135-144), 1 000 m/s over 2.0 s.
const PDC_REACH = 2000; // meters; standard.rs:486,493

// Magazines and the quiet interval that refills them
// (crates/nova_ship/src/sections/ammo.rs). One reload rule serves every
// section kind (ammo.rs:189, sections/mod.rs:223): a SUCCESSFUL shot resets
// the clock (ammo.rs:136), a whole batch lands the moment the clock passes
// the delay (ammo.rs:171-174), and the total is clamped at capacity
// (ammo.rs:156). An EMPTY trigger pull never resets it (ammo.rs:134), so a
// dry weapon reloads while the trigger is still held down.
const PDC_CAPACITY = 500; // standard.rs:503
const PDC_RELOAD_DELAY = 3.0; // standard.rs:505
const PDC_RELOAD_AMOUNT = 200; // standard.rs:506
const PDC_FIRE_RATE = 100; // standard.rs:74 (rounds per second)
const BAY_CAPACITY = 6; // standard.rs:1226
const BAY_RELOAD_DELAY = 10.0; // standard.rs:1238
const BAY_RELOAD_AMOUNT = 1; // standard.rs:1239
const BAY_FIRE_RATE = 1.0; // standard.rs:1189 (launches per second)

// The terminal weave (crates/nova_ship/src/sections/torpedo_section/). The
// corkscrew rides at full amplitude beyond three blast radii and tapers to
// nothing half a radius out, so the torpedo arrives dead on the aim point
// (projectile.rs:435-439).
const WEAVE_FULL_RADII = 3.0; // projectile.rs:437
const WEAVE_ZERO_RADII = 0.5; // projectile.rs:436
const SERPENT_WEAVE_ANGLE = 0.44; // mod.rs:414 (rad, the balance knob)
const SERPENT_WEAVE_RATE = 1.4; // mod.rs:415 (rad/s)

// The spinal lance (standard.rs, `railgun_lance_section`). Its slug is a
// Pierce round with NO layer cap (railgun_section/firing.rs:206 sets
// `layers: u32::MAX`), so `slug_power` alone bounds what one shot takes, and
// at 15 000 m/s the pierce curve sits at its 3.0 ceiling whatever the ships are
// doing. The shot's cycle is the charge plus the one-shell reload.
const LANCE_CHARGE_SECONDS = 1.5; // standard.rs:928
const LANCE_SLUG_SPEED = 15000; // standard.rs:929 (MetersPerSecond)
const LANCE_SLUG_DAMAGE = 300; // standard.rs:943
const LANCE_SLUG_POWER = 1800; // standard.rs:949
const LANCE_RAKE_RADIUS = 10; // standard.rs:968 (Meters)
const LANCE_SLUG_LIFETIME = 1.2; // standard.rs:971
const LANCE_RELOAD_DELAY = 12; // standard.rs:987
const LANCE_CYCLE_SECS = LANCE_CHARGE_SECONDS + LANCE_RELOAD_DELAY;
const LANCE_REACH = LANCE_SLUG_SPEED * LANCE_SLUG_LIFETIME; // meters
// The corridor scope counts BUILD CELLS, and a cell is one world unit - the
// same crossing the gun makes when it hands the rake to the sweep as
// `radius.to_engine()` (railgun_section/firing.rs:215-218). This is the one
// place the authored corridor meets the lattice.
export const LANCE_RAKE_RADIUS_CELLS = LANCE_RAKE_RADIUS / METERS_PER_UNIT;
const REINFORCED_HULL_HP = 200; // standard.rs:604

// The kinetic PDC round (standard.rs:59; the pierce round is half of it,
// :69) and the torpedoes' reach at the bay's 100 s lifetime (standard.rs:1191),
// from the measured along-the-line table at the head of ordnance.rs:13-21 -
// that table is still quoted in world units, so the reaches and cruise caps
// below are its numbers stated in meters. A torpedo's cruise cap is authored:
// ordnance.rs:49 (Lance) and torpedo_section/mod.rs:413 (Serpent).
const KINETIC_PDC_BULLET_DAMAGE = 4.0; // standard.rs:59
const PDC_MUZZLE_SPEED = 1000; // standard.rs:486 (MetersPerSecond)
const SERPENT_REACH = 29140; // meters; ordnance.rs:21 (2 914 u)
const LANCE_TORPEDO_REACH = 31300; // meters; ordnance.rs:21 (3 130 u)
const SERPENT_CRUISE = 320; // torpedo_section/mod.rs:413 (MetersPerSecond)
const LANCE_TORPEDO_CRUISE = 350; // ordnance.rs:49 (MetersPerSecond)
// Rounds one stock PDC spends to stop each type (ordnance.rs:18).
const ROUNDS_PER_LANCE_TORPEDO = 116;
const ROUNDS_PER_SERPENT = 390;
// The starter ship's soft manual-speed cap: what a torpedo has to catch when
// the target is running (first_shift.rs:93, `MetersPerSecond`).
const PLAYER_SPEED_CAP = 250;

// ---- pure models (mirror the Rust rules) ----------------------------------

const clamp = (v: number, lo: number, hi: number): number =>
    Math.min(hi, Math.max(lo, v));

// Steady-state tracking lag against a target crossing at `crossDegS`: lag is
// where the per-frame correction equals the per-frame target motion
// (simplified model - the course of the real servo between frames).
export function aimLagOldDeg(fps: number, crossDegS: number): number {
    return crossDegS / (OLD_PER_FRAME_GAIN * fps);
}
export function aimLagNowDeg(fps: number, crossDegS: number): number {
    const gain = 1 - Math.exp(-AIM_CORRECTION_RATE / fps);
    return crossDegS / fps / gain;
}

// damage.rs:253-255. Closing speed is world units per second, the same
// system as REFERENCE_CLOSING_SPEED.
export function kineticDamageMultiplier(closingSpeed: number): number {
    return clamp(
        closingSpeed / REFERENCE_CLOSING_SPEED,
        KINETIC_DAMAGE_FLOOR,
        KINETIC_DAMAGE_CEILING
    );
}
// damage.rs:265-267, on the same world-unit closing speed.
export function piercePowerMultiplier(closingSpeed: number): number {
    return clamp(
        closingSpeed / REFERENCE_CLOSING_SPEED,
        PIERCE_POWER_FLOOR,
        PIERCE_POWER_CEILING
    );
}

interface SectionResult {
    state: "dead" | "hit" | "intact";
    dealt: number;
}

// Kinetic walk (damage.rs:441-452 rule): the round spends its damage budget;
// it carries on only through sections it destroys, and a section it fails to
// destroy absorbs it whole.
export function kineticWalk(
    damage: number,
    closingSpeed: number,
    sections: number,
    hp: number
): { results: SectionResult[]; leftover: number } {
    const scale = kineticDamageMultiplier(closingSpeed);
    const results: SectionResult[] = [];
    let remaining = damage;
    for (let i = 0; i < sections; i++) {
        if (remaining <= 0) {
            results.push({ state: "intact", dealt: 0 });
            continue;
        }
        const punch = remaining * scale;
        if (punch <= hp) {
            results.push({ state: punch >= hp ? "dead" : "hit", dealt: punch });
            remaining = 0;
        } else {
            results.push({ state: "dead", dealt: hp });
            remaining -= hp / scale;
        }
    }
    return { results, leftover: Math.max(0, remaining) };
}

// Pierce walk (damage.rs:454-462 rule): full authored damage to every section
// crossed; crossing costs the section's MAX health (not remaining) out of the
// round's power budget, with a hard layer ceiling.
export function pierceWalk(
    damage: number,
    closingSpeed: number,
    sections: number,
    hp: number
): { results: SectionResult[]; cost: number; raked: number } {
    const cost = hp / piercePowerMultiplier(closingSpeed);
    const results: SectionResult[] = [];
    let power = PIERCE_BASE_POWER;
    let layers = MAX_PIERCE_LAYERS;
    let raked = 0;
    for (let i = 0; i < sections; i++) {
        if (power <= 0 || layers <= 0) {
            results.push({ state: "intact", dealt: 0 });
            continue;
        }
        results.push({ state: damage >= hp ? "dead" : "hit", dealt: damage });
        raked += 1;
        power -= cost;
        layers -= 1;
    }
    return { results, cost, raked };
}

interface BlastLayer {
    incoming: number;
    state: "dead" | "holds" | "shielded";
}

// Blast ray walk (damage.rs:620-680 rule) over structural layers at fixed
// distances: linear falloff, 0.65x per destroyed layer, a surviving layer
// zeroes everything behind it.
export function blastWalk(
    maxDamage: number,
    radius: number,
    layerDistances: number[],
    hp: number,
    targetDistance: number
): { layers: BlastLayer[]; target: number } {
    const falloff = (d: number): number =>
        d >= radius ? 0 : maxDamage * (1 - d / radius);
    const layers: BlastLayer[] = [];
    let destroyed = 0;
    let stopped = false;
    for (const d of layerDistances) {
        if (stopped) {
            layers.push({ incoming: 0, state: "shielded" });
            continue;
        }
        const incoming =
            falloff(d) * Math.pow(EXPLOSIVE_SECTION_TRANSMISSION, destroyed);
        if (incoming >= hp) {
            layers.push({ incoming, state: "dead" });
            destroyed += 1;
        } else {
            layers.push({ incoming, state: "holds" });
            stopped = true;
        }
    }
    const target = stopped
        ? 0
        : falloff(targetDistance) *
          Math.pow(EXPLOSIVE_SECTION_TRANSMISSION, destroyed);
    return { layers, target };
}

// The pressure the wave still carries at travel distance `d` along the centre
// ray: the same rules as blastWalk, sampled mid-flight. Zero once a surviving
// layer stopped the wave or past the radius. The blast scope derives every
// frame of its sweep from this; the game itself resolves a blast in one tick.
export function blastFront(
    maxDamage: number,
    radius: number,
    layerDistances: number[],
    hp: number,
    d: number
): { pressure: number; destroyed: number; stopped: boolean } {
    const falloff = (x: number): number =>
        x >= radius ? 0 : maxDamage * (1 - x / radius);
    let destroyed = 0;
    for (const dist of layerDistances) {
        if (dist > d) break;
        const incoming =
            falloff(dist) * Math.pow(EXPLOSIVE_SECTION_TRANSMISSION, destroyed);
        if (incoming >= hp) destroyed += 1;
        else return { pressure: 0, destroyed, stopped: true };
    }
    return {
        pressure:
            falloff(d) * Math.pow(EXPLOSIVE_SECTION_TRANSMISSION, destroyed),
        destroyed,
        stopped: false,
    };
}

// The two ceilings and the lower one (attitude.rs:69-91). Inertia is the
// hull's largest principal moment; the arm runs from the centre of mass to the
// outer face of the furthest live section, in world units.
export function torqueCeiling(torque: number, inertia: number): number {
    return inertia > 0 ? Math.max(torque, 0) / inertia : Infinity;
}
// The one crossing in the attitude model: the arm is measured off collider
// boxes in world units, the load limit is `MetersPerSecondSquared`, so the arm
// becomes meters before the division - exactly what the game does when it
// calls `AttitudeEnvelope::new(.., Meters::from_engine(arm))`
// (controller_section.rs:487-489). The result is rad/s^2 either way.
export function structuralCeiling(armUnits: number): number {
    const arm = Math.max(armUnits, 0) * METERS_PER_UNIT;
    return arm > 0 ? LOAD_LIMIT / arm : Infinity;
}
export function attitudeCeiling(
    torque: number,
    inertia: number,
    armUnits: number
): number {
    return Math.min(
        torqueCeiling(torque, inertia),
        structuralCeiling(armUnits)
    );
}

// The rate at which the centripetal load alone spends the whole structural
// budget: hold it and nothing is left to turn harder with (attitude.rs:108-113).
export function sustainedTurnRate(armUnits: number): number {
    return Math.sqrt(structuralCeiling(armUnits));
}

// A bang-bang 180 at `alpha` takes `2 * sqrt(pi / alpha)` (guidance.rs:307-308).
export function flipSeconds(alpha: number): number {
    return alpha > 0 ? 2 * Math.sqrt(Math.PI / alpha) : Infinity;
}

// SOI from mass alone: the distance where the raw inverse-square pull decays
// to the cutoff, floored at the body radius (gravity.rs:96-108).
export function soiRadius(mu: number, bodyRadius: number): number {
    return Math.max(Math.sqrt(mu / SOI_CUTOFF_ACCEL), bodyRadius);
}

// The pull at distance r (gravity.rs:301-333): inverse square off `mu`,
// clamped at the surface margin (no singularity slingshots), smoothstepped to
// exactly zero across the outer 15% of the SOI.
export function wellAccel(
    mu: number,
    r: number,
    bodyRadius: number,
    soi: number
): number {
    if (mu <= 0 || soi <= 0 || r >= soi) return 0; // gravity.rs:314-316
    const rEff = Math.max(r, bodyRadius + GRAVITY_SURFACE_MARGIN); // gravity.rs:319
    const base = mu / (rEff * rEff); // gravity.rs:320
    const fadeStart = soi * (1 - GRAVITY_FADE_FRACTION); // gravity.rs:324
    let fade = 1;
    if (r > fadeStart) {
        const t = clamp((soi - r) / Math.max(soi - fadeStart, 1e-12), 0, 1);
        fade = t * t * (3 - 2 * t); // gravity.rs:325-330
    }
    return base * fade; // gravity.rs:332
}

// gravity.rs:335-342. The ORBIT verb burns to this tangentially.
export function circularOrbitSpeed(mu: number, r: number): number {
    if (mu <= 0 || r <= 0) return 0;
    return Math.sqrt(mu / r);
}

// The band ORBIT will accept a ring in (guidance.rs:216-236): clear of the
// surface by the clearance factor, safely inside the fade band. Null when the
// band is empty (ORBIT refuses the well).
export function orbitBand(
    bodyRadius: number,
    soi: number
): { min: number; max: number } | null {
    const min = ORBIT_CLEARANCE_FACTOR * (bodyRadius + GRAVITY_SURFACE_MARGIN);
    const max = ORBIT_BAND_SAFETY * soi * (1 - GRAVITY_FADE_FRACTION);
    return min > max ? null : { min, max };
}

// Dominant-well pick over (id, pull) candidates (gravity.rs:344-372): the
// strongest wins, but an incumbent holds until a challenger clearly beats it
// - strictly more than `hysteresis x` the incumbent's pull (gravity.rs:365).
export function dominantWell(
    current: number | null,
    pulls: number[]
): number | null {
    let strongest = -1;
    let strongestPull = 0;
    pulls.forEach((p, i) => {
        if (p > 0 && (strongest < 0 || p > strongestPull)) {
            strongest = i;
            strongestPull = p;
        }
    });
    if (strongest < 0) return null;
    if (current !== null) {
        const incumbent = pulls[current] ?? 0;
        if (
            incumbent > 0 &&
            strongestPull <= incumbent * WELL_SWITCH_HYSTERESIS
        )
            return current;
    }
    return strongest;
}

// Lock-on dwell before a radar lock commits (radar.rs:239-252): base time
// stretched by range up to the reference, hard-clamped either side.
export function lockDwellSecs(distance: number): number {
    const reach = clamp(distance / LOCK_DWELL_REFERENCE_RANGE, 0, 1);
    const raw = LOCK_DWELL_BASE * (1 + LOCK_DWELL_RANGE_FACTOR * reach);
    return clamp(raw, LOCK_DWELL_MIN, LOCK_DWELL_MAX);
}

// Staged clearing on a tap (gesture.rs:125-175): one lock per tap, combat
// first; the travel branch is gated on weapons LOWERED.
export function clearStep(
    raised: boolean,
    combat: boolean,
    travel: boolean
): "combat" | "travel" | "none" {
    if (combat) return "combat";
    if (!raised && travel) return "travel";
    return "none";
}

// The contextual HUD model (crates/nova_hud/src/): which elements are up in
// a given situation. Cinematic clears every tier (lib.rs:152-162); the ammo
// layer opens on hot OR low OR reloading (situation.rs:47-49,
// ammo_readout.rs:180); the mode chip and destination marker follow the
// autopilot (flight_status.rs:302-335); the reticle and viewfinder follow
// the combat lock (torpedo_target.rs:408-415, target_inset.rs:658-707).
export interface HudSituationsModel {
    autopilot: boolean;
    combatLock: boolean;
    weaponsHot: boolean;
    lowAmmo: boolean;
    reloading: boolean;
    cinematic: boolean;
}
export interface HudElementState {
    name: string;
    kind: "instrument" | "chrome" | "status";
    on: boolean;
    detail: string;
}
export function hudElements(s: HudSituationsModel): HudElementState[] {
    const e = (
        name: string,
        kind: HudElementState["kind"],
        on: boolean,
        detail: string
    ): HudElementState => ({
        name,
        kind,
        on: on && !s.cinematic,
        detail: s.cinematic ? "cleared at Cinematic" : detail,
    });
    const ammoOpen = s.weaponsHot || s.lowAmmo || s.reloading;
    return [
        e(
            "Velocity sphere",
            "instrument",
            true,
            s.autopilot ? "cyan - the autopilot palette" : "white and blue"
        ),
        e(
            "Speed chip",
            "instrument",
            true,
            s.autopilot ? "grown 1.14x - it is the number you fly by" : "steady"
        ),
        e(
            "Mode chip",
            "instrument",
            s.autopilot,
            s.autopilot
                ? "verb and phase, e.g. AP GOTO - BURN"
                : "arrives with the autopilot"
        ),
        e(
            "Destination marker",
            "instrument",
            s.autopilot,
            s.autopilot
                ? "GOTO and ORBIT only - STOP flies without one"
                : "arrives with GOTO or ORBIT"
        ),
        e(
            "Combat reticle + DST/CLS",
            "instrument",
            s.combatLock,
            !s.combatLock
                ? "arrives with a combat lock"
                : s.weaponsHot
                  ? "readout grown; pulses while firing"
                  : "idle - decays after 30 s, wind-down over the last 5"
        ),
        e(
            "Target viewfinder",
            "chrome",
            s.combatLock,
            !s.combatLock
                ? "arrives with the combat lock"
                : s.weaponsHot
                  ? "frame hot-red, corner ticks out"
                  : "frame steel"
        ),
        e(
            "Ammo gauges",
            "instrument",
            ammoOpen,
            !ammoOpen
                ? "raised by hot weapons, low ammo or a reload"
                : s.lowAmmo && !s.reloading
                  ? "forced up: amber warn breath at a quarter magazine"
                  : s.reloading
                    ? "forced up: reload pulse"
                    : "lit while weapons are hot"
        ),
        e(
            "Turret lead pips",
            "instrument",
            true,
            s.weaponsHot ? "red while hot" : "amber"
        ),
        e(
            "Bore sight (a hull with a railgun)",
            "instrument",
            s.weaponsHot,
            s.weaponsHot
                ? "the line of fire, a ring on each section it would gut"
                : "arrives with hot weapons; dimmed through a reload"
        ),
        e(
            "Allegiance markers",
            "instrument",
            true,
            "deliberately not contextual - a brawl stays legible"
        ),
        e(
            "Keybind dock",
            "chrome",
            true,
            s.combatLock
                ? "RADAR chip inverted - the lock is what you would change"
                : "only the verbs that would do something right now"
        ),
        e("Status bar", "status", true, "fps and version"),
    ];
}

// The three-state relation model (crates/nova_gameplay/src/relations.rs:53-61).
// "none" = a body with no allegiance at all (asteroids, debris).
export type Side = "player" | "enemy" | "neutral" | "none";
export function relation(a: Side, b: Side): "own" | "hostile" | "neutral" {
    const combatant = (s: Side): boolean => s === "player" || s === "enemy";
    if (!combatant(a) || !combatant(b)) return "neutral";
    return a === b ? "own" : "hostile";
}

// The hull's average turn rate from its DERIVED attitude ceiling
// (guidance.rs:307-318): a bang-bang 180 at `alpha` averages
// `sqrt(pi * alpha) / 2`, scaled and clamped by the flight settings.
export function hullTurnRate(alpha: number): number {
    const optimum = Math.sqrt(Math.PI * Math.max(alpha, 0)) * 0.5;
    const lo = (TURN_RATE_MIN_DEG * Math.PI) / 180;
    const hi = (TURN_RATE_MAX_DEG * Math.PI) / 180;
    return clamp(optimum * TURN_RATE_SCALE, lo, hi);
}

// The arrival speed envelope (guidance.rs:25-60), gravity-free form: the
// fastest closing speed from which a flip taking `lead` seconds still stops
// in `distance`. With lead 0 this is exactly sqrt(2 * a * margin * d).
export function arrivalSpeedLimit(
    distance: number,
    accel: number,
    lead: number
): number {
    const braking = Math.max(accel, 0) * DECEL_MARGIN;
    const d = Math.max(distance, 0);
    if (braking <= 0 || d <= 0) return 0;
    const l = Math.max(lead, 0);
    return (
        -braking * l + Math.sqrt(braking * braking * l * l + 2 * braking * d)
    );
}

// The flip line (guidance.rs:86-118), gravity-free: GOTO swings retrograde
// once `distance <= standoff + v * lead + v^2 / (2 * a * margin)`.
export function gotoFlipDistance(
    v: number,
    standoff: number,
    accel: number,
    lead: number
): number {
    const braking = Math.max(accel, 0) * DECEL_MARGIN;
    return standoff + v * lead + (v * v) / (2 * braking);
}

export interface GotoSample {
    t: number;
    x: number; // travelled, world units from the start point
    v: number; // closing speed, world units per second
    phase: "burn" | "flip" | "brake" | "settle";
}

// A 1D replay of a GOTO leg: burn on the envelope, coast through the flip,
// brake at margin, ease the last stretch to rest on RCS. Simplified
// presentation (no gravity, one forward drive group so the brake angle is a
// full 180); the envelope, flip and standoff rules are the game's own.
export function gotoSim(
    targetDistance: number,
    targetRadius: number,
    accel: number
): {
    samples: GotoSample[];
    standoff: number;
    flipT: number;
    flipX: number;
    peakV: number;
    duration: number;
} {
    // The whole arrival model: target radius + mover radius + margin. The
    // corvette this scope flies contributes its own structural arm, so the
    // leg parks its HULL FACE one margin off the target's surface.
    const standoff =
        ARRIVAL_STANDOFF + Math.max(targetRadius, 0) + CORVETTE_ARM_U;
    const park = targetDistance - standoff;
    const turnRate = hullTurnRate(structuralCeiling(CORVETTE_ARM_U));
    const lead = Math.PI / turnRate + ARRIVAL_SPOOL_PAD; // autopilot.rs:209
    const braking = accel * DECEL_MARGIN;
    const dt = 1 / 60;
    const samples: GotoSample[] = [];
    let t = 0;
    let x = 0;
    let v = 0;
    let phase: GotoSample["phase"] = "burn";
    let flipUntil = 0;
    let flipT = 0;
    let flipX = 0;
    let peakV = 0;
    samples.push({ t, x, v, phase });
    // Hard cap the sim at 20 scope minutes; every legal slider setting
    // resolves far earlier.
    while (t < 1200) {
        t += dt;
        if (phase === "burn") {
            v += accel * dt;
            const remaining = park - x;
            if (v * lead + (v * v) / (2 * braking) >= remaining) {
                phase = "flip";
                flipUntil = t + lead;
                flipT = t;
                flipX = x;
            }
        } else if (phase === "flip") {
            if (t >= flipUntil) phase = "brake";
        } else if (phase === "brake") {
            // Outside the standoff the envelope floors the approach at
            // 1.5 world units per second; inside it the drive brakes for zero, and only under
            // the 2.0 world units per second RCS cap do the fine jets take over
            // (autopilot.rs:568-577).
            v = Math.max(v - braking * dt, x < park ? MIN_APPROACH_SPEED : 0);
            if (x >= park && v <= RCS_SPEED_CAP) phase = "settle";
        } else {
            // RCS settle (autopilot.rs:568-602): main drive cut, the fine
            // jets brake the last stretch.
            v = Math.max(v - RCS_ACCEL * dt, 0);
        }
        x += v * dt;
        peakV = Math.max(peakV, v);
        samples.push({ t, x, v, phase });
        if (phase === "settle" && v <= STOP_SPEED_EPSILON) break;
    }
    return { samples, standoff, flipT, flipX, peakV, duration: t };
}

// A weapon's whole ammunition rule: what it holds, how fast it spends it, and
// the quiet batch that brings it back.
export interface AmmoRule {
    capacity: number;
    rate: number; // shots per second
    delay: number; // quiet seconds one batch costs
    amount: number; // shots one batch returns
}

// The rate a weapon holds forever by firing each batch the moment it lands:
// `amount / (delay + amount / rate)`. standard.rs:594 works the shipped PDC
// through it - 200 / (3 + 200/100) = 40 rounds/s against a 100/s cyclic rate.
export function sustainedRate(w: AmmoRule): number {
    return w.amount / (w.delay + w.amount / w.rate);
}

// Quiet seconds from empty back to full: whole batches, since a partial one
// never lands (ammo.rs:172-174).
export function refillSecs(w: AmmoRule): number {
    return Math.ceil(w.capacity / w.amount) * w.delay;
}

export interface AmmoSample {
    t: number;
    rounds: number;
    firing: boolean;
}

// Replay a burst-and-quiet trigger pattern against the reload rule. Mirrors
// `SectionReload::advance` (ammo.rs:161-180): while shots are landing the
// clock is pinned at zero, and it only accumulates through a quiet stretch -
// or through a stretch where the trigger is down on an EMPTY weapon, which
// reloads exactly like silence does.
export function ammoTrace(
    w: AmmoRule,
    burst: number,
    quiet: number,
    span: number,
    step = 0.02
): AmmoSample[] {
    const cycle = burst + quiet;
    let rounds = w.capacity;
    let clock = 0;
    const out: AmmoSample[] = [{ t: 0, rounds, firing: burst > 0 }];
    for (let t = 0; t < span - 1e-9; t += step) {
        const firing = cycle > 0 && t % cycle < burst;
        if (firing && rounds > 0) {
            rounds = Math.max(0, rounds - w.rate * step);
            clock = 0;
        } else {
            clock += step;
            while (w.delay > 0 && clock >= w.delay) {
                clock -= w.delay;
                rounds = Math.min(w.capacity, rounds + w.amount);
                if (rounds >= w.capacity) {
                    clock = 0;
                    break;
                }
            }
        }
        out.push({ t: t + step, rounds, firing });
    }
    return out;
}

// Whether the mount can put its barrel on a target at this elevation. The
// reachable band is derived from the elevation hinge's own limits, never from
// a separate occlusion test (arc.rs:46-102).
export function turretBears(elevationDeg: number): boolean {
    return (
        elevationDeg >= TURRET_DEPRESSION_DEG &&
        elevationDeg <= TURRET_ELEVATION_DEG
    );
}

// The fraction of the whole sky one mount can bear on. Traverse is unbounded,
// so the blind volume is exactly the cap below the depression floor and the
// covered fraction is `(1 - sin(floor)) / 2`.
export function turretSkyFraction(): number {
    return (1 - Math.sin((TURRET_DEPRESSION_DEG * Math.PI) / 180)) / 2;
}

// Seconds to bring the barrel round. Traverse and elevation are separate
// hinges turning at the same rate at the same time, so the swing costs the
// LARGER of the two, not their sum.
export function turretSlewSecs(
    traverseDeg: number,
    elevationDeg: number
): number {
    return (
        Math.max(Math.abs(traverseDeg), Math.abs(elevationDeg)) /
        TURRET_SLEW_DEG_S
    );
}

// How much of the authored weave amplitude survives at `distance` from the
// target (projectile.rs:435-439): full beyond three blast radii, linear to
// zero half a radius out, so the run-in ends on the aim point.
export function weaveFade(distance: number, blastRadius: number): number {
    const terminal = blastRadius * WEAVE_ZERO_RADII;
    const full = blastRadius * WEAVE_FULL_RADII;
    return clamp((distance - terminal) / (full - terminal), 0, 1);
}

// ---- DOM helpers ----------------------------------------------------------

// ---- lance corridor -------------------------------------------------------

// The block the corridor scope shoots: BUILD CELLS on a lattice, `x` across,
// `y` up, `layer` deep along the bore, with the bore through (0, 0). A cell is
// one world unit, so 10 m on a side, and every length in this model - the rake
// radius, the offsets, the tip's travel - is counted in cells. It is the stand
// bank of examples/systems/system_railgun_lance.rs in miniature - the same
// 200 hp cells on the same lattice - so the walk below is checked against what
// the game measured there (tests/widgets.test.ts).
export interface CorridorCell {
    x: number;
    y: number;
    layer: number;
    /** Lateral distance from the bore to the cell's nearest point. */
    offset: number;
    /**
     * How far past the entry face the slug's TIP is when the shot first
     * reaches this cell: the tip's own contact down the bore column, the
     * trailing sphere's for everything beside it. Infinity means never.
     */
    reach: number;
    charged: boolean;
}

export interface CorridorResult {
    /** Every cell in charge order; the charged ones lead. */
    cells: CorridorCell[];
    taken: number;
    /** Power one crossing costs at the slug's speed. */
    cost: number;
    spent: number;
    /** Cells taken per layer, entry face first. */
    profile: number[];
    /** Hull health the shot destroyed. */
    removed: number;
}

// The rake rule (crates/nova_gameplay/src/rounds.rs:714, `sweep_raking`). A
// sphere of the authored radius trails the tip by exactly that radius, so
// its front is tangent to the tip and what it sweeps is a cylinder BEHIND
// the tip. A cell `offset` off the bore is inside that cylinder - and so is
// charged - once the tip is `radius - sqrt(radius^2 - offset^2)` past the
// cell's near face; the bore column is the tip's own contact, and a needle
// (radius 0) reaches the bore column and nothing else. Contacts are charged
// by travel depth, then from the axis outward (pass three), each paying
// `max health / pierce multiplier` out of the one budget (damage.rs
// `pierce_remainder`), and the bite that empties the budget still lands. A
// slug at 15 000 m/s pins that multiplier at its 3.0 ceiling. The budget is
// walked in f32 exactly as the game walks it, because 27 x (200 / 3) IS 1800
// and only the rounding decides whether a 28th crossing lands - it does.
//
// `radius` is in CELLS, not meters: the sweep casts the authored corridor in
// world units (railgun_section/firing.rs:215-218), and the lattice counts the
// same units. LANCE_RAKE_RADIUS_CELLS is the shipped 10 m stated that way.
export function lanceCorridor(
    radius: number,
    hp: number,
    width: number,
    height: number,
    depth: number,
    power = LANCE_SLUG_POWER
): CorridorResult {
    const cost = Math.fround(hp / PIERCE_POWER_CEILING);
    const half = (n: number): number => Math.floor(n / 2);
    const cells: CorridorCell[] = [];
    for (let layer = 0; layer < depth; layer++) {
        for (let y = -half(height); y <= half(height); y++) {
            for (let x = -half(width); x <= half(width); x++) {
                const offset = Math.hypot(
                    Math.max(Math.abs(x) - 0.5, 0),
                    Math.max(Math.abs(y) - 0.5, 0)
                );
                let reach = Infinity;
                if (offset === 0) {
                    reach = layer;
                } else if (offset <= radius) {
                    reach =
                        layer +
                        radius -
                        Math.sqrt(radius * radius - offset * offset);
                }
                cells.push({ x, y, layer, offset, reach, charged: false });
            }
        }
    }
    cells.sort(
        (a, b) =>
            a.reach - b.reach ||
            a.offset - b.offset ||
            a.layer - b.layer ||
            a.y - b.y ||
            a.x - b.x
    );
    const profile = new Array<number>(depth).fill(0);
    let remaining = Math.fround(power);
    let taken = 0;
    for (const cell of cells) {
        if (cell.reach === Infinity || remaining <= 0) break;
        cell.charged = true;
        taken += 1;
        profile[cell.layer] += 1;
        remaining = Math.fround(remaining - cost);
    }
    return {
        cells,
        taken,
        cost,
        spent: taken * cost,
        profile,
        removed: taken * Math.min(hp, LANCE_SLUG_DAMAGE),
    };
}

// The three weapon families' reach and time of flight to a target `range`
// METERS out. Reach is never authored: a round's is muzzle speed times its
// lifetime (config.rs:135-144), the slug's the same (standard.rs:929,:971),
// and a torpedo's is the along-the-line speed it settles at over the bay's
// lifetime. Every speed and reach here is SI, so the flight time is seconds
// with no conversion anywhere. Infinity: the shot never arrives.
export interface ReachRung {
    name: string;
    reach: number;
    flightSecs: number;
}
export function reachLadder(range: number): ReachRung[] {
    const tof = (reach: number, speed: number): number =>
        range <= reach ? range / speed : Infinity;
    return [
        {
            name: "PDC",
            reach: PDC_REACH,
            flightSecs: tof(PDC_REACH, PDC_MUZZLE_SPEED),
        },
        {
            name: "Lance",
            reach: LANCE_REACH,
            flightSecs: tof(LANCE_REACH, LANCE_SLUG_SPEED),
        },
        {
            name: "Serpent",
            reach: SERPENT_REACH,
            flightSecs: tof(SERPENT_REACH, SERPENT_CRUISE),
        },
        {
            name: "Lance torpedo",
            reach: LANCE_TORPEDO_REACH,
            flightSecs: tof(LANCE_TORPEDO_REACH, LANCE_TORPEDO_CRUISE),
        },
    ];
}

function el<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className?: string,
    text?: string
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
}

const SVG_NS = "http://www.w3.org/2000/svg";
function svgEl<K extends keyof SVGElementTagNameMap>(
    tag: K,
    attrs: Record<string, string>,
    text?: string
): SVGElementTagNameMap[K] {
    const node = document.createElementNS(SVG_NS, tag);
    for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
    if (text !== undefined) node.textContent = text;
    return node;
}

interface Control {
    row: HTMLElement;
    input: HTMLInputElement;
}

// One labeled fader row. `format` renders the live value readout.
function control(
    label: string,
    min: number,
    max: number,
    step: number,
    value: number,
    format: (v: number) => string,
    onInput: () => void
): Control {
    const row = el("label", "widget__control");
    const name = el("span", undefined, `${label}: `);
    const val = el("span", "widget__value", format(value));
    // The label column is sized to its content, so a reading that changes
    // length mid-drag ("10 m (shipped)" to "15 m") would resize the fader
    // under the pointer and the thumb would jump. The widest reading the
    // fader can show sits hidden under the live one, so the column holds.
    let widest = format(value);
    const steps = Math.min(4000, Math.round((max - min) / step));
    for (let i = 0; i <= steps; i++) {
        const reading = format(min + i * step);
        if (reading.length > widest.length) widest = reading;
    }
    const ghost = el("span", "widget__value widget__value--ghost", widest);
    ghost.setAttribute("aria-hidden", "true");
    const reading = el("span", "widget__reading");
    reading.appendChild(val);
    reading.appendChild(ghost);
    name.appendChild(reading);
    const input = el("input", "widget__slider");
    input.type = "range";
    input.min = String(min);
    input.max = String(max);
    input.step = String(step);
    input.value = String(value);
    input.addEventListener("input", () => {
        val.textContent = format(Number(input.value));
        onInput();
    });
    row.appendChild(name);
    row.appendChild(input);
    return { row, input };
}

function header(host: HTMLElement, title: string, hint: string): void {
    host.appendChild(el("p", "widget__tag", "interactive"));
    host.appendChild(el("p", "widget__title", title));
    host.appendChild(el("p", "widget__hint", hint));
}

function stat(row: HTMLElement, label: string): HTMLElement {
    const cell = el("span", undefined, `${label} `);
    const value = el("b");
    cell.appendChild(value);
    row.appendChild(cell);
    return value;
}

function sectionCell(word: string, detail: string, state: string): HTMLElement {
    const cell = el("div", "widget__cell");
    if (state) cell.classList.add(state);
    cell.appendChild(el("b", undefined, word));
    cell.appendChild(document.createTextNode(detail));
    return cell;
}

function numAttr(host: HTMLElement, name: string, fallback: number): number {
    const raw = host.dataset[name];
    const parsed = raw === undefined ? NaN : Number(raw);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

// ---- scope transport ------------------------------------------------------

// The animated scope widgets replay a resolved outcome in "scope time"; this
// deck owns the clock: a PLAY/REPLAY key, a scrub fader and a T+ readout.
// Under prefers-reduced-motion nothing autoplays and PLAY cuts straight to
// the resolved end state - the scrub still steps every frame by hand.
function reducedMotion(): boolean {
    return (
        typeof window.matchMedia === "function" &&
        window.matchMedia("(prefers-reduced-motion: reduce)").matches
    );
}

interface Transport {
    row: HTMLElement;
    play: () => void;
    seekEnd: () => void;
}

function makeTransport(
    duration: () => number,
    render: (t: number) => void
): Transport {
    let t = 0;
    let raf = 0;
    let last = 0;
    const btn = el("button", "widget__btn", "PLAY");
    btn.type = "button";
    const scrub = el("input", "widget__slider widget__scrub");
    scrub.type = "range";
    scrub.min = "0";
    scrub.max = "1000";
    scrub.step = "1";
    scrub.value = "0";
    scrub.setAttribute("aria-label", "Scrub the scope timeline");
    const clock = el("span", "widget__value widget__clock", "T+0.00s");

    const apply = (): void => {
        render(t);
        scrub.value = String(
            Math.round((t / Math.max(duration(), 1e-6)) * 1000)
        );
        clock.textContent = `T+${t.toFixed(2)}s`;
    };
    const stop = (): void => {
        if (raf) cancelAnimationFrame(raf);
        raf = 0;
    };
    const tick = (now: number): void => {
        t = Math.min(duration(), t + (now - last) / 1000);
        last = now;
        apply();
        if (t >= duration()) {
            raf = 0;
            return;
        }
        raf = requestAnimationFrame(tick);
    };
    const play = (): void => {
        stop();
        btn.textContent = "REPLAY";
        if (reducedMotion()) {
            t = duration();
            apply();
            return;
        }
        t = 0;
        last = performance.now();
        apply();
        raf = requestAnimationFrame(tick);
    };
    const seekEnd = (): void => {
        stop();
        btn.textContent = "REPLAY";
        t = duration();
        apply();
    };
    btn.addEventListener("click", play);
    scrub.addEventListener("input", () => {
        stop();
        btn.textContent = "REPLAY";
        t = (Number(scrub.value) / 1000) * duration();
        render(t);
        clock.textContent = `T+${t.toFixed(2)}s`;
    });

    const row = el("div", "widget__transport");
    row.appendChild(btn);
    row.appendChild(scrub);
    row.appendChild(clock);
    return { row, play, seekEnd };
}

// ---- aim-decay ------------------------------------------------------------

// Tracking lag vs frame rate for the old per-frame servo and the current
// per-second one, against the fire gate. data-cross overrides the crossing
// rate (deg/s).
function initAimDecay(host: HTMLElement): void {
    const cross = numAttr(host, "cross", 9);
    header(
        host,
        "Tracking lag vs frame rate",
        `A target crossing your gun's line of sight at ${cross} deg/s. The servo ` +
            "used to catch up by a fixed fraction of the error per FRAME, so " +
            "slow frames meant more lag - and the guns went quiet exactly " +
            "when the machine struggled. It now corrects per SECOND: the " +
            "same tracking at any frame rate."
    );

    // Plot geometry: fps 10..120 across, lag 0..2.8 deg up.
    const X0 = 44;
    const X1 = 550;
    const Y0 = 204;
    const Y1 = 12;
    const FPS_MIN = 10;
    const FPS_MAX = 120;
    const LAG_MAX = 2.8;
    const x = (fps: number): number =>
        X0 + ((fps - FPS_MIN) / (FPS_MAX - FPS_MIN)) * (X1 - X0);
    const y = (lag: number): number =>
        Y0 - (Math.min(lag, LAG_MAX) / LAG_MAX) * (Y0 - Y1);

    const svg = svgEl("svg", {
        viewBox: "0 0 560 230",
        role: "img",
        "aria-label":
            "Tracking lag in degrees against frame rate, for the old " +
            "per-frame servo and the current per-second servo, with the " +
            "fire gate marked.",
    });
    // Quiet grid + axis text.
    for (const lag of [0, 1, 2]) {
        svg.appendChild(
            svgEl("line", {
                x1: String(X0),
                y1: String(y(lag)),
                x2: String(X1),
                y2: String(y(lag)),
                class: "widget-mark--grid",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(X0 - 6),
                    y: String(y(lag) + 3),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                lag === 2 ? "2 deg" : String(lag)
            )
        );
    }
    for (const fps of [30, 60, 90, 120]) {
        // The last tick label carries the unit and is end-anchored so it
        // stays inside the viewBox instead of clipping at the right edge.
        const last = fps === 120;
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(last ? X1 + 6 : x(fps)),
                    y: String(Y0 + 16),
                    "text-anchor": last ? "end" : "middle",
                    class: "widget-mark--axis",
                },
                last ? "120 fps" : String(fps)
            )
        );
    }
    const path = (lag: (fps: number) => number): string => {
        const points: string[] = [];
        for (let fps = FPS_MIN; fps <= FPS_MAX; fps += 2) {
            points.push(`${x(fps).toFixed(1)},${y(lag(fps)).toFixed(1)}`);
        }
        return `M${points.join(" L")}`;
    };
    const lagOld = (fps: number): number => aimLagOldDeg(fps, cross);
    const lagNow = (fps: number): number => aimLagNowDeg(fps, cross);
    // The fire gate is the fault line: over it, the gun holds.
    svg.appendChild(
        svgEl("line", {
            x1: String(X0),
            y1: String(y(FIRE_GATE_DEG)),
            x2: String(X1),
            y2: String(y(FIRE_GATE_DEG)),
            class: "widget-mark--gate",
        })
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(X1 - 2),
                y: String(y(FIRE_GATE_DEG) - 5),
                "text-anchor": "end",
                class: "widget-mark--label-gate",
            },
            "fire gate 0.92 deg - over it the gun holds"
        )
    );
    svg.appendChild(
        svgEl("path", { d: path(lagOld), class: "widget-mark--old" })
    );
    svg.appendChild(
        svgEl("path", { d: path(lagNow), class: "widget-mark--now" })
    );
    // Direct labels beat a legend box at this size.
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(x(18) + 6),
                y: String(y(lagOld(18)) - 8),
                class: "widget-mark--label-old",
            },
            "per-frame servo (old)"
        )
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(x(78)),
                y: String(y(lagNow(78)) - 10),
                class: "widget-mark--label-now",
            },
            "per-second servo (current)"
        )
    );
    const cursor = svgEl("line", {
        x1: "0",
        y1: String(Y1),
        x2: "0",
        y2: String(Y0),
        class: "widget-mark--cursor",
    });
    const dotOld = svgEl("circle", { r: "4", class: "widget-mark--dot-old" });
    const dotNow = svgEl("circle", { r: "4", class: "widget-mark--dot-now" });
    svg.appendChild(cursor);
    svg.appendChild(dotOld);
    svg.appendChild(dotNow);

    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stats");
    const oldStat = stat(stats, "per-frame lag");
    const nowStat = stat(stats, "per-second lag");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const fps = Number(fpsControl.input.value);
        const o = lagOld(fps);
        const n = lagNow(fps);
        cursor.setAttribute("x1", String(x(fps)));
        cursor.setAttribute("x2", String(x(fps)));
        dotOld.setAttribute("cx", String(x(fps)));
        dotOld.setAttribute("cy", String(y(o)));
        dotNow.setAttribute("cx", String(x(fps)));
        dotNow.setAttribute("cy", String(y(n)));
        oldStat.textContent = `${o.toFixed(2)} deg`;
        nowStat.textContent = `${n.toFixed(2)} deg`;
        readout.classList.remove("is-fault");
        if (o > FIRE_GATE_DEG && n <= FIRE_GATE_DEG) {
            readout.textContent =
                "Old servo: over the gate - the gun refuses to fire. " +
                "Current servo: on target and shooting.";
        } else if (o > FIRE_GATE_DEG) {
            readout.textContent =
                "Both servos over the gate at this crossing rate - guns hold.";
            readout.classList.add("is-fault");
        } else {
            readout.textContent =
                "Both inside the gate - the difference only shows when " +
                "frames get slow.";
        }
    };
    const fpsControl = control(
        "Frame rate",
        10,
        120,
        1,
        60,
        (v) => `${v} fps`,
        update
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(fpsControl.row);

    host.appendChild(controls);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    update();
}

// ---- round-travel ---------------------------------------------------------

// A side-profile firing-range scope: the same stack of sections in two lanes,
// a kinetic slug and a pierce dart replayed crossing it in scope time while
// each round's budget drains by its own rule. data-sections / data-hp
// override the fixture.
function initRoundTravel(host: HTMLElement): void {
    const sections = numAttr(host, "sections", 5);
    const hp = numAttr(host, "hp", LIGHT_HULL_HP);
    // Presentation only: impacts resolve instantly in game; the scope replays
    // the walk at a legible speed.
    const ROUND_SPEED = 260; // px of round travel per scope second
    header(
        host,
        "Round scope: one round vs a section stack",
        `${sections} light hull sections, ${hp} hp each (the catalog value), ` +
            "at full health. Kinetic spends its damage and stops at the " +
            "first section it cannot destroy; Pierce deals its full damage " +
            "to every section it crosses and spends a separate " +
            `${PIERCE_BASE_POWER}-point power budget on thickness, at most ` +
            `${MAX_PIERCE_LAYERS} sections deep. Play the tape and watch ` +
            "both rounds spend their budgets."
    );

    // Scope geometry.
    const X_START = 108;
    const X0S = 150;
    const XEND = 548;
    const GAP = 8;
    const w = (XEND - X0S - (sections - 1) * GAP) / sections;
    const secLeft = (i: number): number => X0S + i * (w + GAP);
    const LANES = [
        { label: "KINETIC / SLUG", top: 34, cy: 62 },
        { label: "PIERCE / DART", top: 146, cy: 174 },
    ];
    const BAR = { x: 12, w: 120, h: 8 };

    const svg = svgEl("svg", {
        viewBox: "0 0 560 240",
        role: "img",
        "aria-label":
            "Round scope: a kinetic slug and a pierce dart each crossing " +
            "the same stack of sections in a side profile, with each " +
            "round's budget draining as it travels.",
    });
    interface LaneCell {
        rect: SVGRectElement;
        word: SVGTextElement;
        detail: SVGTextElement;
    }
    interface Lane {
        cells: LaneCell[];
        round: SVGRectElement;
        impact: SVGCircleElement;
        barFill: SVGRectElement;
        barText: SVGTextElement;
        endText: SVGTextElement;
    }
    const buildLane = (which: 0 | 1): Lane => {
        const geo = LANES[which];
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(BAR.x),
                    y: String(geo.top - 8),
                    class: "widget-mark--word",
                },
                geo.label
            )
        );
        svg.appendChild(
            svgEl("rect", {
                x: String(BAR.x),
                y: String(geo.top + 6),
                width: String(BAR.w),
                height: String(BAR.h),
                rx: "2",
                class: "widget-mark--barframe",
            })
        );
        const barFill = svgEl("rect", {
            x: String(BAR.x),
            y: String(geo.top + 6),
            width: String(BAR.w),
            height: String(BAR.h),
            rx: "2",
            class: "widget-mark--barfill",
        });
        svg.appendChild(barFill);
        const barText = svgEl(
            "text",
            {
                x: String(BAR.x),
                y: String(geo.top + 30),
                class: "widget-mark--detail",
            },
            ""
        );
        svg.appendChild(barText);
        const cells: LaneCell[] = [];
        for (let i = 0; i < sections; i++) {
            const rect = svgEl("rect", {
                x: String(secLeft(i)),
                y: String(geo.top),
                width: String(w),
                height: "52",
                rx: "2",
                class: "widget-mark--section",
            });
            svg.appendChild(rect);
            const word = svgEl(
                "text",
                {
                    x: String(secLeft(i) + w / 2),
                    y: String(geo.cy - 2),
                    "text-anchor": "middle",
                    class: "widget-mark--word",
                },
                ""
            );
            const detail = svgEl(
                "text",
                {
                    x: String(secLeft(i) + w / 2),
                    y: String(geo.cy + 14),
                    "text-anchor": "middle",
                    class: "widget-mark--detail",
                },
                ""
            );
            svg.appendChild(word);
            svg.appendChild(detail);
            cells.push({ rect, word, detail });
        }
        const round = svgEl("rect", {
            x: "0",
            y: String(which === 0 ? geo.cy - 3 : geo.cy - 1.5),
            width: String(which === 0 ? 16 : 26),
            height: String(which === 0 ? 6 : 3),
            rx: "1.5",
            class: which === 0 ? "widget-mark--slug" : "widget-mark--dart",
        });
        svg.appendChild(round);
        const impact = svgEl("circle", {
            cy: String(geo.cy),
            r: "0",
            class: "widget-mark--impact",
            visibility: "hidden",
        });
        svg.appendChild(impact);
        const endText = svgEl(
            "text",
            {
                x: String(XEND),
                y: String(geo.top + 66),
                "text-anchor": "end",
                class: "widget-mark--detail",
            },
            ""
        );
        svg.appendChild(endText);
        return { cells, round, impact, barFill, barText, endText };
    };
    const laneK = buildLane(0);
    const laneP = buildLane(1);
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    // Inset: the two closing-speed clamp curves (damage.rs:253 and :265).
    // The axis is WORLD UNITS per second, because that is what the curves in
    // damage.rs read; every tick prints the same speed in m/s.
    const IX0 = 44;
    const IX1 = 548;
    const IY0 = 118;
    const IY1 = 16;
    const M_MAX = 3.2;
    const ix = (v: number): number =>
        IX0 + ((v - 10) / (400 - 10)) * (IX1 - IX0);
    const iy = (m: number): number => IY0 - (m / M_MAX) * (IY0 - IY1);
    const inset = svgEl("svg", {
        viewBox: "0 0 560 145",
        role: "img",
        "aria-label":
            "Closing-speed clamps: kinetic damage multiplier clamped " +
            "0.25 to 2.0 and pierce power multiplier clamped 0.5 to 3.0, " +
            "with a cursor at the selected closing speed.",
    });
    for (const m of [1, 2, 3]) {
        inset.appendChild(
            svgEl("line", {
                x1: String(IX0),
                y1: String(iy(m)),
                x2: String(IX1),
                y2: String(iy(m)),
                class: "widget-mark--grid",
            })
        );
        inset.appendChild(
            svgEl(
                "text",
                {
                    x: String(IX0 - 6),
                    y: String(iy(m) + 3),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                `x${m}`
            )
        );
    }
    for (const v of [100, 200, 300, 400]) {
        const lastTick = v === 400;
        inset.appendChild(
            svgEl(
                "text",
                {
                    x: String(lastTick ? IX1 + 6 : ix(v)),
                    y: String(IY0 + 14),
                    "text-anchor": lastTick ? "end" : "middle",
                    class: "widget-mark--axis",
                },
                lastTick
                    ? engineMetersPerSec(v)
                    : numText(v * METERS_PER_UNIT, 0)
            )
        );
    }
    const insetPath = (mult: (v: number) => number): string => {
        const pts: string[] = [];
        for (let v = 10; v <= 400; v += 5) {
            pts.push(`${ix(v).toFixed(1)},${iy(mult(v)).toFixed(1)}`);
        }
        return `M${pts.join(" L")}`;
    };
    inset.appendChild(
        svgEl("path", {
            d: insetPath(kineticDamageMultiplier),
            class: "widget-mark--now",
        })
    );
    inset.appendChild(
        svgEl("path", {
            d: insetPath(piercePowerMultiplier),
            class: "widget-mark--old",
        })
    );
    inset.appendChild(
        svgEl(
            "text",
            {
                x: String(ix(280)),
                y: String(iy(kineticDamageMultiplier(280)) + 16),
                class: "widget-mark--label-now",
            },
            "kinetic damage x"
        )
    );
    inset.appendChild(
        svgEl(
            "text",
            {
                x: String(ix(238)),
                y: String(iy(piercePowerMultiplier(238)) - 10),
                class: "widget-mark--label-old",
            },
            "pierce power x"
        )
    );
    const insetCursor = svgEl("line", {
        y1: String(IY1),
        y2: String(IY0),
        class: "widget-mark--cursor",
    });
    const insetDotK = svgEl("circle", {
        r: "4",
        class: "widget-mark--dot-now",
    });
    const insetDotP = svgEl("circle", {
        r: "4",
        class: "widget-mark--dot-old",
    });
    inset.appendChild(insetCursor);
    inset.appendChild(insetDotK);
    inset.appendChild(insetDotP);
    const insetPlot = el("div", "widget__plot");
    insetPlot.appendChild(inset);

    const kinStats = el("div", "widget__stats");
    const kinScale = stat(kinStats, "kinetic punch");
    const kinDead = stat(kinStats, "destroyed");
    const kinLeft = stat(kinStats, "carries on with");
    const prcStats = el("div", "widget__stats");
    const prcCost = stat(prcStats, "pierce crossing cost");
    const prcRaked = stat(prcStats, "sections raked");
    const prcTotal = stat(prcStats, "total dealt");
    const note = el(
        "p",
        "widget__note",
        "For scale: the shipped PDCs author 4.0 damage per kinetic round " +
            "and 2.0 per pierce round, and chip a section down over a " +
            "burst - the sliders span heavier single rounds so the travel " +
            "rules are visible in one hit."
    );

    // The resolved outcome and the replay schedule, rebuilt per parameter
    // change. All states come from the exported pure walks.
    let kin = kineticWalk(100, REFERENCE_CLOSING_SPEED, sections, hp);
    let prc = pierceWalk(100, REFERENCE_CLOSING_SPEED, sections, hp);
    let kinResolveT: (number | undefined)[] = [];
    let kinRemAfter: number[] = [];
    let kinEndX = XEND;
    let kinStopIdx = -1;
    let prcResolveT: (number | undefined)[] = [];
    let prcEndX = XEND;
    let prcReason = "";
    let damageNow = 100;
    const timeAt = (x: number): number => (x - X_START) / ROUND_SPEED;
    const duration = (): number =>
        Math.max(timeAt(kinEndX), timeAt(prcEndX)) + 0.4;

    const recompute = (): void => {
        const speed = Number(speedControl.input.value);
        damageNow = Number(damageControl.input.value);
        kin = kineticWalk(damageNow, speed, sections, hp);
        prc = pierceWalk(damageNow, speed, sections, hp);
        const scale = kineticDamageMultiplier(speed);
        // Kinetic: the round stops inside the first section it fails to
        // destroy, or dies spending its last point in a destroyed one.
        kinStopIdx = -1;
        if (kin.leftover <= 0) {
            for (let i = sections - 1; i >= 0; i--) {
                if (kin.results[i].dealt > 0) {
                    kinStopIdx = i;
                    break;
                }
            }
        }
        kinResolveT = [];
        kinRemAfter = [];
        let rem = damageNow;
        for (let i = 0; i < sections; i++) {
            const r = kin.results[i];
            if (r.state === "intact") {
                kinResolveT.push(undefined);
            } else if (i === kinStopIdx) {
                const stopX = secLeft(i) + w * (r.state === "hit" ? 0.4 : 0.7);
                kinResolveT.push(timeAt(stopX));
                rem = 0;
            } else {
                kinResolveT.push(timeAt(secLeft(i) + w * 0.55));
                rem = Math.max(0, rem - hp / scale);
            }
            kinRemAfter.push(rem);
        }
        kinEndX =
            kinStopIdx >= 0
                ? secLeft(kinStopIdx) +
                  w * (kin.results[kinStopIdx].state === "hit" ? 0.4 : 0.7)
                : XEND + 30;
        // Pierce: crossings resolve at each section's far edge.
        prcResolveT = [];
        for (let i = 0; i < sections; i++) {
            prcResolveT.push(
                i < prc.raked ? timeAt(secLeft(i) + w) : undefined
            );
        }
        prcEndX =
            prc.raked < sections ? secLeft(prc.raked - 1) + w + 10 : XEND + 30;
        prcReason =
            prc.raked < sections
                ? prc.raked === MAX_PIERCE_LAYERS &&
                  PIERCE_BASE_POWER - prc.cost * prc.raked > 0
                    ? "LAYER CAP"
                    : "POWER SPENT"
                : "";
        // Resolved-outcome stats (the scope replays them).
        kinScale.textContent = `x${scale.toFixed(2)}`;
        kinDead.textContent = String(
            kin.results.filter((r) => r.state === "dead").length
        );
        kinLeft.textContent =
            kin.leftover > 0 ? `${Math.round(kin.leftover)} dmg` : "nothing";
        prcCost.textContent = `${Math.round(prc.cost)} of ${PIERCE_BASE_POWER} power`;
        prcRaked.textContent = String(prc.raked);
        prcTotal.textContent = `${Math.round(prc.raked * damageNow)} dmg`;
        insetCursor.setAttribute("x1", String(ix(speed)));
        insetCursor.setAttribute("x2", String(ix(speed)));
        insetDotK.setAttribute("cx", String(ix(speed)));
        insetDotK.setAttribute(
            "cy",
            String(iy(kineticDamageMultiplier(speed)))
        );
        insetDotP.setAttribute("cx", String(ix(speed)));
        insetDotP.setAttribute("cy", String(iy(piercePowerMultiplier(speed))));
    };

    const setCell = (
        cell: LaneCell,
        word: string,
        detail: string,
        state: string,
        flash: boolean
    ): void => {
        cell.word.textContent = word;
        cell.detail.textContent = detail;
        cell.rect.setAttribute(
            "class",
            `widget-mark--section${state ? ` ${state}` : ""}${flash ? " is-flash" : ""}`
        );
        cell.word.setAttribute(
            "class",
            `widget-mark--word${state ? ` ${state}` : ""}`
        );
    };
    const placeRound = (
        lane: Lane,
        x: number,
        endX: number,
        t: number,
        stops: boolean,
        width: number
    ): void => {
        const tEnd = timeAt(endX);
        if (t >= tEnd && (stops || x > XEND + 20)) {
            lane.round.setAttribute("visibility", "hidden");
        } else {
            lane.round.setAttribute("visibility", "visible");
            lane.round.setAttribute("x", String(Math.min(x, endX) - width));
        }
        if (stops && t >= tEnd && t <= tEnd + 0.45) {
            const k = (t - tEnd) / 0.45;
            lane.impact.setAttribute("visibility", "visible");
            lane.impact.setAttribute("cx", String(endX));
            lane.impact.setAttribute("r", (3 + k * 9).toFixed(1));
            lane.impact.setAttribute("opacity", (1 - k).toFixed(2));
        } else {
            lane.impact.setAttribute("visibility", "hidden");
        }
    };
    const renderFrame = (t: number): void => {
        const x = X_START + t * ROUND_SPEED;
        // Kinetic lane.
        placeRound(laneK, x, kinEndX, t, kinStopIdx >= 0, 16);
        let kinRem = damageNow;
        for (let i = 0; i < sections; i++) {
            const rt = kinResolveT[i];
            const r = kin.results[i];
            if (rt !== undefined && t >= rt) {
                kinRem = kinRemAfter[i];
                setCell(
                    laneK.cells[i],
                    r.state === "dead" ? "DEAD" : "HIT",
                    `-${Math.round(r.dealt)} hp`,
                    r.state === "dead" ? "is-dead" : "is-hit",
                    t - rt < 0.18
                );
            } else if (r.state === "intact" && t >= duration() - 0.05) {
                setCell(laneK.cells[i], "CLEAR", `${hp} hp`, "", false);
            } else {
                setCell(laneK.cells[i], `S${i + 1}`, `${hp} hp`, "", false);
            }
        }
        laneK.barFill.setAttribute(
            "width",
            String((Math.max(0, kinRem) / damageNow) * BAR.w)
        );
        laneK.barText.textContent = `budget ${Math.round(kinRem)} dmg`;
        laneK.endText.textContent =
            kinStopIdx < 0 && t >= timeAt(kinEndX)
                ? `exits with ${Math.round(kin.leftover)} dmg`
                : "";
        // Pierce lane.
        placeRound(laneP, x, prcEndX, t, prc.raked < sections, 26);
        let crossed = 0;
        for (let i = 0; i < sections; i++) {
            const rt = prcResolveT[i];
            const r = prc.results[i];
            if (rt !== undefined && t >= rt) {
                crossed = i + 1;
                setCell(
                    laneP.cells[i],
                    r.state === "dead" ? "DEAD" : "HIT",
                    `-${Math.round(r.dealt)} hp`,
                    r.state === "dead" ? "is-dead" : "is-hit",
                    t - rt < 0.18
                );
            } else if (r.state === "intact" && t >= duration() - 0.05) {
                setCell(laneP.cells[i], "CLEAR", `${hp} hp`, "", false);
            } else {
                setCell(laneP.cells[i], `S${i + 1}`, `${hp} hp`, "", false);
            }
        }
        const power = PIERCE_BASE_POWER - prc.cost * crossed;
        laneP.barFill.setAttribute(
            "width",
            String((Math.max(0, power) / PIERCE_BASE_POWER) * BAR.w)
        );
        laneP.barText.textContent = `power ${Math.round(Math.max(0, power))}`;
        laneP.endText.textContent =
            t >= timeAt(prcEndX)
                ? prcReason ||
                  `exits with ${Math.round(Math.max(0, power))} power`
                : "";
    };

    const transport = makeTransport(duration, renderFrame);
    const onParam = (): void => {
        recompute();
        transport.seekEnd();
    };
    const speedControl = control(
        "Closing speed",
        10,
        400,
        10,
        REFERENCE_CLOSING_SPEED,
        (v) => engineMetersPerSec(v),
        onParam
    );
    const damageControl = control(
        "Authored damage",
        20,
        300,
        10,
        100,
        (v) => `${v} hp`,
        onParam
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(speedControl.row);
    controls.appendChild(damageControl.row);

    host.appendChild(controls);
    host.appendChild(transport.row);
    host.appendChild(plot);
    host.appendChild(kinStats);
    host.appendChild(prcStats);
    host.appendChild(insetPlot);
    host.appendChild(note);
    recompute();
    if (reducedMotion()) transport.seekEnd();
    else transport.play();
}

// ---- blast-layers ---------------------------------------------------------

// A PPI blast scope: detonation at the origin, range rings in meters, the
// structural layers as arcs on one bearing, the shock front replayed in
// scope time, and a pressure-vs-distance profile of the centre ray. Every
// distance in here is METERS, the system the warhead is authored in. The
// slider defaults are the shipped Serpent/Lance warhead.
function initBlastLayers(host: HTMLElement): void {
    const hp = numAttr(host, "hp", LIGHT_HULL_HP);
    const LAYER_DISTANCES = [100, 120, 140];
    const TARGET_DISTANCE = 160;
    // Presentation only: the game resolves a blast in one fixed tick; the
    // scope replays it at a legible sweep speed.
    const WAVE_SPEED = 120; // meters of front travel per scope second
    const RING_STEP = 100;
    header(
        host,
        "Blast scope: pressure through a hull",
        `Detonation at the scope origin; three light hull layers (${hp} hp, ` +
            `the catalog value) at 100, 120 and 140 m on the bearing; the ` +
            `section you care about at ${meters(TARGET_DISTANCE)}. Pressure falls ` +
            "off linearly to zero at the radius, every destroyed layer " +
            "passes 65% on, and a layer that survives stops the wave dead. " +
            "Defaults are the shipped torpedo warhead."
    );

    // Scope geometry: PPI on the left, ray profile inset on the right.
    const CX = 170;
    const CY = 172;
    const R_PX = 148;
    const BEARING = -Math.PI * 0.25;
    const HALF_ARC = (26 * Math.PI) / 180;
    const PX0 = 352;
    const PX1 = 548;
    const PY0 = 300;
    const PY1 = 60;
    const pt = (r: number, a: number): [number, number] => [
        CX + r * Math.cos(a),
        CY + r * Math.sin(a),
    ];
    const arcPath = (rPx: number, half: number): string => {
        const [x0, y0] = pt(rPx, BEARING - half);
        const [x1, y1] = pt(rPx, BEARING + half);
        return (
            `M${x0.toFixed(1)} ${y0.toFixed(1)} ` +
            `A${rPx.toFixed(1)} ${rPx.toFixed(1)} 0 0 1 ` +
            `${x1.toFixed(1)} ${y1.toFixed(1)}`
        );
    };

    const svg = svgEl("svg", {
        viewBox: "0 0 560 336",
        role: "img",
        "aria-label":
            "Blast scope: range rings around the detonation point, three " +
            "hull layers and the target section as arcs on one bearing, an " +
            "expanding shock ring, and a pressure-versus-distance profile " +
            "of the centre ray.",
    });
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);
    const stack = el("div", "widget__stack");
    const stats = el("div", "widget__stats");
    const frontStat = stat(stats, "front");
    const pressureStat = stat(stats, "pressure on the ray");
    const deadStat = stat(stats, "layers destroyed");
    const targetStat = stat(stats, "target section");
    const readout = el("p", "widget__readout");

    // Everything scale-dependent is rebuilt per parameter change (the unit-to-px
    // scale follows the scope range); the sweep then only mutates it.
    let damage = TORPEDO_BLAST_DAMAGE;
    let radius = TORPEDO_BLAST_RADIUS;
    let scopeR = TORPEDO_BLAST_RADIUS;
    let ppu = R_PX / TORPEDO_BLAST_RADIUS;
    let blast = blastWalk(damage, radius, LAYER_DISTANCES, hp, TARGET_DISTANCE);
    let holdDist = Infinity;
    let wave: SVGCircleElement;
    let det: SVGGElement;
    let shadow: SVGPathElement;
    let layerArcs: SVGPathElement[] = [];
    let targetArc: SVGPathElement;
    let profCursor: SVGLineElement;
    let profDot: SVGCircleElement;
    let cells: HTMLElement[] = [];
    const duration = (): number => radius / WAVE_SPEED;

    const rebuild = (): void => {
        blast = blastWalk(damage, radius, LAYER_DISTANCES, hp, TARGET_DISTANCE);
        const holdIdx = blast.layers.findIndex((l) => l.state === "holds");
        holdDist = holdIdx < 0 ? Infinity : LAYER_DISTANCES[holdIdx];
        scopeR =
            Math.ceil(Math.max(radius, TARGET_DISTANCE + 40) / RING_STEP) *
            RING_STEP;
        ppu = R_PX / scopeR;
        svg.replaceChildren();
        // Range rings, labeled in meters along the vertical.
        for (let r = RING_STEP; r <= scopeR; r += RING_STEP) {
            svg.appendChild(
                svgEl("circle", {
                    cx: String(CX),
                    cy: String(CY),
                    r: String(r * ppu),
                    class: "widget-mark--ring",
                })
            );
            svg.appendChild(
                svgEl(
                    "text",
                    {
                        x: String(CX + 4),
                        y: String(CY - r * ppu - 3),
                        class: "widget-mark--axis",
                    },
                    r === scopeR ? meters(r) : numText(r, 0)
                )
            );
        }
        // The authored blast radius: where free pressure reaches zero.
        svg.appendChild(
            svgEl("circle", {
                cx: String(CX),
                cy: String(CY),
                r: String(radius * ppu),
                class: "widget-mark--old",
            })
        );
        const [rlx, rly] = pt(radius * ppu + 4, Math.PI * 0.75);
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(rlx),
                    y: String(rly),
                    "text-anchor": "end",
                    class: "widget-mark--label-old",
                },
                `r ${meters(radius)}`
            )
        );
        // The bearing ray the layers sit on.
        const [rx, ry] = pt(scopeR * ppu, BEARING);
        svg.appendChild(
            svgEl("line", {
                x1: String(CX),
                y1: String(CY),
                x2: String(rx),
                y2: String(ry),
                class: "widget-mark--ray",
            })
        );
        // Blast shadow behind a holding layer (hidden until the front is
        // there): the wedge of the sector the wave never reaches.
        shadow = svgEl("path", {
            d: (() => {
                const r1 = (holdIdx < 0 ? scopeR : holdDist) * ppu;
                const r2 = scopeR * ppu;
                const [ax, ay] = pt(r1, BEARING - HALF_ARC);
                const [bx, by] = pt(r1, BEARING + HALF_ARC);
                const [cx2, cy2] = pt(r2, BEARING + HALF_ARC);
                const [dx, dy] = pt(r2, BEARING - HALF_ARC);
                return (
                    `M${ax.toFixed(1)} ${ay.toFixed(1)} ` +
                    `A${r1.toFixed(1)} ${r1.toFixed(1)} 0 0 1 ` +
                    `${bx.toFixed(1)} ${by.toFixed(1)} ` +
                    `L${cx2.toFixed(1)} ${cy2.toFixed(1)} ` +
                    `A${r2.toFixed(1)} ${r2.toFixed(1)} 0 0 0 ` +
                    `${dx.toFixed(1)} ${dy.toFixed(1)} Z`
                );
            })(),
            class: "widget-mark--shadow",
            visibility: "hidden",
        });
        svg.appendChild(shadow);
        // Layer arcs and the target arc on the bearing.
        layerArcs = LAYER_DISTANCES.map((d) => {
            const arc = svgEl("path", {
                d: arcPath(d * ppu, HALF_ARC),
                class: "widget-mark--layer",
            });
            svg.appendChild(arc);
            return arc;
        });
        targetArc = svgEl("path", {
            d: arcPath(TARGET_DISTANCE * ppu, HALF_ARC * 0.85),
            class: "widget-mark--target-arc",
        });
        svg.appendChild(targetArc);
        const [llx, lly] = pt(
            (LAYER_DISTANCES[2] + 40) * ppu,
            BEARING - HALF_ARC - 0.3
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(llx),
                    y: String(lly),
                    class: "widget-mark--axis",
                },
                "hull layers"
            )
        );
        const [tlx, tly] = pt(
            TARGET_DISTANCE * ppu + 8,
            BEARING + HALF_ARC + 0.16
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(tlx),
                    y: String(tly),
                    class: "widget-mark--axis",
                },
                "target"
            )
        );
        // Detonation point: a small red cross that flashes as the tape starts.
        det = svgEl("g", { class: "widget-mark--det" });
        det.appendChild(
            svgEl("line", {
                x1: String(CX - 5),
                y1: String(CY),
                x2: String(CX + 5),
                y2: String(CY),
            })
        );
        det.appendChild(
            svgEl("line", {
                x1: String(CX),
                y1: String(CY - 5),
                x2: String(CX),
                y2: String(CY + 5),
            })
        );
        svg.appendChild(det);
        // The shock front.
        wave = svgEl("circle", {
            cx: String(CX),
            cy: String(CY),
            r: "0",
            class: "widget-mark--wave",
        });
        svg.appendChild(wave);
        // Ray profile inset: free falloff (dashed) vs transmitted pressure
        // (solid) against distance, with the layer and target positions.
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(PX0),
                    y: String(PY1 - 12),
                    class: "widget-mark--axis",
                },
                "pressure on the ray"
            )
        );
        const xd = (d: number): number => PX0 + (d / scopeR) * (PX1 - PX0);
        const yp = (p: number): number =>
            PY0 - (Math.min(p, damage) / damage) * (PY0 - PY1);
        svg.appendChild(
            svgEl("line", {
                x1: String(PX0),
                y1: String(PY0),
                x2: String(PX1),
                y2: String(PY0),
                class: "widget-mark--grid",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(PX0 - 6),
                    y: String(PY1 + 4),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                String(damage)
            )
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(PX0 - 6),
                    y: String(PY0 + 4),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                "0"
            )
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(PX1),
                    y: String(PY0 + 14),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                meters(scopeR)
            )
        );
        svg.appendChild(
            svgEl("path", {
                d:
                    `M${xd(0).toFixed(1)} ${yp(damage).toFixed(1)} ` +
                    `L${xd(radius).toFixed(1)} ${yp(0).toFixed(1)}`,
                class: "widget-mark--old",
            })
        );
        let prof = `M${xd(0).toFixed(1)} ${yp(damage).toFixed(1)}`;
        let destroyed = 0;
        let stopped = false;
        const fall = (x: number): number =>
            x >= radius ? 0 : damage * (1 - x / radius);
        for (let i = 0; i < LAYER_DISTANCES.length && !stopped; i++) {
            const d = LAYER_DISTANCES[i];
            const before =
                fall(d) * Math.pow(EXPLOSIVE_SECTION_TRANSMISSION, destroyed);
            prof += ` L${xd(d).toFixed(1)} ${yp(before).toFixed(1)}`;
            if (before >= hp) {
                destroyed += 1;
                const after =
                    fall(d) *
                    Math.pow(EXPLOSIVE_SECTION_TRANSMISSION, destroyed);
                prof += ` L${xd(d).toFixed(1)} ${yp(after).toFixed(1)}`;
            } else {
                prof += ` L${xd(d).toFixed(1)} ${yp(0).toFixed(1)}`;
                stopped = true;
            }
        }
        if (!stopped) {
            prof += ` L${xd(radius).toFixed(1)} ${yp(0).toFixed(1)}`;
        }
        svg.appendChild(svgEl("path", { d: prof, class: "widget-mark--now" }));
        for (const d of LAYER_DISTANCES) {
            svg.appendChild(
                svgEl("line", {
                    x1: String(xd(d)),
                    y1: String(PY0),
                    x2: String(xd(d)),
                    y2: String(PY0 + 6),
                    class: "widget-mark--cursor",
                })
            );
        }
        svg.appendChild(
            svgEl("line", {
                x1: String(xd(TARGET_DISTANCE)),
                y1: String(PY0),
                x2: String(xd(TARGET_DISTANCE)),
                y2: String(PY0 + 6),
                class: "widget-mark--gate",
            })
        );
        profCursor = svgEl("line", {
            y1: String(PY1),
            y2: String(PY0),
            class: "widget-mark--cursor",
        });
        profDot = svgEl("circle", { r: "4", class: "widget-mark--dot-now" });
        svg.appendChild(profCursor);
        svg.appendChild(profDot);
        // The per-layer cells restart blank; the sweep fills them.
        stack.replaceChildren();
        cells = LAYER_DISTANCES.map((d) => {
            const cell = sectionCell("STANDBY", `@ ${meters(d)}: ${hp} hp`, "");
            stack.appendChild(cell);
            return cell;
        });
    };

    const setCellState = (
        i: number,
        word: string,
        detail: string,
        state: string
    ): void => {
        if (cells[i].querySelector("b")?.textContent === word) return;
        const cell = sectionCell(word, detail, state);
        cells[i].replaceWith(cell);
        cells[i] = cell;
    };
    const renderFrame = (t: number): void => {
        const front = Math.min(t * WAVE_SPEED, radius);
        const frontInfo = blastFront(
            damage,
            radius,
            LAYER_DISTANCES,
            hp,
            front
        );
        wave.setAttribute("r", String(front * ppu));
        wave.setAttribute("opacity", front >= radius ? "0.35" : "1");
        det.setAttribute("opacity", front < 25 ? "1" : "0.45");
        blast.layers.forEach((layer, i) => {
            const d = LAYER_DISTANCES[i];
            const crossed = front >= d;
            const flash = crossed && front < d + 22;
            let cls = "widget-mark--layer";
            if (crossed) {
                if (layer.state === "dead") cls += " is-dead";
                else if (layer.state === "holds") cls += " is-hold";
                else cls += " is-shielded";
                if (flash) cls += " is-flash";
            }
            layerArcs[i].setAttribute("class", cls);
            const at = `@ ${meters(d)}`;
            if (!crossed) {
                setCellState(i, "STANDBY", `${at}: ${hp} hp`, "");
            } else if (layer.state === "dead") {
                setCellState(
                    i,
                    "DEAD",
                    `${at}: in ${Math.round(layer.incoming)}, passes 65%`,
                    "is-dead"
                );
            } else if (layer.state === "holds") {
                setCellState(
                    i,
                    "HOLDS",
                    `${at}: in ${Math.round(layer.incoming)} vs ${hp} hp`,
                    "is-hit"
                );
            } else {
                setCellState(i, "SHIELDED", `${at}: 0`, "is-clear");
            }
        });
        shadow.setAttribute(
            "visibility",
            front >= holdDist ? "visible" : "hidden"
        );
        const targetReached = front >= TARGET_DISTANCE;
        let tCls = "widget-mark--target-arc";
        if (targetReached) {
            tCls += blast.target > 0 ? " is-hit" : " is-shielded";
            if (front < TARGET_DISTANCE + 22) tCls += " is-flash";
        }
        targetArc.setAttribute("class", tCls);
        // Ray-profile cursor.
        const xd = (d: number): number => PX0 + (d / scopeR) * (PX1 - PX0);
        const yp = (p: number): number =>
            PY0 - (Math.min(p, damage) / damage) * (PY0 - PY1);
        profCursor.setAttribute("x1", String(xd(front)));
        profCursor.setAttribute("x2", String(xd(front)));
        profDot.setAttribute("cx", String(xd(front)));
        profDot.setAttribute("cy", String(yp(frontInfo.pressure)));
        // Readouts tick with the front.
        frontStat.textContent = meters(front);
        pressureStat.textContent = frontInfo.stopped
            ? "0 (stopped)"
            : `${Math.round(frontInfo.pressure)} hp`;
        deadStat.textContent = String(frontInfo.destroyed);
        const destroyed = blast.layers.filter((l) => l.state === "dead").length;
        targetStat.textContent = targetReached
            ? blast.target > 0
                ? `${Math.round(blast.target)} hp`
                : "0 (shielded)"
            : "--";
        readout.classList.remove("is-warn");
        if (t >= duration()) {
            if (blast.target > 0) {
                readout.textContent =
                    `Target section at ${meters(TARGET_DISTANCE)} takes ` +
                    `${Math.round(blast.target)} hp, through ${destroyed} ` +
                    `destroyed layer${destroyed === 1 ? "" : "s"}.`;
            } else {
                readout.textContent =
                    `Target section at ${meters(TARGET_DISTANCE)} takes 0 - a ` +
                    "surviving layer stopped the wave.";
                readout.classList.add("is-warn");
            }
        } else if (frontInfo.stopped) {
            readout.textContent =
                `Wave stopped at the layer holding at ${meters(holdDist)} - ` +
                "everything behind it on the bearing is shielded.";
            readout.classList.add("is-warn");
        } else {
            readout.textContent =
                `Shock front at ${meters(front)} - carrying ` +
                `${Math.round(frontInfo.pressure)} hp along the bearing.`;
        }
    };

    const transport = makeTransport(duration, renderFrame);
    const onParam = (): void => {
        damage = Number(damageControl.input.value);
        radius = Number(radiusControl.input.value);
        rebuild();
        transport.seekEnd();
    };
    const damageControl = control(
        "Blast damage",
        100,
        900,
        25,
        TORPEDO_BLAST_DAMAGE,
        (v) => `${v} hp`,
        onParam
    );
    const radiusControl = control(
        "Blast radius",
        160,
        600,
        20,
        TORPEDO_BLAST_RADIUS,
        (v) => meters(v),
        onParam
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(damageControl.row);
    controls.appendChild(radiusControl.row);

    host.appendChild(controls);
    host.appendChild(transport.row);
    host.appendChild(plot);
    host.appendChild(stack);
    host.appendChild(stats);
    host.appendChild(readout);
    rebuild();
    if (reducedMotion()) transport.seekEnd();
    else transport.play();
}

// ---- shipped hulls --------------------------------------------------------

type Vec3T = [number, number, number];

interface ShipPart {
    id: string;
    label: string;
    health: number;
    center: Vec3T;
    size: Vec3T;
}

// One authored craft part, in the terms the ship files write it in: an origin
// plus the bounding box the art was cut to. The section sits at the middle of
// that box and its collider IS the box, so nothing here is re-derived
// (crates/nova_authoring/src/base_content/ships/shared.rs:44-50,:235).
function shipPart(
    id: string,
    label: string,
    health: number,
    origin: Vec3T,
    boxMin: Vec3T,
    boxMax: Vec3T
): ShipPart {
    return {
        id,
        label,
        health,
        center: [
            origin[0] + (boxMin[0] + boxMax[0]) * 0.5,
            origin[1] + (boxMin[1] + boxMax[1]) * 0.5,
            origin[2] + (boxMin[2] + boxMax[2]) * 0.5,
        ],
        size: [
            boxMax[0] - boxMin[0],
            boxMax[1] - boxMin[1],
            boxMax[2] - boxMin[2],
        ],
    };
}

// A turret MOUNT POINT carries no art and no box of its own: the shared PDC
// fills it, so the section that lands there is the PDC's own cube
// (sections/standard.rs:71,:228,:240-242).
const PDC_TURRET_SIZE = 0.5; // standard.rs:91
const TURRET_BASE_HEALTH = 130; // standard.rs:32
function turretMount(id: string, label: string, center: Vec3T): ShipPart {
    return {
        id,
        label,
        health: TURRET_BASE_HEALTH,
        center,
        size: [PDC_TURRET_SIZE, PDC_TURRET_SIZE, PDC_TURRET_SIZE],
    };
}

// The shipped corvette (ships/cargo_a.rs:16-96): two drives on two pods, a
// nose carrying both guns on its cheeks, a tail, and the fuselage that IS the
// flight computer.
const CARGOA_PARTS: ShipPart[] = [
    shipPart(
        "engine_starboard",
        "DRV S",
        70,
        [1.0, 0.5, 2.0],
        [-0.19, -0.2975, -0.5],
        [0.6, 0.4975, 0.45]
    ),
    shipPart(
        "engine_port",
        "DRV P",
        70,
        [-1.0, 0.5, 2.0],
        [-0.6, -0.2975, -0.5],
        [0.19, 0.4975, 0.45]
    ),
    shipPart(
        "pod_starboard",
        "POD S",
        350,
        [1.0, 0.5, 0.5],
        [-0.19, -0.3, -1.05],
        [0.6, 0.7, 1.0]
    ),
    shipPart(
        "pod_port",
        "POD P",
        350,
        [-1.0, 0.5, 0.5],
        [-0.6, -0.3, -1.05],
        [0.19, 0.7, 1.0]
    ),
    shipPart(
        "nose",
        "NOSE",
        180,
        [0.0, 1.0, -2.0],
        [-0.8, -0.8, -0.45],
        [0.8, 0.4, 0.85]
    ),
    shipPart(
        "tail",
        "TAIL",
        150,
        [0.0, 0.5, 2.0],
        [-0.81, -0.5, -0.5],
        [0.81, 0.675, 0.45]
    ),
    shipPart(
        "fuselage",
        "FUSELAGE",
        350,
        [0.0, 1.0, 0.0],
        [-0.81, -1.0, -1.15],
        [0.81, 0.6, 1.5]
    ),
    turretMount("turret_starboard", "T", [0.95, 0.8, -1.8]),
    turretMount("turret_port", "T", [-0.95, 0.8, -1.8]),
];

// The authored structural mates (cargo_a.rs:98-108). Both guns hang off the
// NOSE, and each drive hangs off its own pod - which is what decides who goes
// adrift when a part in the middle dies.
const CARGOA_MATES: [string, string][] = [
    ["fuselage", "nose"],
    ["fuselage", "tail"],
    ["fuselage", "pod_starboard"],
    ["fuselage", "pod_port"],
    ["pod_starboard", "engine_starboard"],
    ["pod_port", "engine_port"],
    ["nose", "turret_starboard"],
    ["nose", "turret_port"],
];

// The civilian yacht, which flies UNARMED: the base assembly takes the meshed
// seven and leaves its two mount points empty (ships/racer.rs:13-88,:107-115).
const RACER_PARTS: ShipPart[] = [
    shipPart(
        "engine_starboard",
        "DRV S",
        70,
        [0.5, 0.5, 1.5],
        [-0.09, -0.3, -0.3],
        [0.4, 0.44189, 0.32567]
    ),
    shipPart(
        "engine_port",
        "DRV P",
        70,
        [-0.5, 0.5, 1.5],
        [-0.4, -0.3, -0.3],
        [0.09, 0.44189, 0.32567]
    ),
    shipPart(
        "wing_starboard",
        "WING S",
        180,
        [1.0, 0.5, 0.0],
        [-0.59, -0.5, -0.964329],
        [0.2, 0.5, 1.2]
    ),
    shipPart(
        "wing_port",
        "WING P",
        180,
        [-1.0, 0.5, 0.0],
        [-0.2, -0.5, -0.964329],
        [0.59, 0.5, 1.2]
    ),
    shipPart(
        "nose",
        "NOSE",
        120,
        [0.0, 0.5, -1.5],
        [-0.4, -0.5, -0.52567],
        [0.4, 0.72265, 0.5]
    ),
    shipPart(
        "tail",
        "TAIL",
        120,
        [0.0, 1.0, 1.5],
        [-0.41, -0.8, -0.3],
        [0.41, 0.5, 0.52567]
    ),
    shipPart(
        "fuselage",
        "FUSELAGE",
        240,
        [0.0, 0.5, 0.0],
        [-0.41, -0.5, -1.0],
        [0.41, 0.9, 1.2]
    ),
];

// The torpedo hauler (ships/cargo_b.rs:9-82). Its two big side pods are the
// tubes, and its guns stand on their shoulders rather than on the nose.
const CARGOB_PARTS: ShipPart[] = [
    shipPart(
        "engine_starboard",
        "DRV S",
        70,
        [1.0, 0.5, 2.0],
        [-0.39, -0.3, -0.5],
        [0.4, 0.7, 0.5]
    ),
    shipPart(
        "engine_port",
        "DRV P",
        70,
        [-1.0, 0.5, 2.0],
        [-0.4, -0.3, -0.5],
        [0.39, 0.7, 0.5]
    ),
    shipPart(
        "pod_starboard",
        "POD S",
        350,
        [1.0, 0.5, -0.5],
        [-0.39, -0.3, -2.0],
        [0.5, 0.7, 2.0]
    ),
    shipPart(
        "pod_port",
        "POD P",
        350,
        [-1.0, 0.5, -0.5],
        [-0.5, -0.3, -2.0],
        [0.39, 0.7, 2.0]
    ),
    shipPart(
        "nose",
        "NOSE",
        180,
        [0.0, 1.0, -2.0],
        [-0.61, -0.8, -0.5],
        [0.61, 0.8, 1.0]
    ),
    shipPart(
        "tail",
        "TAIL",
        150,
        [0.0, 0.5, 2.0],
        [-0.61, -0.5, -0.5],
        [0.61, 0.8, 0.5]
    ),
    shipPart(
        "fuselage",
        "FUSELAGE",
        300,
        [0.0, 1.0, 0.5],
        [-0.61, -1.0, -1.5],
        [0.61, 0.8, 1.0]
    ),
    turretMount("turret_starboard", "T", [1.55, 1.2, 0.0]),
    turretMount("turret_port", "T", [-1.55, 1.2, 0.0]),
];

// The largest eigenvalue of a symmetric 3x3, closed form. This is the
// "conservative axis" the attitude budget is taken against
// (attitude.rs:69-71): one scalar has to cover a hull that rolls far more
// easily than it yaws, so it is the WORST of the three.
function largestPrincipal(m: number[][]): number {
    const offDiagonal = m[0][1] ** 2 + m[0][2] ** 2 + m[1][2] ** 2;
    if (offDiagonal < 1e-12) return Math.max(m[0][0], m[1][1], m[2][2]);
    const q = (m[0][0] + m[1][1] + m[2][2]) / 3;
    const p2 =
        (m[0][0] - q) ** 2 +
        (m[1][1] - q) ** 2 +
        (m[2][2] - q) ** 2 +
        2 * offDiagonal;
    const p = Math.sqrt(p2 / 6);
    const b = m.map((row, i) => row.map((v, j) => (v - (i === j ? q : 0)) / p));
    const determinant =
        b[0][0] * (b[1][1] * b[2][2] - b[1][2] * b[2][1]) -
        b[0][1] * (b[1][0] * b[2][2] - b[1][2] * b[2][0]) +
        b[0][2] * (b[1][0] * b[2][1] - b[1][1] * b[2][0]);
    const phi = Math.acos(clamp(determinant / 2, -1, 1)) / 3;
    return q + 2 * p * Math.cos(phi);
}

interface HullState {
    mass: number;
    centerOfMass: Vec3T;
    inertia: number;
    arm: number;
    armSetBy: string;
}

// What avian would measure for a hull assembled from these sections, plus the
// structural arm derived off it (attitude.rs:146-186). Density is 1 and not
// authorable, so a section's mass is exactly its box volume
// (base_section.rs:376) - which is why NOTHING in this function reads an
// authored number. Every shipped section is mounted axis-aligned except the
// turret mounts, whose box is a cube and so is the same under any rotation;
// that is what lets the arm drop the rotation term the Rust carries.
export function hullState(parts: ShipPart[]): HullState {
    let mass = 0;
    const com: Vec3T = [0, 0, 0];
    for (const part of parts) {
        const m = part.size[0] * part.size[1] * part.size[2];
        mass += m;
        for (let i = 0; i < 3; i++) com[i] += m * part.center[i];
    }
    if (mass <= 0) {
        return {
            mass: 0,
            centerOfMass: [0, 0, 0],
            inertia: 0,
            arm: 0,
            armSetBy: "",
        };
    }
    for (let i = 0; i < 3; i++) com[i] /= mass;

    const tensor = [
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
    ];
    for (const part of parts) {
        const [a, b, c] = part.size;
        const m = a * b * c;
        const own = [
            (m * (b * b + c * c)) / 12,
            (m * (a * a + c * c)) / 12,
            (m * (a * a + b * b)) / 12,
        ];
        const d = part.center.map((v, i) => v - com[i]);
        const r2 = d[0] ** 2 + d[1] ** 2 + d[2] ** 2;
        for (let i = 0; i < 3; i++) {
            for (let j = 0; j < 3; j++) {
                tensor[i][j] +=
                    i === j
                        ? own[i] + m * (r2 - d[i] * d[i])
                        : -m * d[i] * d[j];
            }
        }
    }

    let arm = 0;
    let armSetBy = "";
    for (const part of parts) {
        const d = part.center.map((v, i) => v - com[i]);
        const distance = Math.hypot(d[0], d[1], d[2]);
        const half = part.size.map((s) => s * 0.5);
        // A section sitting ON the balance point has no radial ray of its own:
        // its own furthest face is the whole arm it offers (attitude.rs:155-159).
        const reach =
            distance <= 1e-6
                ? Math.max(half[0], half[1], half[2])
                : distance +
                  half.reduce(
                      (sum, h, i) => sum + (h * Math.abs(d[i])) / distance,
                      0
                  );
        if (reach > arm) {
            arm = reach;
            armSetBy = part.id;
        }
    }
    return {
        mass,
        centerOfMass: com,
        inertia: largestPrincipal(tensor),
        arm,
        armSetBy,
    };
}

// Which sections are still THE SHIP after `destroyed` have been shot off.
//
// A cut that disconnects the structural graph severs it: the body carrying the
// live computers keeps ship identity and every other piece drifts away as an
// inert wreck (nova_ship/src/sections/integrity.rs:231-349). The fuselage is
// the only computer on all three shipped craft, so the retained body is the
// one it sits in.
export function severedParts(
    parts: ShipPart[],
    mates: [string, string][],
    destroyed: Set<string>
): { held: ShipPart[]; adrift: ShipPart[] } {
    const live = parts.filter((part) => !destroyed.has(part.id));
    if (destroyed.has("fuselage")) return { held: [], adrift: live };
    const reached = new Set<string>(["fuselage"]);
    for (let pass = 0; pass < live.length; pass++) {
        for (const [a, b] of mates) {
            if (destroyed.has(a) || destroyed.has(b)) continue;
            if (reached.has(a)) reached.add(b);
            if (reached.has(b)) reached.add(a);
        }
    }
    return {
        held: live.filter((part) => reached.has(part.id)),
        adrift: live.filter((part) => !reached.has(part.id)),
    };
}

// ---- controller-arm -------------------------------------------------------

// The corvette in plan view, with the balance point, the structural arm as a
// ring, and the ceiling that arm buys read off the 8 G curve beside it. Shoot
// pieces off and both move - which is the whole model: the ceiling is not
// authored anywhere, it falls out of where the metal ended up.
function initControllerArm(host: HTMLElement): void {
    header(
        host,
        "The arm: what the metal allows",
        "Hull metal takes 8 G at any point on it, so the turn ceiling is " +
            "that limit over the arm from the ship's balance point to its " +
            "furthest face. Shoot pieces off the corvette and watch the " +
            "balance point move, the arm shorten and the ceiling climb."
    );

    // Plan view, drawn in the INTACT ship's frame so the hull stays put and
    // the balance point is the thing seen to move. A ship faces -Z with
    // starboard at +X, so a view from ABOVE with the nose at screen left puts
    // starboard at the TOP - `py` runs against +X, or the caption is lying and
    // every port/starboard label is on the wrong side.
    const SCALE = 42;
    const AX = 154;
    const AY = 144;
    const px = (z: number): number => AX + z * SCALE;
    const py = (x: number): number => AY - x * SCALE;

    // The 8 G curve beside it. Linear on both axes: the two ceilings are a
    // factor of fifteen apart on this hull, so drawing them as two lines would
    // need a log scale that flattens the only curve worth seeing.
    const BX0 = 336;
    const BX1 = 548;
    const BY0 = 210;
    const BY1 = 40;
    const ARM_MIN = 1.0;
    const ARM_MAX = 3.0;
    const CEIL_MAX = 8;
    const bx = (a: number): number =>
        BX0 +
        ((clamp(a, ARM_MIN, ARM_MAX) - ARM_MIN) / (ARM_MAX - ARM_MIN)) *
            (BX1 - BX0);
    const by = (c: number): number =>
        BY0 - (clamp(c, 0, CEIL_MAX) / CEIL_MAX) * (BY0 - BY1);

    const svg = svgEl("svg", {
        viewBox: "0 0 560 280",
        role: "img",
        "aria-label":
            "Left: the corvette from above, with its balance point, a ring " +
            "at its structural arm, and any sections shot off or set adrift " +
            "marked. Right: turn ceiling against structural arm, with the " +
            "intact ship and the current wreck marked on the curve.",
    });

    svg.appendChild(
        svgEl(
            "text",
            { x: "8", y: "16", class: "widget-mark--axis" },
            "corvette, from above - nose to the left"
        )
    );
    svg.appendChild(
        svgEl(
            "text",
            { x: String(BX0), y: "16", class: "widget-mark--axis" },
            "ceiling (rad/s^2) against arm (m)"
        )
    );

    // --- panel B furniture, drawn once ---
    for (const c of [2, 4, 6, 8]) {
        svg.appendChild(
            svgEl("line", {
                x1: String(BX0),
                y1: String(by(c)),
                x2: String(BX1),
                y2: String(by(c)),
                class: "widget-mark--grid",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(BX0 - 5),
                    y: String(by(c) + 3),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                String(c)
            )
        );
    }
    for (const a of [1, 1.5, 2, 2.5, 3]) {
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(bx(a)),
                    y: String(BY0 + 14),
                    "text-anchor": "middle",
                    class: "widget-mark--axis",
                },
                a.toFixed(1)
            )
        );
    }
    const curve: string[] = [];
    for (let a = ARM_MIN; a <= ARM_MAX + 1e-9; a += 0.05) {
        curve.push(
            `${bx(a).toFixed(1)},${by(structuralCeiling(a)).toFixed(1)}`
        );
    }
    // `fill` has to be said out loud: the gate mark was written for a straight
    // LINE, which has no fill, and an unfilled path would otherwise flood the
    // area under the curve with the default black.
    svg.appendChild(
        svgEl("path", {
            d: `M${curve.join(" L")}`,
            fill: "none",
            class: "widget-mark--gate",
        })
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(bx(ARM_MIN) + 6),
                y: String(by(structuralCeiling(ARM_MIN)) - 6),
                class: "widget-mark--label-gate",
            },
            "8 G / arm"
        )
    );

    const intact = hullState(CARGOA_PARTS);
    // Only drawn once there is damage to compare against: on the intact hull
    // this dot sits exactly under the live one.
    const intactDot = svgEl("circle", {
        cx: String(bx(intact.arm)),
        cy: String(by(structuralCeiling(intact.arm))),
        r: "4",
        class: "widget-mark--dot-old",
    });
    const intactLabel = svgEl(
        "text",
        {
            x: String(bx(intact.arm)),
            y: String(by(structuralCeiling(intact.arm)) + 16),
            "text-anchor": "middle",
            class: "widget-mark--label-old",
        },
        "intact"
    );
    svg.appendChild(intactDot);
    svg.appendChild(intactLabel);
    const nowDot = svgEl("circle", { r: "6", class: "widget-mark--dot-now" });

    // --- panel A furniture ---
    const ring = svgEl("circle", { r: "0", class: "widget-mark--ring" });
    const ray = svgEl("line", { class: "widget-mark--ray" });
    const comDot = svgEl("circle", { r: "3.5", class: "widget-mark--dot-now" });
    // The live reading sits in the empty quarter under the curve rather than
    // on the ray, which runs straight through the hull it is measuring.
    const armLabel = svgEl(
        "text",
        { x: String(BX0 - 16), y: "248", class: "widget-mark--label-now" },
        ""
    );
    const adriftWord = svgEl(
        "text",
        { x: "8", y: "272", class: "widget-mark--word is-dead" },
        ""
    );
    const hullGroup = svgEl("g", {});
    svg.appendChild(ring);
    svg.appendChild(hullGroup);
    svg.appendChild(ray);
    svg.appendChild(comDot);
    svg.appendChild(armLabel);
    svg.appendChild(adriftWord);
    svg.appendChild(nowDot);

    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stats");
    const armStat = stat(stats, "structural arm");
    const ceilingStat = stat(stats, "turn ceiling");
    const bindsStat = stat(stats, "what binds");
    const torqueStat = stat(stats, "the computer offers");
    const flipStat = stat(stats, "180 flip");
    const readout = el("p", "widget__readout");

    const destroyed = new Set<string>();
    const keys = el("div", "widget__keys");
    const keyButtons = new Map<string, HTMLButtonElement>();

    const update = (): void => {
        const { held, adrift } = severedParts(
            CARGOA_PARTS,
            CARGOA_MATES,
            destroyed
        );
        const state = hullState(held);
        const structural = structuralCeiling(state.arm);
        // ONE computer: the fuselage is the corvette's only Controller part
        // (cargo_a.rs:77-86), and it is the only one the shipped craft carry.
        // Torque sums with no curve and no cap (controller_section.rs:385-388),
        // so with the fuselage gone there is no propulsive ceiling at all -
        // and no ship, because the fuselage is also what holds it together.
        const torque = torqueCeiling(CONTROLLER_MAX_TORQUE, state.inertia);
        const ceiling = Math.min(structural, torque);

        hullGroup.replaceChildren();
        for (const part of CARGOA_PARTS) {
            if (destroyed.has(part.id)) continue;
            const gone = adrift.includes(part);
            hullGroup.appendChild(
                svgEl("rect", {
                    x: String(px(part.center[2] - part.size[2] * 0.5)),
                    y: String(py(part.center[0] + part.size[0] * 0.5)),
                    width: String(part.size[2] * SCALE),
                    height: String(part.size[0] * SCALE),
                    rx: "2",
                    class: `widget-mark--section${gone ? " is-dead" : ""}`,
                })
            );
            hullGroup.appendChild(
                svgEl(
                    "text",
                    {
                        x: String(px(part.center[2])),
                        // Along the TOP edge of the box rather than through
                        // its middle: the balance point sits inside the
                        // fuselage, and a centred label runs straight under it.
                        y: String(py(part.center[0] + part.size[0] * 0.5) + 11),
                        "text-anchor": "middle",
                        class: gone
                            ? "widget-mark--word is-dead"
                            : "widget-mark--detail",
                    },
                    part.label
                )
            );
        }

        // Named while the list is short enough to read, counted after that:
        // the plan view already marks them, and one line of glass cannot
        // carry eight part names.
        const adriftNames = adrift.map((part) => part.id.replace(/_/g, " "));
        adriftWord.textContent = !adrift.length
            ? ""
            : adrift.length <= 3
              ? `severed: ${adriftNames.join(", ")} - no longer this ship`
              : `${adrift.length} sections severed - no longer this ship`;

        if (!held.length) {
            ring.setAttribute("r", "0");
            ray.setAttribute("x1", "0");
            ray.setAttribute("y1", "0");
            ray.setAttribute("x2", "0");
            ray.setAttribute("y2", "0");
            comDot.setAttribute("cx", "-20");
            comDot.setAttribute("cy", "-20");
            armLabel.textContent = "";
            nowDot.setAttribute("cx", "-20");
            nowDot.setAttribute("cy", "-20");
            intactDot.setAttribute("opacity", "1");
            intactLabel.setAttribute("opacity", "1");
            armStat.textContent = "-";
            ceilingStat.textContent = "-";
            bindsStat.textContent = "nothing steers";
            torqueStat.textContent = "0 rad/s^2";
            flipStat.textContent = "-";
            readout.classList.add("is-fault");
            readout.textContent = destroyed.has("fuselage")
                ? "The fuselage carried the only flight computer, so nothing " +
                  "is steering: what is left of the corvette is a drifting, " +
                  "tumbling derelict, and every piece of it has severed away."
                : "Nothing left of the ship at all.";
            return;
        }
        readout.classList.remove("is-fault");

        const cx = px(state.centerOfMass[2]);
        const cy = py(state.centerOfMass[0]);
        comDot.setAttribute("cx", String(cx));
        comDot.setAttribute("cy", String(cy));
        ring.setAttribute("cx", String(cx));
        ring.setAttribute("cy", String(cy));
        ring.setAttribute("r", String(state.arm * SCALE));

        const setter =
            held.find((part) => part.id === state.armSetBy) ?? held[0];
        const dz = setter.center[2] - state.centerOfMass[2];
        const dx = setter.center[0] - state.centerOfMass[0];
        const length = Math.hypot(dz, dx) || 1;
        const tipX = cx + (dz / length) * state.arm * SCALE;
        const tipY = cy - (dx / length) * state.arm * SCALE;
        ray.setAttribute("x1", String(cx));
        ray.setAttribute("y1", String(cy));
        ray.setAttribute("x2", String(tipX));
        ray.setAttribute("y2", String(tipY));
        armLabel.textContent = `arm ${engineMeters(state.arm, 1)}, ceiling ${structural.toFixed(2)} rad/s^2`;

        nowDot.setAttribute("cx", String(bx(state.arm)));
        nowDot.setAttribute("cy", String(by(structural)));
        const damaged = destroyed.size > 0;
        intactDot.setAttribute("opacity", damaged ? "1" : "0");
        intactLabel.setAttribute("opacity", damaged ? "1" : "0");

        armStat.textContent = `${engineMeters(state.arm, 1)}`;
        ceilingStat.textContent = `${ceiling.toFixed(2)} rad/s^2`;
        bindsStat.textContent =
            torque < structural ? "torque-limited" : "structure-limited";
        torqueStat.textContent = `${torque.toFixed(1)} rad/s^2`;
        flipStat.textContent = `${flipSeconds(ceiling).toFixed(2)} s`;

        const gain = (structural / structuralCeiling(intact.arm) - 1) * 100;
        if (!destroyed.size) {
            readout.textContent =
                `Nine sections, ${engineMeters(state.arm, 1)} of arm. The metal ` +
                `gives up at ${structural.toFixed(2)} rad/s^2, and the one ` +
                `flight computer in its fuselage could push ` +
                `${torque.toFixed(1)} - ${(torque / structural).toFixed(0)} ` +
                "times as hard. Every shipped craft sits this far clear of " +
                "its computers, which is why fitting more of them buys no " +
                "turn rate at all.";
        } else if (gain <= -0.5) {
            readout.textContent =
                `${destroyed.size} section${destroyed.size === 1 ? "" : "s"} ` +
                `gone${adrift.length ? ` and ${adrift.length} adrift` : ""}, ` +
                `and the arm has grown to ${engineMeters(state.arm, 1)} - so the ` +
                `wreck turns ${Math.abs(gain).toFixed(0)}% SOFTER than the ` +
                "whole ship did. Losing weight off one end drags the balance " +
                "point toward the other, and the reach to whatever is left " +
                "out there gets longer.";
        } else if (gain >= 0.5) {
            readout.textContent =
                `${destroyed.size} section${destroyed.size === 1 ? "" : "s"} ` +
                `gone${adrift.length ? ` and ${adrift.length} adrift` : ""}. ` +
                `The arm is down to ${engineMeters(state.arm, 1)} and the wreck ` +
                `turns ${gain.toFixed(0)}% harder than the whole ship did - ` +
                `a 180 in ${flipSeconds(ceiling).toFixed(2)} s against ` +
                `${flipSeconds(structuralCeiling(intact.arm)).toFixed(2)} s.`;
        } else {
            readout.textContent =
                `${destroyed.size} section${destroyed.size === 1 ? "" : "s"} ` +
                `gone${adrift.length ? ` and ${adrift.length} adrift` : ""}, ` +
                `and the arm is still ${engineMeters(state.arm, 1)}. Damage only ` +
                "buys a turn when it takes weight off ONE end: cut the same " +
                "amount off both and the balance point stays where it was.";
        }
    };

    for (const part of CARGOA_PARTS) {
        const name =
            part.id === "turret_starboard"
                ? "TURRET S"
                : part.id === "turret_port"
                  ? "TURRET P"
                  : part.label;
        const btn = el("button", "widget__btn", name);
        btn.type = "button";
        btn.setAttribute("aria-pressed", "false");
        btn.addEventListener("click", () => {
            if (destroyed.has(part.id)) destroyed.delete(part.id);
            else destroyed.add(part.id);
            const out = destroyed.has(part.id);
            btn.textContent = out ? `${name} OUT` : name;
            btn.classList.toggle("is-hot", out);
            btn.setAttribute("aria-pressed", String(out));
            update();
        });
        keyButtons.set(part.id, btn);
        keys.appendChild(btn);
    }
    const rebuild = el("button", "widget__btn", "REBUILD");
    rebuild.type = "button";
    rebuild.addEventListener("click", () => {
        destroyed.clear();
        for (const [id, btn] of keyButtons) {
            const part = CARGOA_PARTS.find((p) => p.id === id);
            btn.textContent =
                id === "turret_starboard"
                    ? "TURRET S"
                    : id === "turret_port"
                      ? "TURRET P"
                      : (part?.label ?? id);
            btn.classList.remove("is-hot");
            btn.setAttribute("aria-pressed", "false");
        }
        update();
    });
    keys.appendChild(rebuild);

    const note = el(
        "p",
        "widget__note",
        "The lit dot is the ship's balance point and the ring is its arm: " +
            "the reach to the outer FACE of the furthest section, not to its " +
            "centre. Mass is not authored anywhere - a section weighs its " +
            "own authored box - so both the balance point and the arm are " +
            "read off the corvette's parts and nothing else. Both guns hang " +
            "off the nose and each drive off its own pod, so killing a part " +
            "in the middle cuts the pieces beyond it loose."
    );

    host.appendChild(keys);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    update();
}

// ---- controller-margin ----------------------------------------------------

// The 8 G budget as ONE acceleration at the hull's furthest point, split
// between the sideways load a turn already carries and what is left to turn
// harder with. The two add as a vector (attitude.rs:112-130), so authority
// does not fall off gently - it holds, then collapses.
function initControllerMargin(host: HTMLElement): void {
    header(
        host,
        "A hard turn spends the margin",
        "The 8 G limit is one acceleration at the ship's furthest point, " +
            "and a turn already spends part of it just holding the curve. " +
            "Roll the corvette's turn rate up and watch what is left to " +
            "turn HARDER with."
    );

    const arm = hullState(CARGOA_PARTS).arm;
    const structural = structuralCeiling(arm);
    const sustained = sustainedTurnRate(arm);
    const sustainedDeg = (sustained * 180) / Math.PI;
    const MAX_DEG = sustainedDeg * 1.18;

    const X0 = 46;
    const X1 = 548;
    const Y0 = 168;
    const Y1 = 16;
    const x = (deg: number): number =>
        X0 + (clamp(deg, 0, MAX_DEG) / MAX_DEG) * (X1 - X0);
    const y = (a: number): number =>
        Y0 - (clamp(a, 0, structural) / structural) * (Y0 - Y1);
    // attitude.rs:121-130, in the widget's units: the two loads are
    // perpendicular components of one acceleration, so the tangential half is
    // what is left of the budget after the centripetal half. Past the corner
    // the hull is already over the limit and the whole budget comes back, to
    // shed rate with.
    const available = (deg: number): number => {
        const spin = (deg * Math.PI) / 180;
        const centripetal = spin * spin;
        return centripetal <= structural
            ? Math.sqrt(structural * structural - centripetal * centripetal)
            : structural;
    };

    const svg = svgEl("svg", {
        viewBox: "0 0 560 200",
        role: "img",
        "aria-label":
            "Turn authority left against the rate the hull is already " +
            "turning at: flat at first, collapsing to nothing at the " +
            "sustained rate, and coming back beyond it so an over-spun hull " +
            "can brake.",
    });
    for (const frac of [0, 0.25, 0.5, 0.75, 1]) {
        const a = structural * frac;
        svg.appendChild(
            svgEl("line", {
                x1: String(X0),
                y1: String(y(a)),
                x2: String(X1),
                y2: String(y(a)),
                class: "widget-mark--grid",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(X0 - 5),
                    y: String(y(a) + 3),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                a.toFixed(1)
            )
        );
    }
    for (const deg of [0, 25, 50, 75]) {
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(x(deg)),
                    y: String(Y0 + 15),
                    "text-anchor": "middle",
                    class: "widget-mark--axis",
                },
                `${deg}`
            )
        );
    }
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(X1),
                y: String(Y0 + 15),
                "text-anchor": "end",
                class: "widget-mark--axis",
            },
            "turn rate, deg/s"
        )
    );
    // The over-spun band: reachable only when something else put the rate
    // there, so it is marked as ground the ship does not steer into.
    svg.appendChild(
        svgEl("rect", {
            x: String(x(sustainedDeg)),
            y: String(Y1),
            width: String(x(MAX_DEG) - x(sustainedDeg)),
            height: String(Y0 - Y1),
            class: "widget-mark--band",
        })
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String((x(sustainedDeg) + x(MAX_DEG)) / 2),
                y: String(Y1 + 12),
                "text-anchor": "middle",
                class: "widget-mark--detail",
            },
            "over-spun"
        )
    );
    const climb: string[] = [];
    for (let deg = 0; deg <= sustainedDeg; deg += sustainedDeg / 160) {
        climb.push(`${x(deg).toFixed(1)},${y(available(deg)).toFixed(1)}`);
    }
    svg.appendChild(
        svgEl("path", { d: `M${climb.join(" L")}`, class: "widget-mark--now" })
    );
    svg.appendChild(
        svgEl("path", {
            d: `M${x(sustainedDeg).toFixed(1)},${y(structural).toFixed(1)} L${x(MAX_DEG).toFixed(1)},${y(structural).toFixed(1)}`,
            class: "widget-mark--now",
        })
    );
    svg.appendChild(
        svgEl("line", {
            x1: String(x(sustainedDeg)),
            y1: String(Y1),
            x2: String(x(sustainedDeg)),
            y2: String(Y0),
            class: "widget-mark--gate",
        })
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(x(sustainedDeg) - 5),
                y: String(Y1 + 12),
                "text-anchor": "end",
                class: "widget-mark--label-gate",
            },
            `committed at ${sustainedDeg.toFixed(0)} deg/s`
        )
    );
    const cursor = svgEl("line", {
        y1: String(Y1),
        y2: String(Y0),
        class: "widget-mark--cursor",
    });
    svg.appendChild(cursor);
    const dot = svgEl("circle", { r: "5", class: "widget-mark--dot-now" });
    svg.appendChild(dot);
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stats");
    const rateStat = stat(stats, "turn rate");
    const leftStat = stat(stats, "authority left");
    const spentStat = stat(stats, "budget spent holding it");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const deg = Number(rateControl.input.value);
        const left = available(deg);
        cursor.setAttribute("x1", String(x(deg)));
        cursor.setAttribute("x2", String(x(deg)));
        dot.setAttribute("cx", String(x(deg)));
        dot.setAttribute("cy", String(y(left)));

        rateStat.textContent = `${deg.toFixed(0)} deg/s`;
        leftStat.textContent = `${left.toFixed(2)} rad/s^2`;
        spentStat.textContent = `${Math.round(100 - (100 * left) / structural)}%`;
        if (deg < sustainedDeg * 0.5) {
            readout.classList.remove("is-warn");
            readout.textContent =
                `At ${deg.toFixed(0)} deg/s the corvette has ` +
                `${Math.round((100 * left) / structural)}% of its budget ` +
                "still in hand. Below about half the committed rate the " +
                "sideways load is a rounding error and the ship tightens " +
                "as fast as it did from rest.";
        } else if (deg < sustainedDeg) {
            readout.classList.add("is-warn");
            readout.textContent =
                `At ${deg.toFixed(0)} deg/s holding the curve already costs ` +
                `${Math.round(100 - (100 * left) / structural)}% of the ` +
                "budget, and the last of it goes fast: the two loads add as " +
                "a vector, so authority does not taper - it holds, then " +
                "falls off a cliff.";
        } else {
            readout.classList.add("is-warn");
            readout.textContent =
                `Past ${sustainedDeg.toFixed(0)} deg/s the turn alone is over ` +
                "the limit. Nothing a player does gets here - a ram or a " +
                "blast does - and the ship gets its whole budget back to " +
                "SHED the rate with, rather than being trapped in a spin it " +
                "can never stop.";
        }
    };
    const rateControl = control(
        "Turn rate",
        0,
        Math.round(MAX_DEG),
        1,
        Math.round(sustainedDeg * 0.75),
        (v) => `${v} deg/s`,
        update
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(rateControl.row);

    const note = el(
        "p",
        "widget__note",
        "The corvette's own numbers: a " +
            `${engineMeters(arm, 1)} arm, so ${structural.toFixed(2)} rad/s^2 ` +
            `of budget and ${sustainedDeg.toFixed(0)} deg/s of committed ` +
            "turn. A longer ship commits earlier and a shorter one later, " +
            "but the shape of this curve is the same on every hull, because " +
            "the arm divides out of it."
    );

    host.appendChild(controls);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    update();
}

// ---- gravity-well ---------------------------------------------------------

// A pull-vs-distance profile of one well: the surface clamp plateau, the
// inverse-square slope, the smoothstep fade band, the SOI edge where the pull
// is exactly zero, and ORBIT's trusted band. Mass is the only authored
// quantity - reach and strength both fall out of the mu slider.
function initGravityWell(host: HTMLElement): void {
    header(
        host,
        "Well scope: the pull profile",
        "One gravity well, read along a ray from its center. The pull is " +
            "a = mu / r^2 off the authored mass alone - held at its surface " +
            "value below the rock (no slingshots), smoothstepped to exactly " +
            "zero across the outer 15% of the sphere of influence. Drag the " +
            "ship out and watch the zones hand over."
    );

    // A wider left gutter than the sibling plots: the top axis label carries
    // a unit ("33 m/s^2") and must not clip at the viewBox edge.
    const X0 = 68;
    const X1 = 548;
    const Y0 = 196;
    const Y1 = 16;
    const svg = svgEl("svg", {
        viewBox: "0 0 560 250",
        role: "img",
        "aria-label":
            "Pull against distance for one gravity well: a flat surface " +
            "clamp, an inverse-square falloff, a shaded fade band, the " +
            "sphere-of-influence edge at zero, and the ORBIT band marked " +
            "along the axis.",
    });
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);
    const stats = el("div", "widget__stats");
    const pullStat = stat(stats, "pull");
    const zoneStat = stat(stats, "zone");
    const soiStat = stat(stats, "SOI radius");
    const orbitStat = stat(stats, "orbit speed here");
    const readout = el("p", "widget__readout");

    let mu = INSPECTION_PLANETOID_MU;
    let bodyR = 90;
    let soi = soiRadius(mu, bodyR);
    let xMax = 600;
    let aMax = 1;
    let cursor: SVGLineElement;
    let dot: SVGCircleElement;
    const x = (r: number): number => X0 + (r / xMax) * (X1 - X0);
    const y = (a: number): number =>
        Y0 - (Math.min(a, aMax) / aMax) * (Y0 - Y1);

    const rebuild = (): void => {
        soi = soiRadius(mu, bodyR);
        xMax = Math.ceil((soi + 40) / 50) * 50;
        aMax = wellAccel(mu, 0, bodyR, soi) * 1.12;
        svg.replaceChildren();
        // Axis grid: quarter steps of the clamp value.
        const surface = wellAccel(mu, 0, bodyR, soi);
        for (const a of [surface * 0.5, surface]) {
            svg.appendChild(
                svgEl("line", {
                    x1: String(X0),
                    y1: String(y(a)),
                    x2: String(X1),
                    y2: String(y(a)),
                    class: "widget-mark--grid",
                })
            );
            svg.appendChild(
                svgEl(
                    "text",
                    {
                        x: String(X0 - 6),
                        y: String(y(a) + 3),
                        "text-anchor": "end",
                        class: "widget-mark--axis",
                    },
                    a === surface
                        ? engineMetersPerSec2(a, 0)
                        : numText(a * METERS_PER_UNIT, 0)
                )
            );
        }
        for (const r of [100, 200, 300, 400, 500, 600]) {
            if (r > xMax) break;
            const last = r + 100 > xMax;
            svg.appendChild(
                svgEl(
                    "text",
                    {
                        x: String(last ? x(r) + 6 : x(r)),
                        y: String(Y0 + 16),
                        "text-anchor": last ? "end" : "middle",
                        class: "widget-mark--axis",
                    },
                    last
                        ? engineKilometers(r, 1)
                        : numText((r * METERS_PER_UNIT) / 1000, 1)
                )
            );
        }
        // The rock itself, and the fade band on the glass.
        svg.appendChild(
            svgEl("rect", {
                x: String(X0),
                y: String(Y1),
                width: String(x(bodyR) - X0),
                height: String(Y0 - Y1),
                class: "widget-mark--shadow",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(X0 + 4),
                    y: String(Y1 + 12),
                    class: "widget-mark--axis",
                },
                "rock"
            )
        );
        const fadeStart = soi * (1 - GRAVITY_FADE_FRACTION);
        svg.appendChild(
            svgEl("rect", {
                x: String(x(fadeStart)),
                y: String(Y1),
                width: String(x(soi) - x(fadeStart)),
                height: String(Y0 - Y1),
                class: "widget-mark--band",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(x(fadeStart) - 4),
                    y: String(Y1 + 12),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                "fade band"
            )
        );
        // SOI edge: past it the well does not exist.
        svg.appendChild(
            svgEl("line", {
                x1: String(x(soi)),
                y1: String(Y1),
                x2: String(x(soi)),
                y2: String(Y0),
                class: "widget-mark--old",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(x(soi) + 4),
                    y: String(Y1 + 26),
                    class: "widget-mark--label-old",
                },
                `SOI ${engineMeters(soi)}`
            )
        );
        // ORBIT's trusted band, as a strip on the axis.
        const band = orbitBand(bodyR, soi);
        if (band) {
            svg.appendChild(
                svgEl("line", {
                    x1: String(x(band.min)),
                    y1: String(Y0 + 24),
                    x2: String(x(band.max)),
                    y2: String(Y0 + 24),
                    class: "widget-mark--now",
                })
            );
            svg.appendChild(
                svgEl(
                    "text",
                    {
                        x: String(x((band.min + band.max) / 2)),
                        y: String(Y0 + 38),
                        "text-anchor": "middle",
                        class: "widget-mark--label-now",
                    },
                    "ORBIT band"
                )
            );
        }
        // The pull profile itself.
        const pts: string[] = [];
        for (let r = 0; r <= xMax; r += xMax / 260) {
            pts.push(
                `${x(r).toFixed(1)},${y(wellAccel(mu, r, bodyR, soi)).toFixed(1)}`
            );
        }
        svg.appendChild(
            svgEl("path", {
                d: `M${pts.join(" L")}`,
                class: "widget-mark--now",
            })
        );
        cursor = svgEl("line", {
            y1: String(Y1),
            y2: String(Y0),
            class: "widget-mark--cursor",
        });
        dot = svgEl("circle", { r: "4", class: "widget-mark--dot-now" });
        svg.appendChild(cursor);
        svg.appendChild(dot);
    };

    const update = (): void => {
        const r = (Number(rControl.input.value) / 100) * xMax;
        const a = wellAccel(mu, r, bodyR, soi);
        cursor.setAttribute("x1", String(x(r)));
        cursor.setAttribute("x2", String(x(r)));
        dot.setAttribute("cx", String(x(r)));
        dot.setAttribute("cy", String(y(a)));
        pullStat.textContent = engineMetersPerSec2(a, 1);
        const fadeStart = soi * (1 - GRAVITY_FADE_FRACTION);
        const zone =
            r >= soi
                ? "OUTSIDE THE SOI"
                : r > fadeStart
                  ? "FADE BAND"
                  : r <= bodyR + GRAVITY_SURFACE_MARGIN
                    ? "ON THE CLAMP"
                    : "INVERSE SQUARE";
        zoneStat.textContent = zone;
        soiStat.textContent = engineMeters(soi);
        const band = orbitBand(bodyR, soi);
        const inBand = band !== null && r >= band.min && r <= band.max;
        orbitStat.textContent =
            r >= soi || r <= bodyR
                ? "--"
                : `${engineMetersPerSec(circularOrbitSpeed(mu, r))}` +
                  (inBand ? "" : " (outside the ORBIT band)");
        readout.classList.remove("is-warn");
        if (r >= soi) {
            readout.textContent =
                "Outside the sphere of influence the well does not exist " +
                "as far as your ship is concerned - the pull is exactly zero.";
        } else if (r > fadeStart) {
            readout.textContent =
                "Inside the fade band: the pull smoothsteps to zero at the " +
                "edge, so there is no force discontinuity to bump across.";
        } else if (r <= bodyR + GRAVITY_SURFACE_MARGIN) {
            readout.textContent =
                "Below the surface margin the pull is held at its surface " +
                "value - grazing the rock is a bump, not a slingshot.";
            readout.classList.add("is-warn");
        } else if (inBand) {
            readout.textContent =
                `Clean inverse square. ORBIT would accept a ring here, at ` +
                `${engineMetersPerSec(circularOrbitSpeed(mu, r))} tangential.`;
        } else {
            readout.textContent =
                "Clean inverse square - but outside ORBIT's trusted band " +
                "(1.5x clearance off the rock, safely inside the fade).";
        }
    };
    const onParam = (): void => {
        mu = Number(muControl.input.value);
        bodyR = Number(radiusControl.input.value);
        rebuild();
        update();
    };
    const muControl = control(
        "Body mass (mu)",
        4000,
        48000,
        1000,
        INSPECTION_PLANETOID_MU,
        (v) => String(v),
        onParam
    );
    const radiusControl = control(
        "Body radius",
        20,
        120,
        5,
        90,
        (v) => engineMeters(v),
        onParam
    );
    const rControl = control(
        "Ship distance",
        0,
        100,
        1,
        40,
        (v) => engineMeters(Math.round((v / 100) * xMax)),
        update
    );
    // The distance fader spans the live scope range, so its readout re-labels
    // when mass changes the SOI.
    const relabel = (): void => {
        const val = rControl.row.querySelector(".widget__value");
        if (val)
            val.textContent = engineMeters(
                Math.round((Number(rControl.input.value) / 100) * xMax)
            );
    };
    muControl.input.addEventListener("input", relabel);
    radiusControl.input.addEventListener("input", relabel);
    const controls = el("div", "widget__controls");
    controls.appendChild(muControl.row);
    controls.appendChild(radiusControl.row);
    controls.appendChild(rControl.row);

    const note = el(
        "p",
        "widget__note",
        "Defaults are the campaign inspection planetoid (mass 27000, drawn rock " +
            "~900 m): a 3.29 km sphere of influence. The concealment planetoid " +
            "authors 20000. The drawn rock is bigger than the body's " +
            "authored nominal radius; the surface clamp bites at the drawn " +
            "surface."
    );

    host.appendChild(controls);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    rebuild();
    update();
}

// ---- dominant-well --------------------------------------------------------

// Two overlapping wells on one line: the strongest pull owns the ship, but
// the incumbent keeps ownership until a challenger clearly beats it (a 1.1x
// margin), so dragging the ship back and forth shows the sticky handoff.
function initDominantWell(host: HTMLElement): void {
    const D = 600; // separation of the two well centers, world units
    const RADIUS_A = 60;
    const RADIUS_B = 60;
    header(
        host,
        "Handoff scope: the dominant well",
        `Two wells ${engineMeters(D)} apart. Where their spheres of influence overlap ` +
            "the pulls do not blend - you feel only the DOMINANT well, and " +
            "it keeps ownership until a challenger pulls more than 1.10x " +
            "harder. Drag the ship across the boundary both ways: the " +
            "handoff point depends on where you came from."
    );

    const X0 = 44;
    const X1 = 548;
    const Y0 = 168;
    const Y1 = 16;
    const svg = svgEl("svg", {
        viewBox: "0 0 560 224",
        role: "img",
        "aria-label":
            "Pull curves of two overlapping gravity wells against position " +
            "on the line between them, with the hysteresis window shaded " +
            "and a cursor at the ship position.",
    });
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);
    const stats = el("div", "widget__stats");
    const pullAStat = stat(stats, "pull from A");
    const pullBStat = stat(stats, "pull from B");
    const ownerStat = stat(stats, "dominant well");
    const readout = el("p", "widget__readout");

    const muA = INSPECTION_PLANETOID_MU;
    const muB = CONCEALMENT_PLANETOID_MU;
    let owner: number | null = null;
    let cursor: SVGLineElement;
    let dotA: SVGCircleElement;
    let dotB: SVGCircleElement;
    let window0 = 0;
    let window1 = 0;
    const x = (p: number): number => X0 + (p / D) * (X1 - X0);
    const pulls = (p: number): [number, number] => [
        wellAccel(muA, p, RADIUS_A, soiRadius(muA, RADIUS_A)),
        wellAccel(muB, D - p, RADIUS_B, soiRadius(muB, RADIUS_B)),
    ];

    const rebuild = (): void => {
        // The hysteresis window: between the point where A can no longer be
        // taken from (moving right) and where B can (scanned numerically).
        window0 = D;
        window1 = 0;
        for (let p = 0; p <= D; p += 1) {
            const [a, b] = pulls(p);
            const aHolds = a > 0 && b <= a * WELL_SWITCH_HYSTERESIS;
            const bHolds = b > 0 && a <= b * WELL_SWITCH_HYSTERESIS;
            if (aHolds && bHolds) {
                window0 = Math.min(window0, p);
                window1 = Math.max(window1, p);
            }
        }
        const aMax =
            Math.max(
                wellAccel(
                    muA,
                    RADIUS_A + 40,
                    RADIUS_A,
                    soiRadius(muA, RADIUS_A)
                ),
                wellAccel(
                    muB,
                    RADIUS_B + 40,
                    RADIUS_B,
                    soiRadius(muB, RADIUS_B)
                )
            ) * 1.1;
        const y = (a: number): number =>
            Y0 - (Math.min(a, aMax) / aMax) * (Y0 - Y1);
        svg.replaceChildren();
        for (const p of [0, 150, 300, 450, 600]) {
            svg.appendChild(
                svgEl(
                    "text",
                    {
                        x: String(x(p)),
                        y: String(Y0 + 16),
                        "text-anchor": p === D ? "end" : "middle",
                        class: "widget-mark--axis",
                    },
                    p === 0 ? "WELL A" : p === D ? "WELL B" : engineMeters(p)
                )
            );
        }
        if (window1 > window0) {
            svg.appendChild(
                svgEl("rect", {
                    x: String(x(window0)),
                    y: String(Y1),
                    width: String(x(window1) - x(window0)),
                    height: String(Y0 - Y1),
                    class: "widget-mark--band",
                })
            );
            svg.appendChild(
                svgEl(
                    "text",
                    {
                        x: String(x((window0 + window1) / 2)),
                        y: String(Y1 + 12),
                        "text-anchor": "middle",
                        class: "widget-mark--axis",
                    },
                    "hysteresis window"
                )
            );
        }
        // Off-scale samples (the surface clamps near each rock dwarf the
        // crossover the scope is scaled for) leave a gap instead of drawing
        // a false plateau along the top edge.
        const pathOf = (which: 0 | 1): string => {
            let d = "";
            let pen = false;
            for (let p = 0; p <= D; p += 2) {
                const a = pulls(p)[which];
                if (a > aMax) {
                    pen = false;
                    continue;
                }
                d += `${pen ? " L" : " M"}${x(p).toFixed(1)} ${y(a).toFixed(1)}`;
                pen = true;
            }
            return d.trim();
        };
        svg.appendChild(
            svgEl("path", { d: pathOf(0), class: "widget-mark--now" })
        );
        svg.appendChild(
            svgEl("path", { d: pathOf(1), class: "widget-mark--old" })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(x(90)),
                    y: String(y(pulls(90)[0]) - 8),
                    class: "widget-mark--label-now",
                },
                "pull from A"
            )
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(x(D - 130)),
                    y: String(y(pulls(D - 130)[1]) - 8),
                    "text-anchor": "start",
                    class: "widget-mark--label-old",
                },
                "pull from B"
            )
        );
        cursor = svgEl("line", {
            y1: String(Y1),
            y2: String(Y0),
            class: "widget-mark--cursor",
        });
        dotA = svgEl("circle", { r: "4", class: "widget-mark--dot-now" });
        dotB = svgEl("circle", { r: "4", class: "widget-mark--dot-old" });
        svg.appendChild(cursor);
        svg.appendChild(dotA);
        svg.appendChild(dotB);
        const aMaxRef = aMax;
        yRef = (a: number): number =>
            Y0 - (Math.min(a, aMaxRef) / aMaxRef) * (Y0 - Y1);
    };
    let yRef: (a: number) => number = () => Y0;

    const update = (): void => {
        const p = Number(posControl.input.value);
        const [a, b] = pulls(p);
        owner = dominantWell(owner, [a, b]);
        cursor.setAttribute("x1", String(x(p)));
        cursor.setAttribute("x2", String(x(p)));
        dotA.setAttribute("cx", String(x(p)));
        dotA.setAttribute("cy", String(yRef(a)));
        dotB.setAttribute("cx", String(x(p)));
        dotB.setAttribute("cy", String(yRef(b)));
        pullAStat.textContent = engineMetersPerSec2(a, 1);
        pullBStat.textContent = engineMetersPerSec2(b, 1);
        ownerStat.textContent =
            owner === null ? "NONE" : owner === 0 ? "WELL A" : "WELL B";
        readout.classList.remove("is-warn");
        if (owner === null) {
            readout.textContent =
                "Outside both spheres of influence - no well owns the ship.";
        } else {
            const incumbent = owner === 0 ? a : b;
            const challenger = owner === 0 ? b : a;
            const ratio = incumbent > 0 ? challenger / incumbent : 0;
            if (p >= window0 && p <= window1) {
                readout.textContent =
                    `Inside the hysteresis window: the challenger pulls ` +
                    `${ratio.toFixed(2)}x the incumbent - it needs more than ` +
                    "1.10x to take over, so ownership keeps whichever well " +
                    "had you last. ORBIT flies the owner.";
                readout.classList.add("is-warn");
            } else {
                readout.textContent =
                    `${owner === 0 ? "Well A" : "Well B"} owns the ship ` +
                    "outright - its pull is what your hull and the ORBIT " +
                    "verb feel; the other well might as well not exist.";
            }
        }
    };
    const posControl = control(
        "Ship position",
        0,
        D,
        2,
        120,
        (v) => engineMeters(v),
        update
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(posControl.row);

    const note = el(
        "p",
        "widget__note",
        "Well A is the campaign inspection planetoid (mass 27000), well B " +
            "the concealment planetoid (mass 20000); both drawn at 600 m here so " +
            "the curves stay legible. The pick and the 1.10x margin are the " +
            "game's own dominant_well rule."
    );

    host.appendChild(controls);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    rebuild();
    update();
}

// ---- goto-verb ------------------------------------------------------------

// A GOTO leg on the range: burn out on the speed envelope, swing retrograde
// at the flip line, brake at margin, ease onto the standoff with RCS. The
// scope replays the resolved 1D leg in scope time with the real phase chip.
function initGotoVerb(host: HTMLElement): void {
    const TARGET_RADIUS = 20; // the widget's fixture body, world units
    header(
        host,
        "Autopilot scope: one GOTO leg",
        "GOTO flies the real hull: it burns toward the lock while the " +
            "arrival envelope allows, swings retrograde one flip early, " +
            "brakes at 85% of what the drive can do, and eases the last " +
            "stretch onto a standoff 500 m off the surface with the fine " +
            "jets. Play the tape; the plot shows speed against the envelope."
    );

    // Lane geometry (top): start at left, body at right.
    const LX0 = 24;
    const LX1 = 548;
    const LY = 46;
    // Plot geometry (below): speed vs travelled distance.
    const X0 = 44;
    const X1 = 548;
    const Y0 = 240;
    const Y1 = 106;
    const svg = svgEl("svg", {
        viewBox: "0 0 560 268",
        role: "img",
        "aria-label":
            "A GOTO leg: a ship glyph crossing a lane toward a target body " +
            "with a standoff mark, above a plot of speed against distance " +
            "with the arrival envelope, the flip point and the brake ramp.",
    });
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);
    const stats = el("div", "widget__stats");
    const peakStat = stat(stats, "peak speed");
    const flipStat = stat(stats, "flip at");
    const etaStat = stat(stats, "leg time");
    const standoffStat = stat(stats, "standoff");
    const readout = el("p", "widget__readout");

    let sim = gotoSim(1200, TARGET_RADIUS, 8);
    let targetDistance = 1200;
    let xMaxV = 1;
    let ship: SVGPathElement;
    let plume: SVGLineElement;
    let phaseText: SVGTextElement;
    let traceEl: SVGPathElement;
    let tracePts: string[] = [];
    let cursorDot: SVGCircleElement;
    const lx = (p: number): number => LX0 + (p / targetDistance) * (LX1 - LX0);
    const px = (p: number): number => X0 + (p / targetDistance) * (X1 - X0);
    const py = (v: number): number =>
        Y0 - (Math.min(v, xMaxV) / xMaxV) * (Y0 - Y1);

    const rebuild = (): void => {
        targetDistance = Number(distControl.input.value);
        const accel = Number(accelControl.input.value);
        sim = gotoSim(targetDistance, TARGET_RADIUS, accel);
        xMaxV = sim.peakV * 1.18;
        svg.replaceChildren();
        // Lane: track, body, standoff tick, park point.
        svg.appendChild(
            svgEl("line", {
                x1: String(LX0),
                y1: String(LY),
                x2: String(LX1),
                y2: String(LY),
                class: "widget-mark--grid",
            })
        );
        const park = targetDistance - sim.standoff;
        // Body radius 11 keeps the disc inside the 560 viewBox at LX1 548.
        svg.appendChild(
            svgEl("circle", {
                cx: String(LX1),
                cy: String(LY),
                r: "11",
                class: "widget-mark--shadow-stroke",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(LX1),
                    y: String(LY + 30),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                "target"
            )
        );
        svg.appendChild(
            svgEl("line", {
                x1: String(lx(park)),
                y1: String(LY - 12),
                x2: String(lx(park)),
                y2: String(LY + 12),
                class: "widget-mark--gate",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(lx(park)),
                    y: String(LY - 18),
                    "text-anchor": "middle",
                    class: "widget-mark--label-gate",
                },
                "standoff"
            )
        );
        phaseText = svgEl(
            "text",
            {
                x: String(LX0),
                y: String(LY - 18),
                class: "widget-mark--word",
            },
            "AP GOTO - ALIGN"
        );
        svg.appendChild(phaseText);
        plume = svgEl("line", {
            y1: String(LY),
            y2: String(LY),
            class: "widget-mark--plume",
            visibility: "hidden",
        });
        svg.appendChild(plume);
        ship = svgEl("path", { d: "", class: "widget-mark--ship" });
        svg.appendChild(ship);
        // Plot: axes, envelope, flip and park gates, then the live trace.
        for (const v of [Math.round(sim.peakV / 2), Math.round(sim.peakV)]) {
            if (v <= 0) continue;
            svg.appendChild(
                svgEl("line", {
                    x1: String(X0),
                    y1: String(py(v)),
                    x2: String(X1),
                    y2: String(py(v)),
                    class: "widget-mark--grid",
                })
            );
            svg.appendChild(
                svgEl(
                    "text",
                    {
                        x: String(X0 - 6),
                        y: String(py(v) + 3),
                        "text-anchor": "end",
                        class: "widget-mark--axis",
                    },
                    engineMetersPerSec(v)
                )
            );
        }
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(X1),
                    y: String(Y0 + 14),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                engineMeters(targetDistance)
            )
        );
        // The arrival envelope: the fastest speed the flip still recovers
        // from, drawn against distance travelled.
        const turnRate = hullTurnRate(structuralCeiling(CORVETTE_ARM_U));
        const lead = Math.PI / turnRate + ARRIVAL_SPOOL_PAD;
        const env: string[] = [];
        for (let p = 0; p <= park; p += targetDistance / 200) {
            const v = arrivalSpeedLimit(
                park - p,
                Number(accelControl.input.value),
                lead
            );
            env.push(`${px(p).toFixed(1)},${py(v).toFixed(1)}`);
        }
        svg.appendChild(
            svgEl("path", {
                d: `M${env.join(" L")}`,
                class: "widget-mark--old",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(px(park * 0.62)),
                    y: String(
                        py(
                            arrivalSpeedLimit(
                                park * 0.38,
                                Number(accelControl.input.value),
                                lead
                            )
                        ) - 8
                    ),
                    "text-anchor": "middle",
                    class: "widget-mark--label-old",
                },
                "arrival envelope"
            )
        );
        svg.appendChild(
            svgEl("line", {
                x1: String(px(sim.flipX)),
                y1: String(Y1),
                x2: String(px(sim.flipX)),
                y2: String(Y0),
                class: "widget-mark--cursor",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(px(sim.flipX)),
                    y: String(Y1 - 4),
                    "text-anchor": "middle",
                    class: "widget-mark--axis",
                },
                "flip"
            )
        );
        svg.appendChild(
            svgEl("line", {
                x1: String(px(park)),
                y1: String(Y1),
                x2: String(px(park)),
                y2: String(Y0),
                class: "widget-mark--gate",
            })
        );
        tracePts = sim.samples.map(
            (s) => `${px(s.x).toFixed(1)},${py(s.v).toFixed(1)}`
        );
        traceEl = svgEl("path", { d: "", class: "widget-mark--now" });
        svg.appendChild(traceEl);
        cursorDot = svgEl("circle", { r: "4", class: "widget-mark--dot-now" });
        svg.appendChild(cursorDot);
        // Resolved stats.
        peakStat.textContent = `${engineMetersPerSec(sim.peakV)}`;
        flipStat.textContent = `${engineMeters(sim.flipX)} out, T+${sim.flipT.toFixed(1)}s`;
        etaStat.textContent = `${sim.duration.toFixed(1)} s`;
        standoffStat.textContent =
            `${engineMeters(sim.standoff)} off the center ` +
            `(${engineMeters(ARRIVAL_STANDOFF)} + the body + this hull)`;
    };

    const PHASE_LABEL: Record<GotoSample["phase"], string> = {
        // The HUD chip only knows ALIGN (engines cold) and BURN; the scope
        // annotates what the leg is doing inside them.
        burn: "AP GOTO - BURN",
        flip: "AP GOTO - ALIGN (flip)",
        brake: "AP GOTO - BURN (brake)",
        settle: "AP GOTO - ALIGN (RCS settle)",
    };
    const renderFrame = (t: number): void => {
        const idx = Math.min(
            sim.samples.length - 1,
            Math.round(t * 60) // samples at the sim's fixed 60 Hz
        );
        const s = sim.samples[idx];
        const sx = lx(s.x);
        const forward = s.phase === "burn";
        // A small side-profile dart, nose toward or away from the target.
        const nose = forward ? 12 : -12;
        ship.setAttribute(
            "d",
            `M${(sx + nose).toFixed(1)} ${LY} L${(sx - nose * 0.7).toFixed(1)} ` +
                `${LY - 5} L${(sx - nose * 0.7).toFixed(1)} ${LY + 5} Z`
        );
        const burning = s.phase === "burn" || s.phase === "brake";
        plume.setAttribute(
            "visibility",
            burning && s.v > 0 ? "visible" : "hidden"
        );
        const tail = -nose * 0.7;
        plume.setAttribute("x1", String(sx + tail));
        plume.setAttribute("x2", String(sx + tail + (forward ? -14 : 14)));
        phaseText.textContent = PHASE_LABEL[s.phase];
        traceEl.setAttribute(
            "d",
            idx > 0 ? `M${tracePts.slice(0, idx + 1).join(" L")}` : ""
        );
        cursorDot.setAttribute("cx", String(px(s.x)));
        cursorDot.setAttribute("cy", String(py(s.v)));
        if (s.phase === "burn") {
            readout.textContent =
                `Burning out at ${engineMetersPerSec(s.v)} - under the envelope, ` +
                "so the flip still recovers all of it.";
        } else if (s.phase === "flip") {
            readout.textContent =
                "On the flip line: engines cold while the hull swings " +
                "retrograde - the envelope already budgeted this coast.";
        } else if (s.phase === "brake") {
            readout.textContent =
                `Braking at 85% authority, riding the envelope down - the ` +
                "floor is the 15 m/s minimum approach.";
        } else {
            readout.textContent =
                "Inside the standoff: main drive cut, RCS eases the last " +
                "stretch to rest - arrivals settle, they do not pulse.";
        }
    };

    const transport = makeTransport(() => sim.duration, renderFrame);
    const onParam = (): void => {
        rebuild();
        transport.seekEnd();
    };
    const distControl = control(
        "Leg distance",
        300,
        3000,
        50,
        1200,
        (v) => engineMeters(v),
        onParam
    );
    const accelControl = control(
        "Drive acceleration",
        2,
        20,
        1,
        8,
        (v) => engineMetersPerSec2(v, 0),
        onParam
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(distControl.row);
    controls.appendChild(accelControl.row);

    const note = el(
        "p",
        "widget__note",
        "Simplified to one dimension: no gravity, one forward drive group " +
            "(so the brake angle is a full 180), a stationary target. The " +
            "hull is the shipped corvette, held by its own structure to " +
            "2.84 rad/s^2. The envelope, flip line, 85% brake margin, " +
            "15 m/s approach floor, standoff and RCS settle are the game's " +
            "own rules."
    );

    host.appendChild(controls);
    host.appendChild(transport.row);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    rebuild();
    if (reducedMotion()) transport.seekEnd();
    else transport.play();
}

// ---- lock-sweep -----------------------------------------------------------

// A hands-on radar trainer: hold the RADAR key to sweep, watch the slot
// latch at the tap threshold from your stance, hold the aim through the
// range-scaled dwell, tap to clear in stages. All timing constants are the
// game's own; the trainer is driven entirely by the reader's own hold.
function initLockSweep(host: HTMLElement): void {
    interface Contact {
        name: string;
        dist: number; // world units
        bearing: number; // deg from straight up
    }
    const CONTACTS: Contact[] = [
        { name: "TORPEDO", dist: 300, bearing: -34 },
        { name: "PICKET", dist: 900, bearing: 4 },
        { name: "FREIGHTER", dist: 1800, bearing: 38 },
    ];
    header(
        host,
        "Radar trainer: earn a lock",
        `Hold the RADAR key to sweep (a ${TARGETING_CONE_HALF_ANGLE_DEG}-` +
            `degree half-angle cone around your aim). Past the ` +
            `${RADAR_TAP_SECS}-second tap threshold the slot latches from ` +
            "your stance - lowered writes the white TRAVEL lock, raised " +
            "the red COMBAT lock - then a range-scaled dwell must fill " +
            "while you hold the aim. A release under the threshold is a " +
            "TAP: staged clearing."
    );

    // Scope geometry.
    const CX = 280;
    const CY = 224;
    const R_EDGE = 205;
    const rPx = (dist: number): number => 40 + (dist / 1800) * 160;
    const pos = (c: Contact): [number, number] => {
        const a = (c.bearing * Math.PI) / 180;
        return [CX + rPx(c.dist) * Math.sin(a), CY - rPx(c.dist) * Math.cos(a)];
    };

    const svg = svgEl("svg", {
        viewBox: "0 0 560 244",
        role: "img",
        "aria-label":
            "Radar scope: your ship at the bottom, three contacts at " +
            "different ranges, the sweep cone around your aim, a dwell " +
            "ring filling on the candidate, and committed locks drawn as " +
            "a white box or a red diamond.",
    });
    const cone = svgEl("path", { d: "", class: "widget-mark--cone" });
    svg.appendChild(cone);
    const ray = svgEl("line", { class: "widget-mark--ray" });
    svg.appendChild(ray);
    // Ship glyph at the scope origin.
    svg.appendChild(
        svgEl("path", {
            d: `M${CX} ${CY - 8} L${CX - 7} ${CY + 8} L${CX + 7} ${CY + 8} Z`,
            class: "widget-mark--ship",
        })
    );
    interface Blip {
        travelBox: SVGRectElement;
        combatBox: SVGRectElement;
        dwellRing: SVGCircleElement;
    }
    const blips: Blip[] = CONTACTS.map((c) => {
        const [bx, by] = pos(c);
        svg.appendChild(
            svgEl("circle", {
                cx: String(bx),
                cy: String(by),
                r: "4",
                class: "widget-mark--blip",
            })
        );
        const anchor = c.bearing < 0 ? "end" : "start";
        const tx = c.bearing < 0 ? bx - 10 : bx + 10;
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(tx),
                    y: String(by),
                    "text-anchor": anchor,
                    class: "widget-mark--word",
                },
                c.name
            )
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(tx),
                    y: String(by + 13),
                    "text-anchor": anchor,
                    class: "widget-mark--detail",
                },
                `${engineMeters(c.dist)} - dwell ${lockDwellSecs(c.dist).toFixed(2)} s`
            )
        );
        const travelBox = svgEl("rect", {
            x: String(bx - 11),
            y: String(by - 11),
            width: "22",
            height: "22",
            class: "widget-mark--navlock",
            visibility: "hidden",
        });
        svg.appendChild(travelBox);
        const combatBox = svgEl("rect", {
            x: String(bx - 10),
            y: String(by - 10),
            width: "20",
            height: "20",
            transform: `rotate(45 ${bx} ${by})`,
            class: "widget-mark--combatlock",
            visibility: "hidden",
        });
        svg.appendChild(combatBox);
        const dwellRing = svgEl("circle", {
            cx: String(bx),
            cy: String(by),
            r: "15",
            transform: `rotate(-90 ${bx} ${by})`,
            class: "widget-mark--dwell",
            visibility: "hidden",
        });
        svg.appendChild(dwellRing);
        return { travelBox, combatBox, dwellRing };
    });
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    // State.
    let raised = false;
    let aimed = 1;
    let held = false;
    let heldSecs = 0;
    let slot: "travel" | "combat" | null = null;
    let dwellOn: number | null = null;
    let dwellSecs = 0;
    let travelLock: number | null = null;
    let combatLock: number | null = null;
    let raf = 0;
    let last = 0;

    const stats = el("div", "widget__stats");
    const holdStat = stat(stats, "hold");
    const slotStat = stat(stats, "slot");
    const dwellStat = stat(stats, "dwell");
    const readout = el("p", "widget__readout");
    readout.textContent =
        "Pick an aim, set your stance, then press and hold RADAR. " +
        "Release early for a tap - that is the staged CLEAR gesture.";

    const paint = (): void => {
        // Cone and ray follow the aimed contact.
        const c = CONTACTS[aimed];
        const a = (c.bearing * Math.PI) / 180;
        const half = (TARGETING_CONE_HALF_ANGLE_DEG * Math.PI) / 180;
        const pt = (ang: number, r: number): string =>
            `${(CX + r * Math.sin(ang)).toFixed(1)} ${(CY - r * Math.cos(ang)).toFixed(1)}`;
        cone.setAttribute(
            "d",
            `M${CX} ${CY} L${pt(a - half, R_EDGE)} ` +
                `A${R_EDGE} ${R_EDGE} 0 0 1 ${pt(a + half, R_EDGE)} Z`
        );
        ray.setAttribute("x1", String(CX));
        ray.setAttribute("y1", String(CY));
        const [rx, ry] = [CX + R_EDGE * Math.sin(a), CY - R_EDGE * Math.cos(a)];
        ray.setAttribute("x2", String(rx));
        ray.setAttribute("y2", String(ry));
        blips.forEach((blip, i) => {
            blip.travelBox.setAttribute(
                "visibility",
                travelLock === i ? "visible" : "hidden"
            );
            blip.combatBox.setAttribute(
                "visibility",
                combatLock === i ? "visible" : "hidden"
            );
            const charging =
                held && slot !== null && dwellOn === i && dwellSecs > 0;
            if (charging) {
                const needed = lockDwellSecs(CONTACTS[i].dist);
                const frac = clamp(dwellSecs / needed, 0, 1);
                const circ = 2 * Math.PI * 15;
                blip.dwellRing.setAttribute("visibility", "visible");
                blip.dwellRing.setAttribute(
                    "stroke-dasharray",
                    `${(frac * circ).toFixed(1)} ${circ.toFixed(1)}`
                );
                blip.dwellRing.setAttribute(
                    "class",
                    `widget-mark--dwell${slot === "combat" ? " is-combat" : ""}`
                );
            } else {
                blip.dwellRing.setAttribute("visibility", "hidden");
            }
        });
        holdStat.textContent = held ? `${heldSecs.toFixed(2)} s` : "--";
        slotStat.textContent =
            slot === null
                ? held
                    ? "tap window"
                    : "--"
                : slot === "combat"
                  ? "COMBAT (red)"
                  : "TRAVEL (white)";
        if (held && slot !== null && dwellOn !== null) {
            const needed = lockDwellSecs(CONTACTS[dwellOn].dist);
            dwellStat.textContent = `${Math.min(dwellSecs, needed).toFixed(2)} / ${needed.toFixed(2)} s`;
        } else {
            dwellStat.textContent = "--";
        }
    };

    const tick = (now: number): void => {
        const dt = Math.min((now - last) / 1000, 0.1);
        last = now;
        heldSecs += dt;
        if (heldSecs >= RADAR_TAP_SECS && slot === null) {
            // The slot latches at the threshold from the stance at that
            // moment (radar.rs:108-114).
            slot = raised ? "combat" : "travel";
            readout.textContent =
                `Slot latched: ${slot === "combat" ? "COMBAT - weapons were raised" : "TRAVEL - weapons were lowered"} ` +
                "at the threshold. Now hold the aim while the dwell fills.";
        }
        if (slot !== null) {
            if (dwellOn !== aimed) {
                // Re-designating restarts the dwell from zero; the committed
                // lock keeps-last underneath (radar.rs:135-138).
                dwellOn = aimed;
                dwellSecs = 0;
            }
            dwellSecs += dt;
            const needed = lockDwellSecs(CONTACTS[aimed].dist);
            const already =
                (slot === "combat" ? combatLock : travelLock) === aimed;
            if (dwellSecs >= needed && !already) {
                if (slot === "combat") combatLock = aimed;
                else travelLock = aimed;
                readout.textContent =
                    `Lock committed: ${CONTACTS[aimed].name} in the ` +
                    `${slot.toUpperCase()} slot after ${needed.toFixed(2)} s. ` +
                    "It sticks - releasing just ends the sweep.";
            }
        } else if (held) {
            readout.textContent = `${heldSecs.toFixed(2)} s held - still inside the ${RADAR_TAP_SECS} s tap window; releasing now would CLEAR, not sweep.`;
        }
        paint();
        if (held) raf = requestAnimationFrame(tick);
    };
    const startHold = (): void => {
        if (held) return;
        held = true;
        heldSecs = 0;
        slot = null;
        dwellOn = null;
        dwellSecs = 0;
        last = performance.now();
        raf = requestAnimationFrame(tick);
    };
    const endHold = (): void => {
        if (!held) return;
        held = false;
        cancelAnimationFrame(raf);
        if (heldSecs < RADAR_TAP_SECS) {
            // A tap: staged clearing (gesture.rs:125-175).
            const step = clearStep(
                raised,
                combatLock !== null,
                travelLock !== null
            );
            if (step === "combat") {
                combatLock = null;
                readout.textContent =
                    "Tap: the COMBAT lock clears first. Tap again with " +
                    "weapons lowered to drop the travel lock too (that also " +
                    "disengages an engaged GOTO).";
            } else if (step === "travel") {
                travelLock = null;
                readout.textContent =
                    "Tap: the TRAVEL lock clears - and clearing the " +
                    "designation disengages an engaged GOTO.";
            } else if (raised && travelLock !== null) {
                readout.textContent =
                    "Tap with weapons RAISED never touches the travel lock " +
                    "- lower them and tap again.";
            } else {
                readout.textContent = "Tap: nothing left to clear.";
            }
        } else if (slot !== null) {
            const locked = slot === "combat" ? combatLock : travelLock;
            if (locked !== aimed) {
                readout.textContent =
                    "Released before the dwell filled - no lock. The dwell " +
                    "is earned by holding steady, not granted by pointing.";
            }
        }
        slot = null;
        dwellOn = null;
        dwellSecs = 0;
        paint();
    };

    // Controls: aim keys, the stance key, the radar key.
    const keys = el("div", "widget__keys");
    const aimBtns = CONTACTS.map((c, i) => {
        const b = el("button", "widget__btn", `AIM: ${c.name}`);
        b.type = "button";
        b.addEventListener("click", () => {
            aimed = i;
            aimBtns.forEach((btn, j) => btn.classList.toggle("is-on", j === i));
            paint();
        });
        keys.appendChild(b);
        return b;
    });
    aimBtns[aimed].classList.add("is-on");
    const stanceBtn = el("button", "widget__btn", "STANCE: LOWERED");
    stanceBtn.type = "button";
    stanceBtn.addEventListener("click", () => {
        raised = !raised;
        stanceBtn.textContent = raised ? "STANCE: RAISED" : "STANCE: LOWERED";
        stanceBtn.classList.toggle("is-hot", raised);
        paint();
    });
    keys.appendChild(stanceBtn);
    const radarBtn = el(
        "button",
        "widget__btn widget__btn--wide",
        "RADAR - HOLD TO SWEEP / TAP TO CLEAR"
    );
    radarBtn.type = "button";
    radarBtn.setAttribute(
        "aria-label",
        "Radar key: press and hold to sweep, release under a quarter " +
            "second to clear"
    );
    radarBtn.addEventListener("pointerdown", (e) => {
        e.preventDefault();
        radarBtn.setPointerCapture(e.pointerId);
        startHold();
    });
    radarBtn.addEventListener("pointerup", endHold);
    radarBtn.addEventListener("pointercancel", endHold);
    radarBtn.addEventListener("keydown", (e) => {
        if ((e.key === " " || e.key === "Enter") && !e.repeat) {
            e.preventDefault();
            startHold();
        }
    });
    radarBtn.addEventListener("keyup", (e) => {
        if (e.key === " " || e.key === "Enter") endHold();
    });
    radarBtn.addEventListener("blur", endHold);
    keys.appendChild(radarBtn);

    const note = el(
        "p",
        "widget__note",
        "Constants are the shipped ones: 18-degree cone half-angle, 0.25 s " +
            "tap threshold, dwell 0.6 s point-blank stretching to 1.5 s at " +
            "20 km, staged clearing. " +
            "Candidate picking is simplified to the aim keys; in game the " +
            "sweep scores whatever is nearest your look ray, with " +
            "hysteresis. An idle combat lock also decays after " +
            `${COMBAT_DECAY_SECS} s - the ` +
            "trainer leaves that clock out."
    );

    host.appendChild(keys);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    paint();
}

// ---- relation-matrix ------------------------------------------------------

// The whole faction model in one grid: pick any pair of sides and read the
// relation, the marker tint and what combat does with it.
function initRelationMatrix(host: HTMLElement): void {
    const SIDES: { key: Side; label: string }[] = [
        { key: "player", label: "PLAYER" },
        { key: "enemy", label: "ENEMY" },
        { key: "neutral", label: "NEUTRAL" },
        { key: "none", label: "UNMARKED" },
    ];
    header(
        host,
        "Relation matrix",
        "Any two things resolve to one of three relations: OWN, HOSTILE " +
            "or NEUTRAL. Only the combatant sides relate strongly - " +
            "anything Neutral or unmarked (asteroids, debris, salvage) " +
            "resolves NEUTRAL, even against itself. Pick a pair."
    );

    const matrix = el("div", "widget__matrix");
    matrix.appendChild(el("span", "widget__mhead", ""));
    for (const s of SIDES)
        matrix.appendChild(el("span", "widget__mhead", s.label));
    const readout = el("p", "widget__readout");
    let selected: [number, number] = [0, 1];
    const cells: HTMLButtonElement[][] = [];
    SIDES.forEach((rowSide, r) => {
        matrix.appendChild(el("span", "widget__mhead", rowSide.label));
        const row: HTMLButtonElement[] = [];
        SIDES.forEach((colSide, c) => {
            const rel = relation(rowSide.key, colSide.key);
            const cell = el("button", `widget__mcell is-${rel}`);
            cell.type = "button";
            cell.textContent = rel.toUpperCase();
            cell.addEventListener("click", () => {
                selected = [r, c];
                update();
            });
            matrix.appendChild(cell);
            row.push(cell);
        });
        cells.push(row);
    });

    const update = (): void => {
        const [r, c] = selected;
        cells.forEach((row, ri) =>
            row.forEach((cell, ci) =>
                cell.classList.toggle("is-sel", ri === r && ci === c)
            )
        );
        const a = SIDES[r];
        const b = SIDES[c];
        const rel = relation(a.key, b.key);
        if (rel === "own") {
            readout.textContent =
                `${a.label} vs ${b.label}: OWN - the same combatant side. ` +
                "Your projectiles copy your side at launch and keep it even " +
                "if you die, so your own torpedo never reads as a target " +
                "and never tempts your point defense.";
        } else if (rel === "hostile") {
            readout.textContent =
                `${a.label} vs ${b.label}: HOSTILE - valid targets for each ` +
                "other. This is what AI acquisition hunts, what raises an " +
                "enemy's weapons, and what the HUD paints threat-red.";
        } else {
            readout.textContent =
                `${a.label} vs ${b.label}: NEUTRAL - out of the fight. ` +
                "Neutral never relates strongly, not even to another " +
                "neutral; the AI ignores it and the HUD marks it steel-grey " +
                "until a scenario re-aligns it mid-flight.";
        }
    };
    update();

    const note = el(
        "p",
        "widget__note",
        "The marker triangle over a ship wears the relation palette: ally " +
            "green for your side (AI wingmen included), threat red for " +
            "hostiles, light steel for neutral or unmarked. The viewfinder " +
            "caption carries the same relation as text."
    );

    host.appendChild(matrix);
    host.appendChild(readout);
    host.appendChild(note);
}

// ---- hud-context ----------------------------------------------------------

// The contextual HUD as a switchboard: flip the situation keys and watch
// which elements are up. The rules are the hudElements model above, lifted
// from the nova_hud drivers.
function initHudContext(host: HTMLElement): void {
    header(
        host,
        "HUD switchboard: what shows when",
        "The HUD is contextual: elements surface while their situation is " +
            "live and settle back when it passes. Flip the situation keys " +
            "- the board shows every element with the state it would be " +
            "in. Grave/tilde toggles the whole display; Cinematic clears " +
            "every tier."
    );

    const state: HudSituationsModel = {
        autopilot: false,
        combatLock: false,
        weaponsHot: false,
        lowAmmo: false,
        reloading: false,
        cinematic: false,
    };
    const KEYS: { key: keyof HudSituationsModel; label: string }[] = [
        { key: "autopilot", label: "AUTOPILOT ENGAGED" },
        { key: "combatLock", label: "COMBAT LOCK" },
        { key: "weaponsHot", label: "WEAPONS HOT" },
        { key: "lowAmmo", label: "LOW AMMO" },
        { key: "reloading", label: "RELOADING" },
        { key: "cinematic", label: "CINEMATIC" },
    ];
    const keysRow = el("div", "widget__keys");
    const board = el("div", "widget__stack widget__stack--board");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const elements = hudElements(state);
        board.replaceChildren();
        for (const item of elements) {
            const cell = el("div", "widget__cell widget__cell--board");
            cell.classList.add(item.on ? "is-live" : "is-off");
            cell.appendChild(el("b", undefined, item.name));
            cell.appendChild(
                el(
                    "span",
                    "widget__kind",
                    `${item.kind.toUpperCase()} - ${item.on ? "ON" : "OFF"}`
                )
            );
            cell.appendChild(document.createTextNode(item.detail));
            board.appendChild(cell);
        }
        const upCount = elements.filter((i) => i.on).length;
        if (state.cinematic) {
            readout.textContent =
                "Cinematic: a clean screen for captures and quiet flying - " +
                "every tier clears, Instrument, Chrome and Status alike.";
        } else if (upCount <= 6) {
            readout.textContent =
                "Idle cruise: the sphere, your speed, the dock's few live " +
                "verbs, the always-on markers - a quiet screen without you " +
                "managing it.";
        } else {
            readout.textContent = `${upCount} of ${elements.length} elements up - each arrived with its situation and will settle back when it passes.`;
        }
    };
    for (const { key, label } of KEYS) {
        const btn = el("button", "widget__btn", label);
        btn.type = "button";
        btn.setAttribute("aria-pressed", "false");
        btn.addEventListener("click", () => {
            state[key] = !state[key];
            btn.classList.toggle("is-on", state[key]);
            btn.setAttribute("aria-pressed", String(state[key]));
            update();
        });
        keysRow.appendChild(btn);
    }

    const note = el(
        "p",
        "widget__note",
        "Firing, RCS (a violet sphere palette that wins over cyan) and the " +
            "in-well gravity sphere are extra states the board folds away; " +
            "the objective and comms stacks arrive event-driven with their " +
            "own dwells. Only the ammo layer uses the central context gate " +
            "- the rest drive their own anchors."
    );

    host.appendChild(keysRow);
    host.appendChild(board);
    host.appendChild(readout);
    host.appendChild(note);
    update();
}

// ---- NOVA OS surfaces ------------------------------------------------------

// The NOVA OS command set and where each command lands. Names, summaries and
// dispatch are the registered command tree: core builtins
// (crates/nova_os/src/command.rs:166-175), the map tree
// (crates/nova_os_ui/src/map/mod.rs:96-113) and the ship tree
// (crates/nova_os_ui/src/ship/mod.rs:142-169); dispatch classes are
// crates/nova_os/src/shell.rs:50-80 (Cli prints, App takes the screen,
// Gameplay acts on the live ship and prints the result).
type NovaOsDispatch = "print" | "app" | "action" | "close";
interface NovaOsCommand {
    name: string;
    summary: string;
    dispatch: NovaOsDispatch;
    app?: "map" | "ship";
    outcome: string;
}
const NOVA_OS_COMMANDS: NovaOsCommand[] = [
    // command.rs:168-173 (names + summaries verbatim).
    {
        name: "help",
        summary: "Show this command list",
        dispatch: "print",
        outcome:
            "Prints the command list into the scrollback " +
            "(every command also answers `<command> help`).",
    },
    {
        name: "log",
        summary: "Print comms and mission events",
        dispatch: "print",
        outcome:
            "Prints the flight log: comms lines and posted / " +
            "completed objective events.",
    },
    {
        name: "objectives",
        summary: "Print active objectives",
        dispatch: "print",
        outcome: "Prints the active objectives.",
    },
    {
        name: "clear",
        summary: "Clear terminal scrollback",
        dispatch: "print",
        outcome: "Clears the scrollback back to the boot report.",
    },
    {
        name: "version",
        summary: "Print the NOVA OS version",
        dispatch: "print",
        outcome: "Prints the version banner.",
    },
    {
        name: "exit",
        summary: "Suspend the NOVA OS computer",
        dispatch: "close",
        outcome:
            "Powers the monitor off - the picture collapses to a dot, " +
            "then flight resumes.",
    },
    // map/mod.rs:99-112.
    {
        name: "map",
        summary: "Open the local-space map",
        dispatch: "app",
        app: "map",
        outcome:
            "Hands the screen to the MAP app; the footer swaps to its keys.",
    },
    {
        name: "map view",
        summary: "Print local-space contacts",
        dispatch: "print",
        outcome: "Prints the local-space contact table into the scrollback.",
    },
    {
        name: "map goto <label>",
        summary: "Fly the ship to a contact label",
        dispatch: "action",
        outcome:
            "Engages the flight autopilot toward the contact and prints " +
            "the result; the burn continues after the computer closes.",
    },
    // ship/mod.rs:143-168.
    {
        name: "ship",
        summary: "Open the ship computer",
        dispatch: "app",
        app: "ship",
        outcome:
            "Hands the screen to the SHIP app; the footer swaps to its keys.",
    },
    {
        name: "ship view",
        summary: "Print ship status summary",
        dispatch: "print",
        outcome: "Prints the ship status table into the scrollback.",
    },
    {
        name: "ship section <id>",
        summary: "Show one section's detail",
        dispatch: "action",
        outcome:
            "Prints one section's detail: kind, integrity bar, status, ammo.",
    },
    {
        name: "ship reload <id>",
        summary: "Reload a weapon section",
        dispatch: "action",
        outcome:
            "Reloads that weapon section on the live ship and prints " +
            "the new ammo count.",
    },
    {
        name: "ship repair <id>",
        summary: "Repair a section",
        dispatch: "action",
        outcome:
            "Repairs that section on the live ship and prints the " +
            "restored integrity.",
    },
];

// The three surfaces a command can leave you on. Breadcrumb format:
// crates/nova_os_ui/src/terminal/content.rs:45-55 (`NOVA OS <ver> // SHELL`,
// `NOVA OS <ver> // APPS / <ID>`; the build version is elided here). Body
// labels are the app titles (map/app.rs:115-117, ship/app.rs:24-26). Footer
// hint sets verbatim: crates/nova_os/src/app.rs:15-25,
// crates/nova_os_ui/src/map/mod.rs:70-80,
// crates/nova_os_ui/src/ship/mod.rs:82-94.
interface NovaOsSurface {
    crumb: string;
    body: string;
    hints: string[];
    esc: boolean;
}
const NOVA_OS_SURFACES: Record<string, NovaOsSurface> = {
    shell: {
        crumb: "NOVA OS // SHELL",
        body: "TERMINAL SCROLLBACK",
        hints: [
            "TAB: COMPLETE",
            "ENTER: RUN",
            "UP/DN: HISTORY",
            "PGUP/PGDN: SCROLL",
            "ESC: CLOSE",
            "TYPE HELP",
        ],
        esc: false,
    },
    map: {
        crumb: "NOVA OS // APPS / MAP",
        body: "MAP / LOCAL SPACE",
        hints: [
            "WASD: MOVE",
            "Q/E: TURN",
            "R/F: TILT",
            "DRAG: LOOK",
            "WHEEL: ZOOM",
            "[ / ]: CYCLE",
            "G: GOTO",
            "T: RESET",
            "ESC: BACK",
        ],
        esc: true,
    },
    ship: {
        crumb: "NOVA OS // APPS / SHIP",
        body: "SHIP / SCHEMATIC",
        hints: [
            "Q/E: TURN",
            "R/F: TILT",
            "DRAG: LOOK",
            "WHEEL: ZOOM",
            "[ / ]: SELECT",
            "G: MATES",
            "L: RELOAD",
            "P: REPAIR",
            "B: REBIND",
            "T: RESET",
            "ESC: BACK",
        ],
        esc: true,
    },
};

const NOVA_OS_DISPATCH_LABEL: Record<NovaOsDispatch, string> = {
    print: "PRINTS TO THE SCROLLBACK",
    app: "APP TAKES THE SCREEN",
    action: "ACTS ON THE SHIP + PRINTS",
    close: "POWERS OFF",
};

function initNovaOsSurfaces(host: HTMLElement): void {
    header(
        host,
        "NOVA OS console: where each command lands",
        "Every command lands on one of three surfaces: most print into " +
            "the terminal scrollback, `map` and `ship` hand the whole " +
            "screen to an app, and the acting verbs touch the live ship " +
            "and print the result. Pick a command - the frame shows the " +
            "surface you end up on, with that surface's real header " +
            "breadcrumb and footer keys."
    );

    const keysRow = el("div", "widget__keys");
    const frame = el("div", "widget__console");
    const head = el("div", "widget__console-head");
    const crumb = el("span", undefined, NOVA_OS_SURFACES.shell.crumb);
    // The amber [ ESC ] close control is visible only while an app owns the
    // screen (crates/nova_os_ui/src/terminal/spawn.rs:372-393).
    const escControl = el("span", "widget__console-esc", "[ ESC ]");
    head.appendChild(crumb);
    head.appendChild(escControl);
    const body = el("div", "widget__console-body");
    const echo = el("div", "widget__console-echo");
    const surfaceLabel = el("div", "widget__console-surface");
    body.appendChild(echo);
    body.appendChild(surfaceLabel);
    const foot = el("div", "widget__console-foot");
    frame.appendChild(head);
    frame.appendChild(body);
    frame.appendChild(foot);
    const readout = el("p", "widget__readout");

    const show = (command: NovaOsCommand | undefined): void => {
        const surface =
            NOVA_OS_SURFACES[command?.app ?? "shell"] ?? NOVA_OS_SURFACES.shell;
        const off = command?.dispatch === "close";
        frame.classList.toggle("is-off", off);
        crumb.textContent = surface.crumb;
        escControl.style.visibility = surface.esc ? "visible" : "hidden";
        // Submits echo as `nova> <line>` (crates/nova_os/src/terminal/edit.rs:18,112-115).
        echo.textContent = command ? `nova> ${command.name}` : "nova> _";
        surfaceLabel.textContent = off ? "" : surface.body;
        foot.textContent = surface.hints.join("   ");
        readout.textContent = command
            ? `${NOVA_OS_DISPATCH_LABEL[command.dispatch]} - ${command.outcome}`
            : "Pick a command above. The footer row always lists the " +
              "keys of the active surface.";
    };

    let active: HTMLButtonElement | undefined;
    for (const command of NOVA_OS_COMMANDS) {
        const btn = el("button", "widget__btn", command.name);
        btn.type = "button";
        btn.title = command.summary;
        btn.setAttribute("aria-pressed", "false");
        btn.addEventListener("click", () => {
            if (active) {
                active.classList.remove("is-on");
                active.setAttribute("aria-pressed", "false");
            }
            active = btn;
            btn.classList.add("is-on");
            btn.setAttribute("aria-pressed", "true");
            show(command);
        });
        keysRow.appendChild(btn);
    }

    const note = el(
        "p",
        "widget__note",
        "The real header carries the build version (NOVA OS v... // " +
            "SHELL) and a live SHIP / LINK / FPS status line. Esc backs " +
            "out one level - an app returns to the prompt, the prompt " +
            "powers the monitor off - and Shift+Esc powers off from " +
            "anywhere."
    );

    host.appendChild(keysRow);
    host.appendChild(frame);
    host.appendChild(readout);
    host.appendChild(note);
    show(undefined);
}

// ---- ammo-rhythm ----------------------------------------------------------

interface AmmoWeapon extends AmmoRule {
    name: string;
    unit: string;
    short: string;
}

const AMMO_WEAPONS: AmmoWeapon[] = [
    {
        name: "PDC turret",
        unit: "rounds",
        short: "rd",
        capacity: PDC_CAPACITY,
        rate: PDC_FIRE_RATE,
        delay: PDC_RELOAD_DELAY,
        amount: PDC_RELOAD_AMOUNT,
    },
    {
        name: "torpedo bay",
        unit: "torpedoes",
        short: "tp",
        capacity: BAY_CAPACITY,
        rate: BAY_FIRE_RATE,
        delay: BAY_RELOAD_DELAY,
        amount: BAY_RELOAD_AMOUNT,
    },
];

// A magazine as a RATE LIMIT rather than a budget: hold a trigger pattern
// against the reload rule and watch the level. The point the plot makes that
// prose cannot is that the batch is all-or-nothing on a whole quiet interval,
// so a pause one tick short of the delay returns absolutely nothing.
function initAmmoRhythm(host: HTMLElement): void {
    header(
        host,
        "Trigger discipline: what a magazine is worth",
        "A weapon is never left with nothing - it refills. What it imposes " +
            "is a RHYTHM: a batch only lands after a whole quiet interval, " +
            "and every shot that lands restarts that interval. Set a burst " +
            "and a pause and read what the weapon actually holds."
    );

    const X0 = 46;
    const X1 = 546;
    const Y0 = 176;
    const Y1 = 14;

    const svg = svgEl("svg", {
        viewBox: "0 0 560 200",
        role: "img",
        "aria-label":
            "Ammunition remaining over time under a repeating burst and " +
            "pause, against the weapon's capacity and its sustained rate.",
    });
    const bands = svgEl("g", {});
    svg.appendChild(bands);
    for (const frac of [0, 0.5, 1]) {
        const y = Y0 - frac * (Y0 - Y1);
        svg.appendChild(
            svgEl("line", {
                x1: String(X0),
                y1: String(y),
                x2: String(X1),
                y2: String(y),
                class: "widget-mark--grid",
            })
        );
    }
    const capLabel = svgEl(
        "text",
        {
            x: String(X0 - 6),
            y: String(Y1 + 3),
            "text-anchor": "end",
            class: "widget-mark--axis",
        },
        ""
    );
    svg.appendChild(capLabel);
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(X0 - 6),
                y: String(Y0 + 3),
                "text-anchor": "end",
                class: "widget-mark--axis",
            },
            "0"
        )
    );
    // The dry line is the fault threshold, so it wears the gate colour and
    // ships with a label rather than relying on the red alone. Bottom LEFT:
    // every trace opens at capacity, so that corner is the one the curve is
    // guaranteed to be nowhere near.
    const dryLabel = svgEl(
        "text",
        {
            x: String(X0 + 4),
            y: String(Y0 - 6),
            "text-anchor": "start",
            class: "widget-mark--label-gate",
        },
        ""
    );
    svg.appendChild(dryLabel);
    const level = svgEl("path", { d: "", class: "widget-mark--now" });
    svg.appendChild(level);
    const spanLabel = svgEl(
        "text",
        {
            x: String(X1),
            y: String(Y0 + 16),
            "text-anchor": "end",
            class: "widget-mark--axis",
        },
        ""
    );
    svg.appendChild(spanLabel);
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stats");
    const sustainedStat = stat(stats, "sustained");
    const emptyStat = stat(stats, "held trigger empties in");
    const refillStat = stat(stats, "quiet from empty to full");
    const floorStat = stat(stats, "this pattern settles at");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const w = AMMO_WEAPONS[Number(weaponControl.input.value)];
        const burst = Number(burstControl.input.value);
        const quiet = Number(quietControl.input.value);
        const span = clamp(3 * (burst + quiet), 24, 96);
        const trace = ammoTrace(w, burst, quiet, span, span / 900);

        const x = (t: number): number => X0 + (t / span) * (X1 - X0);
        const y = (r: number): number => Y0 - (r / w.capacity) * (Y0 - Y1);
        level.setAttribute(
            "d",
            `M${trace
                .map((s) => `${x(s.t).toFixed(1)},${y(s.rounds).toFixed(1)}`)
                .join(" L")}`
        );
        // One shaded block per firing stretch, so the burst reads off the
        // glass without a legend.
        bands.replaceChildren();
        const cycle = burst + quiet;
        if (cycle > 0 && burst > 0)
            for (let start = 0; start < span; start += cycle)
                bands.appendChild(
                    svgEl("rect", {
                        x: String(x(start)),
                        y: String(Y1),
                        width: String(
                            x(Math.min(start + burst, span)) - x(start)
                        ),
                        height: String(Y0 - Y1),
                        class: "widget-mark--band",
                    })
                );
        capLabel.textContent = `${w.capacity}`;
        spanLabel.textContent = `${span.toFixed(0)} s`;

        const tail = trace.filter((s) => s.t >= span - Math.max(cycle, 1e-6));
        const floor = Math.min(...tail.map((s) => s.rounds));
        const ranDry = trace.some((s) => s.firing && s.rounds <= 0.001);
        dryLabel.textContent = ranDry ? "ran dry" : "";
        sustainedStat.textContent = `${sustainedRate(w).toFixed(2)} ${w.unit}/s`;
        emptyStat.textContent = `${(w.capacity / w.rate).toFixed(2)} s`;
        refillStat.textContent = `${refillSecs(w).toFixed(0)} s`;
        floorStat.textContent = `${floor.toFixed(0)} ${w.short}`;

        const batches = Math.floor(quiet / w.delay);
        readout.classList.remove("is-fault", "is-warn");
        if (batches === 0) {
            readout.classList.add("is-fault");
            readout.textContent =
                `A ${quiet.toFixed(2)}-second pause returns NOTHING. The ` +
                `batch is worth ${w.amount} ${w.unit} or nothing at all, ` +
                `and it needs the full ${w.delay.toFixed(0)} quiet seconds ` +
                "to land - a pause a tick short of that is the same as no " +
                "pause. This pattern is spending a magazine it is not " +
                "refilling.";
            return;
        }
        const spent = Math.min(w.rate * burst, w.capacity);
        const back = batches * w.amount;
        if (back >= spent) {
            readout.textContent =
                `${quiet.toFixed(2)} quiet seconds buy ${batches} batch` +
                `${batches === 1 ? "" : "es"} - ${back} ${w.unit} against ` +
                `the ${spent.toFixed(0)} the burst spends. The weapon holds ` +
                "this pattern forever; the magazine is never the thing you " +
                "run out of.";
        } else {
            readout.classList.add("is-warn");
            readout.textContent =
                `The burst spends ${spent.toFixed(0)} ${w.unit} and the ` +
                `pause returns ${back}, so this pattern loses ` +
                `${(spent - back).toFixed(0)} a cycle. It works until the ` +
                `weapon reaches its floor, and from there you are firing at ` +
                `the refill rate whatever the trigger is doing.`;
        }
    };

    const weaponControl = control(
        "Weapon",
        0,
        AMMO_WEAPONS.length - 1,
        1,
        0,
        (v) => AMMO_WEAPONS[v].name,
        update
    );
    const burstControl = control(
        "Burst",
        0.25,
        10,
        0.25,
        3,
        (v) => `${v.toFixed(2)} s`,
        update
    );
    const quietControl = control(
        "Pause",
        0,
        20,
        0.25,
        3,
        (v) => `${v.toFixed(2)} s`,
        update
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(weaponControl.row);
    controls.appendChild(burstControl.row);
    controls.appendChild(quietControl.row);

    const note = el(
        "p",
        "widget__note",
        "Sustained is the rate a weapon holds forever by firing each batch " +
            "the moment it lands. An EMPTY trigger pull does not restart " +
            "the interval, so a dry weapon reloads while you are still " +
            "holding the trigger down - the shaded stretches keep running " +
            "the clock once the level hits zero."
    );

    host.appendChild(controls);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    update();
}

// ---- turret-arc -----------------------------------------------------------

// What one mount can and cannot bear on. Traverse is unbounded, so the whole
// blind volume is the cap under the keel - and the reader can see that it is
// the ship's own hull in the way, not an authored arc.
function initTurretArc(host: HTMLElement): void {
    header(
        host,
        "Where a mount can bear",
        `A turret swings all the way round, but its barrel stops ` +
            `${Math.abs(TURRET_DEPRESSION_DEG)} degrees below level - it ` +
            "cannot depress back through its own ship. Put a target " +
            "somewhere and see whether this mount is one of the ones that " +
            "answers."
    );

    const CX = 280;
    const CY = 152;
    const R = 126;
    const rad = (deg: number): number => (deg * Math.PI) / 180;
    const pt = (deg: number, r: number): [number, number] => [
        CX + r * Math.cos(rad(deg)),
        CY - r * Math.sin(rad(deg)),
    ];
    // Screen y runs down, so an increasing elevation sweeps counter-clockwise
    // on the glass: sweep-flag 0.
    const sector = (a0: number, a1: number, r: number): string => {
        const [x0, y0] = pt(a0, r);
        const [x1, y1] = pt(a1, r);
        const large = Math.abs(a1 - a0) > 180 ? 1 : 0;
        return `M${CX} ${CY} L${x0.toFixed(1)} ${y0.toFixed(1)} A${r} ${r} 0 ${large} 0 ${x1.toFixed(1)} ${y1.toFixed(1)} Z`;
    };

    const svg = svgEl("svg", {
        viewBox: "0 0 560 300",
        role: "img",
        "aria-label":
            "A turret seen from the side: the elevation band it covers " +
            "sweeps from ten degrees below level up over the top and down " +
            "the far side, and the remaining cone under the mount is blind " +
            "because the ship's own hull is there.",
    });
    // Covered first, blind over it, so the wedge edge reads as a cut.
    svg.appendChild(
        svgEl("path", {
            d: sector(TURRET_DEPRESSION_DEG, 180 - TURRET_DEPRESSION_DEG, R),
            class: "widget-mark--cone",
        })
    );
    svg.appendChild(
        svgEl("path", {
            d: sector(
                180 - TURRET_DEPRESSION_DEG,
                360 + TURRET_DEPRESSION_DEG,
                R
            ),
            class: "widget-mark--band",
        })
    );
    for (const side of [1, -1]) {
        const [fx, fy] = pt(
            side > 0 ? TURRET_DEPRESSION_DEG : 180 - TURRET_DEPRESSION_DEG,
            R
        );
        svg.appendChild(
            svgEl("line", {
                x1: String(CX),
                y1: String(CY),
                x2: fx.toFixed(1),
                y2: fy.toFixed(1),
                class: "widget-mark--gate",
            })
        );
    }
    // The hull the mount is bolted to, filling the wedge it cannot shoot into.
    svg.appendChild(
        svgEl("rect", {
            x: String(CX - 96),
            y: String(CY + 6),
            width: "192",
            height: "34",
            rx: "3",
            class: "widget-mark--shadow-stroke",
        })
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(CX),
                y: String(CY + 27),
                "text-anchor": "middle",
                class: "widget-mark--word",
            },
            "OWN HULL"
        )
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(CX),
                y: String(CY + 68),
                "text-anchor": "middle",
                class: "widget-mark--label-gate",
            },
            "blind - no mount covers this"
        )
    );
    for (const [deg, label] of [
        [TURRET_ELEVATION_DEG, "+90 straight up"],
        [0, "0 level"],
    ] as [number, string][]) {
        const [lx, ly] = pt(deg, R + 8);
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(deg === 0 ? lx + 4 : lx),
                    y: String(deg === 0 ? ly + 3 : ly - 4),
                    "text-anchor": deg === 0 ? "start" : "middle",
                    class: "widget-mark--axis",
                },
                label
            )
        );
    }
    // Named off the PORT end of the floor and pushed into the left margin: the
    // starboard end of the same line runs straight through the hull block, and
    // two red words over that block is exactly where the reader stops reading.
    const floorLabel = pt(180 - TURRET_DEPRESSION_DEG, R + 10);
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(floorLabel[0].toFixed(1)),
                y: String((floorLabel[1] + 14).toFixed(1)),
                "text-anchor": "end",
                class: "widget-mark--label-gate",
            },
            `${TURRET_DEPRESSION_DEG} depression floor`
        )
    );
    const ray = svgEl("line", {
        x1: String(CX),
        y1: String(CY),
        class: "widget-mark--now",
    });
    svg.appendChild(ray);
    const blip = svgEl("circle", { r: "5", class: "widget-mark--blip" });
    svg.appendChild(blip);
    svg.appendChild(
        svgEl("circle", {
            cx: String(CX),
            cy: String(CY),
            r: "6",
            class: "widget-mark--ship",
        })
    );
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stats");
    const bearsStat = stat(stats, "this mount");
    const slewStat = stat(stats, "swing takes");
    const heldStat = stat(stats, "rounds not fired while slewing");
    const skyStat = stat(stats, "one mount covers");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const elevation = Number(elevControl.input.value);
        const traverse = Number(traverseControl.input.value);
        const bears = turretBears(elevation);
        // Traverse is drawn as foreshortening: a target swung round behind
        // the mount reads as one closer to the scope centre, which keeps the
        // single elevation plane honest about what decides the bearing.
        const reach = R * (0.34 + 0.66 * Math.cos(rad(traverse / 2)));
        const [tx, ty] = pt(elevation, reach);
        ray.setAttribute("x2", tx.toFixed(1));
        ray.setAttribute("y2", ty.toFixed(1));
        ray.setAttribute(
            "class",
            bears ? "widget-mark--now" : "widget-mark--det"
        );
        blip.setAttribute("cx", tx.toFixed(1));
        blip.setAttribute("cy", ty.toFixed(1));

        const slew = turretSlewSecs(traverse, elevation);
        bearsStat.textContent = bears ? "bears" : "cannot bear";
        slewStat.textContent = `${slew.toFixed(2)} s`;
        heldStat.textContent = `${Math.round(slew * PDC_FIRE_RATE)}`;
        skyStat.textContent = `${(turretSkyFraction() * 100).toFixed(1)}% of the sky`;

        readout.classList.remove("is-fault", "is-warn");
        if (!bears) {
            readout.classList.add("is-fault");
            readout.textContent =
                `${elevation} degrees is under the depression floor, so this ` +
                "mount holds and contributes nothing - however far round it " +
                "swings, the barrel would be pointing back through the hull. " +
                "The mounts on the other side of the ship take this one; " +
                "that is why a torpedo run under the keel meets less fire " +
                "than one across the beam.";
            return;
        }
        readout.textContent =
            `The mount can put its barrel there, and takes ${slew.toFixed(2)} ` +
            `seconds to do it - ${Math.round(slew * PDC_FIRE_RATE)} rounds it ` +
            "does not fire, because a gun shoots only while the barrel is " +
            `already ON the aim point (within ${FIRE_GATE_DEG.toFixed(2)} ` +
            "degrees). Wrenching the ship around mid-burst stops the guns " +
            "until the barrels catch up.";
    };

    const elevControl = control(
        "Target elevation",
        -90,
        90,
        1,
        24,
        (v) => `${v} deg`,
        update
    );
    const traverseControl = control(
        "Traverse to swing",
        0,
        180,
        5,
        60,
        (v) => `${v} deg`,
        update
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(elevControl.row);
    controls.appendChild(traverseControl.row);

    const note = el(
        "p",
        "widget__note",
        `Both hinges turn at ${TURRET_SLEW_DEG_S} deg/s at the same time, so ` +
            "a swing costs the larger of the two angles rather than their " +
            "sum; the timings above assume the barrel starts level and on " +
            `the old bearing. Reach is ${meters(PDC_REACH)} - muzzle speed times ` +
            "how long a round lives, not an authored range."
    );

    host.appendChild(controls);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    update();
}

// ---- torpedo-run ----------------------------------------------------------

interface TorpedoType {
    name: string;
    weaveAngle: number;
    cruise: number; // authored cap, meters per second
    lineSpeed: number; // measured speed along the direct line, m/s
    runSecs: number; // measured time over the 3 km run-in
    rounds: number; // rounds one stock PDC spends to stop it
    killedAt: number; // where that PDC finally kills it, meters out
    lane: number;
}

// The run-in the harness measured: 3 km, one stock PDC. Every number in the
// table is the module header of
// crates/nova_authoring/src/base_content/sections/ordnance.rs:13-21 - a
// measurement, not a derivation, so nothing here is interpolated. That table
// is still quoted in world units, so each figure below is its number stated
// in meters (300 u = 3 000 m, 35 u/s = 350 m/s, and so on).
const TORPEDO_RUN_IN = 3000;
const TORPEDO_TYPES: TorpedoType[] = [
    {
        name: "LANCE",
        weaveAngle: 0,
        cruise: LANCE_TORPEDO_CRUISE,
        lineSpeed: 313,
        runSecs: 9.1,
        rounds: ROUNDS_PER_LANCE_TORPEDO,
        killedAt: 1140,
        lane: 86,
    },
    {
        name: "SERPENT",
        weaveAngle: SERPENT_WEAVE_ANGLE,
        cruise: SERPENT_CRUISE,
        lineSpeed: 291,
        runSecs: 9.78,
        rounds: ROUNDS_PER_SERPENT,
        killedAt: 400,
        lane: 178,
    },
];

// Both torpedoes race one run-in under one clock, because the whole trade is
// a comparison: the Lance arrives first, the Serpent survives longer.
function initTorpedoRun(host: HTMLElement): void {
    header(
        host,
        "The run-in: Lance against Serpent",
        "Same warhead, same rack, same blast - the only difference is how " +
            "they cross the last three kilometres. Press PLAY and watch " +
            "both go in. Arm the defender and watch where each one dies."
    );

    const X0 = 48;
    const X1 = 512;
    const dx = (d: number): number => X1 - (d / TORPEDO_RUN_IN) * (X1 - X0);
    // The one measured amplitude: at the shipped 0.44 rad and 1.4 rad/s the
    // torpedo swings 111 m off the direct line (the harness measured 11.1 u).
    // The drawn swing scales off that anchor with sin(angle), never off a
    // number nobody measured.
    const MEASURED_SWING = 111;
    const pxPerMeter = (X1 - X0) / TORPEDO_RUN_IN;
    const swingPx = (angle: number): number =>
        (MEASURED_SWING * pxPerMeter * Math.sin(angle)) /
        Math.sin(SERPENT_WEAVE_ANGLE);

    const svg = svgEl("svg", {
        viewBox: "0 0 560 236",
        role: "img",
        "aria-label":
            "Two torpedo run-ins over three kilometres: a Lance flying " +
            "the bare intercept and a Serpent corkscrewing off it, the " +
            "corkscrew tapering to nothing in the terminal band, with the " +
            "point where one point-defense mount kills each of them.",
    });
    // The band where the weave tapers out, drawn once behind both lanes.
    svg.appendChild(
        svgEl("rect", {
            x: String(dx(TORPEDO_BLAST_RADIUS * WEAVE_FULL_RADII)),
            y: "40",
            width: String(
                dx(TORPEDO_BLAST_RADIUS * WEAVE_ZERO_RADII) -
                    dx(TORPEDO_BLAST_RADIUS * WEAVE_FULL_RADII)
            ),
            height: "168",
            class: "widget-mark--band",
        })
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(dx(TORPEDO_BLAST_RADIUS * WEAVE_FULL_RADII) + 4),
                y: "34",
                class: "widget-mark--axis",
            },
            "weave tapers out"
        )
    );
    svg.appendChild(
        svgEl("rect", {
            x: String(X1),
            y: "70",
            width: "26",
            height: "124",
            rx: "3",
            class: "widget-mark--shadow-stroke",
        })
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(X1 + 13),
                y: "210",
                "text-anchor": "middle",
                class: "widget-mark--axis",
            },
            "target"
        )
    );
    for (const d of [3000, 2000, 1000, 0])
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(dx(d)),
                    y: "228",
                    "text-anchor": "middle",
                    class: "widget-mark--axis",
                },
                d === TORPEDO_RUN_IN ? `${meters(d)} out` : numText(d / 1000, 1)
            )
        );

    interface Lane {
        path: SVGPathElement;
        dart: SVGPathElement;
        kill: SVGCircleElement;
        killLabel: SVGTextElement;
        state: SVGTextElement;
    }
    const lanes: Lane[] = TORPEDO_TYPES.map((type) => {
        svg.appendChild(
            svgEl("line", {
                x1: String(X0),
                y1: String(type.lane),
                x2: String(X1),
                y2: String(type.lane),
                class: "widget-mark--ray",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(X0),
                    y: String(type.lane - 38),
                    class: "widget-mark--word",
                },
                type.name
            )
        );
        // Clear of the weave: the corkscrew reaches its full amplitude at the
        // launch end, which is exactly where these labels sit.
        const detail = svgEl(
            "text",
            {
                x: String(X0),
                y: String(type.lane - 26),
                class: "widget-mark--detail",
            },
            `${metersPerSec(type.cruise)} cap, weave ${type.weaveAngle.toFixed(2)} rad`
        );
        svg.appendChild(detail);
        const path = svgEl("path", { d: "", class: "widget-mark--now" });
        svg.appendChild(path);
        const kill = svgEl("circle", {
            cx: String(dx(type.killedAt)),
            cy: String(type.lane),
            r: "7",
            class: "widget-mark--impact",
            visibility: "hidden",
        });
        svg.appendChild(kill);
        const killLabel = svgEl(
            "text",
            {
                x: String(dx(type.killedAt)),
                y: String(type.lane + 22),
                "text-anchor": "middle",
                class: "widget-mark--label-gate",
                visibility: "hidden",
            },
            `killed ${meters(type.killedAt)} out`
        );
        svg.appendChild(killLabel);
        const dart = svgEl("path", { d: "", class: "widget-mark--dart" });
        svg.appendChild(dart);
        // Rides the torpedo rather than the lane end, so STOPPED names the
        // place it actually died instead of the target it never reached.
        const state = svgEl(
            "text",
            {
                x: String(X1),
                y: String(type.lane),
                "text-anchor": "middle",
                class: "widget-mark--word",
            },
            ""
        );
        svg.appendChild(state);
        return { path, dart, kill, killLabel, state };
    });
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stats");
    const arrivalStat = stat(stats, "arrives");
    const runnerStat = stat(
        stats,
        `closes on a ${PLAYER_SPEED_CAP} m/s runner`
    );
    const costStat = stat(stats, "rounds one PDC spends");
    const readout = el("p", "widget__readout");

    let defended = false;
    // Position on the direct line at scope time `t`, and the lateral offset
    // the corkscrew has put on it there. The run is anchored on the arrival
    // time measured for THIS run-in, not on the cruise cap or on the speed
    // along the line: those are separate measurements of separate questions,
    // and dividing one by the other lands the torpedo somewhere the harness
    // never put it. `lineSpeed` is only ever read for the runner stat, which
    // is the question it was measured to answer.
    const distanceAt = (type: TorpedoType, t: number): number =>
        Math.max(0, TORPEDO_RUN_IN * (1 - t / type.runSecs));
    const offsetAt = (type: TorpedoType, t: number): number =>
        swingPx(type.weaveAngle) *
        Math.sin(SERPENT_WEAVE_RATE * t) *
        weaveFade(distanceAt(type, t), TORPEDO_BLAST_RADIUS);

    const duration = (): number =>
        Math.max(...TORPEDO_TYPES.map((t) => t.runSecs));

    const render = (t: number): void => {
        TORPEDO_TYPES.forEach((type, index) => {
            const lane = lanes[index];
            const dead = defended && distanceAt(type, t) <= type.killedAt;
            const stopT = dead
                ? type.runSecs * (1 - type.killedAt / TORPEDO_RUN_IN)
                : t;
            const points: string[] = [];
            for (let s = 0; s <= stopT; s += 0.04)
                points.push(
                    `${dx(distanceAt(type, s)).toFixed(1)},${(type.lane + offsetAt(type, s)).toFixed(1)}`
                );
            points.push(
                `${dx(distanceAt(type, stopT)).toFixed(1)},${(type.lane + offsetAt(type, stopT)).toFixed(1)}`
            );
            lane.path.setAttribute("d", `M${points.join(" L")}`);
            const hx = dx(distanceAt(type, stopT));
            const hy = type.lane + offsetAt(type, stopT);
            lane.dart.setAttribute(
                "d",
                dead
                    ? ""
                    : `M${(hx + 7).toFixed(1)} ${hy.toFixed(1)} L${(hx - 5).toFixed(1)} ${(hy - 4).toFixed(1)} L${(hx - 5).toFixed(1)} ${(hy + 4).toFixed(1)} Z`
            );
            lane.kill.setAttribute("visibility", dead ? "visible" : "hidden");
            lane.killLabel.setAttribute(
                "visibility",
                defended ? "visible" : "hidden"
            );
            lane.state.textContent = dead
                ? "STOPPED"
                : distanceAt(type, t) <= 0
                  ? "HIT"
                  : "";
            // Centred on the torpedo but held clear of the frame edges, so a
            // word never runs out of the viewBox at either end of the run.
            lane.state.setAttribute("x", String(clamp(hx, X0 + 34, X1 - 20)));
            lane.state.setAttribute("y", String((hy - 14).toFixed(1)));
        });
    };

    const transport = makeTransport(duration, render);

    const setDefended = (on: boolean): void => {
        defended = on;
        defenderKey.classList.toggle("is-on", on);
        defenderKey.setAttribute("aria-pressed", String(on));
        arrivalStat.textContent = TORPEDO_TYPES.map(
            (t) => `${t.name.toLowerCase()} ${t.runSecs.toFixed(2)} s`
        ).join(", ");
        runnerStat.textContent = TORPEDO_TYPES.map(
            (t) => `${metersPerSec(t.lineSpeed - PLAYER_SPEED_CAP)}`
        ).join(" / ");
        costStat.textContent = on
            ? TORPEDO_TYPES.map((t) => `${t.rounds}`).join(" / ")
            : "not defended";
        readout.classList.toggle("is-warn", on);
        readout.textContent = on
            ? "One stock PDC stops both, and that is the whole point: it " +
              `spends ${TORPEDO_TYPES[0].rounds} rounds on the Lance and ` +
              `kills it ${meters(TORPEDO_TYPES[0].killedAt)} out, then spends ` +
              `${TORPEDO_TYPES[1].rounds} on the Serpent and only catches ` +
              `it ${meters(TORPEDO_TYPES[1].killedAt)} out - on its own doorstep. ` +
              "Saturation is what beats point defense; one torpedo at a " +
              "time never does."
            : "Nothing shooting back, and the Lance simply wins: it holds " +
              `the faster cap and arrives ${(TORPEDO_TYPES[1].runSecs - TORPEDO_TYPES[0].runSecs).toFixed(2)} ` +
              "seconds sooner over the same 3 km. That is what the weave " +
              "costs, and it is why the type you load depends entirely on " +
              "whether the target can answer.";
        transport.seekEnd();
    };
    const defenderKey = el(
        "button",
        "widget__btn widget__btn--wide",
        "ONE STOCK PDC"
    );
    defenderKey.type = "button";
    defenderKey.addEventListener("click", () => setDefended(!defended));
    const keys = el("div", "widget__keys");
    keys.appendChild(defenderKey);

    const note = el(
        "p",
        "widget__note",
        "Arrival times, kill ranges and round counts are the harness " +
            "measurement over this exact 3 km run-in against one stock " +
            "mount - not a formula run on the cruise caps. The drawn " +
            "corkscrew is scaled from the one swing the harness measured: " +
            `${meters(MEASURED_SWING)} off the direct line at the shipped ` +
            `${SERPENT_WEAVE_ANGLE} rad and ${SERPENT_WEAVE_RATE} rad/s.`
    );

    host.appendChild(keys);
    host.appendChild(plot);
    host.appendChild(transport.row);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    setDefended(false);
}

// ---- thruster-mass --------------------------------------------------------

// Thrust is authored per drive; MASS is not authored at all. A section weighs
// exactly its own box (base_section.rs:376), so the same two drives move three
// shipped hulls at three different rates and nothing anywhere says so.
interface DriveRig {
    name: string;
    detail: string;
    parts: ShipPart[];
    drives: number;
}
const DRIVE_RIGS: DriveRig[] = [
    { name: "racer", detail: "civilian yacht", parts: RACER_PARTS, drives: 2 },
    { name: "corvette", detail: "cargoa", parts: CARGOA_PARTS, drives: 2 },
    { name: "hauler", detail: "cargob", parts: CARGOB_PARTS, drives: 2 },
];

function initThrusterMass(host: HTMLElement): void {
    header(
        host,
        "What the drive has to move",
        "Every shipped drive pushes with the same 1.0, and every hull " +
            "carries two of them. What differs is the MASS on the other " +
            "side of it - a section weighs its own box, and nothing " +
            "authors that. Bolt basic drives on and watch it out."
    );

    const EXTRA_MAX = 8;
    const ACCEL_MAX = 700; // m/s^2
    const X0 = 48;
    const X1 = 484;
    const Y0 = 196;
    const Y1 = 22;
    const x = (k: number): number => X0 + (k / EXTRA_MAX) * (X1 - X0);
    const y = (a: number): number =>
        Y0 - (clamp(a, 0, ACCEL_MAX) / ACCEL_MAX) * (Y0 - Y1);
    // A basic drive is a unit cube (base_section.rs:79-85) pushing 1.0
    // (standard.rs:650), so each one added is +1 of impulse over +1 of mass.
    // The impulse, the tick rate and the box masses are all ENGINE figures, so
    // the quotient is world units per second squared - and this is the one
    // place it crosses into the m/s^2 every reading below is in.
    const accel = (rig: DriveRig, extra: number): number =>
        (((rig.drives + extra) * THRUSTER_MAGNITUDE * FIXED_TICK_HZ) /
            (hullState(rig.parts).mass + extra)) *
        METERS_PER_UNIT;
    // What one drive carrying only itself would do: the hard ceiling every
    // curve climbs toward, in the same m/s^2.
    const DRIVE_CEILING = THRUSTER_MAGNITUDE * FIXED_TICK_HZ * METERS_PER_UNIT;

    const svg = svgEl("svg", {
        viewBox: "0 0 560 230",
        role: "img",
        "aria-label":
            "Acceleration against drives added, one curve per shipped hull. " +
            "All three climb toward the same hard ceiling, and the light " +
            "yacht starts more than twice as high as the hauler.",
    });
    for (const a of [0, 200, 400, 600]) {
        svg.appendChild(
            svgEl("line", {
                x1: String(X0),
                y1: String(y(a)),
                x2: String(X1),
                y2: String(y(a)),
                class: "widget-mark--grid",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(X0 - 5),
                    y: String(y(a) + 3),
                    "text-anchor": "end",
                    class: "widget-mark--axis",
                },
                String(a)
            )
        );
    }
    for (let k = 0; k <= EXTRA_MAX; k += 2) {
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(x(k)),
                    y: String(Y0 + 15),
                    "text-anchor": "middle",
                    class: "widget-mark--axis",
                },
                `+${k}`
            )
        );
    }
    svg.appendChild(
        svgEl(
            "text",
            { x: String(X0 - 5), y: "14", class: "widget-mark--axis" },
            "m/s^2 against basic drives added"
        )
    );
    svg.appendChild(
        svgEl("line", {
            x1: String(X0),
            y1: String(y(DRIVE_CEILING)),
            x2: String(X1),
            y2: String(y(DRIVE_CEILING)),
            class: "widget-mark--gate",
        })
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(X1),
                y: String(y(DRIVE_CEILING) - 5),
                "text-anchor": "end",
                class: "widget-mark--label-gate",
            },
            `${metersPerSec2(DRIVE_CEILING, 0)} - a hull built of nothing but drives`
        )
    );

    // The corvette and the hauler end within 30 m/s^2 of each other, which is
    // seven pixels: the end labels are pushed apart to a readable gap rather
    // than left to sit on the curve heights exactly.
    const ends = DRIVE_RIGS.map((rig, index) => ({
        index,
        endY: y(accel(rig, EXTRA_MAX)),
    })).sort((a, b) => a.endY - b.endY);
    const labelY = new Array<number>(DRIVE_RIGS.length).fill(0);
    let floor = -Infinity;
    for (const end of ends) {
        const placed = Math.max(end.endY, floor + 13);
        labelY[end.index] = placed;
        floor = placed;
    }
    const curves = DRIVE_RIGS.map((rig, index) => {
        const points: string[] = [];
        for (let k = 0; k <= EXTRA_MAX; k += 0.25) {
            points.push(`${x(k).toFixed(1)},${y(accel(rig, k)).toFixed(1)}`);
        }
        const path = svgEl("path", { d: `M${points.join(" L")}`, class: "" });
        svg.appendChild(path);
        const label = svgEl(
            "text",
            { x: String(X1 + 6), y: String(labelY[index] + 3), class: "" },
            rig.name
        );
        svg.appendChild(label);
        return { path, label };
    });
    const cursor = svgEl("circle", { r: "5", class: "widget-mark--dot-now" });
    svg.appendChild(cursor);
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stats");
    const massStat = stat(stats, "hull mass");
    const drivesStat = stat(stats, "drives");
    const accelStat = stat(stats, "acceleration");
    const gStat = stat(stats, "in G");
    const sprintStat = stat(stats, "0 to 1,000 m/s");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const rig = DRIVE_RIGS[Number(rigControl.input.value)];
        const extra = Number(extraControl.input.value);
        const mass = hullState(rig.parts).mass + extra;
        const a = accel(rig, extra);

        curves.forEach((curve, index) => {
            const lit = DRIVE_RIGS[index] === rig;
            curve.path.setAttribute(
                "class",
                lit ? "widget-mark--now" : "widget-mark--old"
            );
            curve.label.setAttribute(
                "class",
                lit ? "widget-mark--label-now" : "widget-mark--label-old"
            );
        });
        cursor.setAttribute("cx", String(x(extra)));
        cursor.setAttribute("cy", String(y(a)));

        massStat.textContent = mass.toFixed(2);
        drivesStat.textContent = String(rig.drives + extra);
        accelStat.textContent = metersPerSec2(a, 1);
        gStat.textContent = `${(a / 9.81).toFixed(1)} G`;
        sprintStat.textContent = `${(1000 / a).toFixed(1)} s`;

        const stock = DRIVE_RIGS.map((r) => accel(r, 0));
        if (extra === 0) {
            readout.textContent =
                `Stock, all three carry two drives pushing 1.0 each. The ` +
                `yacht weighs ${hullState(RACER_PARTS).mass.toFixed(2)} and ` +
                `pulls ${metersPerSec2(stock[0], 0)}; the hauler weighs ` +
                `${hullState(CARGOB_PARTS).mass.toFixed(2)} and pulls ` +
                `${metersPerSec2(stock[2], 0)}. Nothing authored that gap - it is ` +
                "the volume of the boxes each hull is built from.";
        } else {
            const gain = ((a / accel(rig, 0) - 1) * 100).toFixed(0);
            readout.textContent =
                `${extra} basic drive${extra === 1 ? "" : "s"} on the ` +
                `${rig.name}: ${metersPerSec2(a, 0)}, ${gain}% up on stock. ` +
                "Each one is a unit of mass as well as a unit of push, so " +
                `the return tapers - and no stack of them passes ` +
                `${metersPerSec2(DRIVE_CEILING, 0)}, which is what one drive would do ` +
                "carrying only itself.";
        }
    };
    const rigControl = control(
        "Hull",
        0,
        DRIVE_RIGS.length - 1,
        1,
        1,
        (v) => `${DRIVE_RIGS[v].name} (${DRIVE_RIGS[v].detail})`,
        update
    );
    const extraControl = control(
        "Basic drives added",
        0,
        EXTRA_MAX,
        1,
        0,
        (v) => `+${v}`,
        update
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(rigControl.row);
    controls.appendChild(extraControl.row);

    const note = el(
        "p",
        "widget__note",
        "Thrust is authored as an impulse per physics tick rather than a " +
            `force, and the game runs ${FIXED_TICK_HZ} of them a second, so ` +
            "a hull's acceleration is its summed magnitude times that rate " +
            "over its mass. The curves assume every drive is aimed along " +
            "the ship's nose and balanced about its centre of mass; a " +
            "lopsided set spends part of itself cancelling its own torque."
    );

    host.appendChild(controls);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    update();
}

// ---- hull-armour ----------------------------------------------------------

// Health is authored per part; the mass that carries it is NOT. So the
// catalog's health column and the cost of bolting a part on rank the shipped
// hulls in different orders, and only one of those orders is a build decision.
interface ArmourPart {
    name: string;
    health: number;
    mass: number;
}

function craftPart(parts: ShipPart[], id: string, name: string): ArmourPart {
    const part = parts.find((p) => p.id === id);
    if (!part) throw new Error(`no shipped part ${id}`);
    return {
        name,
        health: part.health,
        mass: part.size[0] * part.size[1] * part.size[2],
    };
}

// The two unit-cell hulls author no collider at all (standard.rs:311,:411), so
// each is the default unit cube - one of mass, exactly
// (base_section.rs:79-85). Their health is standard.rs:308,:408.
const CARGOA_NOSE = craftPart(CARGOA_PARTS, "nose", "CargoA // Nose");
const RACER_TAIL = craftPart(RACER_PARTS, "tail", "Racer // Tail");
const ARMOUR_PARTS: ArmourPart[] = [
    { name: "Reinforced Hull Section", health: 200, mass: 1 },
    { name: "Light Hull Section", health: 60, mass: 1 },
    craftPart(RACER_PARTS, "wing_starboard", "Racer // Wing"),
    craftPart(RACER_PARTS, "nose", "Racer // Nose"),
    RACER_TAIL,
    craftPart(CARGOA_PARTS, "pod_starboard", "CargoA // Pod"),
    CARGOA_NOSE,
    craftPart(CARGOA_PARTS, "tail", "CargoA // Tail"),
    craftPart(CARGOB_PARTS, "nose", "CargoB // Nose"),
    craftPart(CARGOB_PARTS, "tail", "CargoB // Tail"),
];

function initHullArmour(host: HTMLElement): void {
    header(
        host,
        "Armour, and what it costs to carry",
        "A hull part's mass is its own authored box and nothing else - no " +
            "part is denser than another. So the health column is not the " +
            "order you want when you are picking what to bolt on. Switch " +
            "the ranking and watch it come apart."
    );

    const rows = el("div");
    const stats = el("div", "widget__stats");
    const bestStat = stat(stats, "best per mass");
    const worstStat = stat(stats, "worst per mass");
    const spreadStat = stat(stats, "spread");
    const readout = el("p", "widget__readout");
    // Opens on the CATALOG's order, which is the order the reader arrived
    // with and the one the fallback prose states. The switch is the argument.
    let perMass = false;

    const value = (part: ArmourPart): number =>
        perMass ? part.health / part.mass : part.health;

    const update = (): void => {
        const ranked = [...ARMOUR_PARTS].sort((a, b) => value(b) - value(a));
        const top = value(ranked[0]);
        rows.replaceChildren();
        for (const part of ranked) {
            const label = el(
                "p",
                "widget__rowlabel",
                `${part.name} - ${part.health} hp, ${part.mass.toFixed(2)} mass, ` +
                    `${(part.health / part.mass).toFixed(0)} per mass`
            );
            const bar = el("div", "widget__bar");
            const fill = el("div", "widget__bar-fill");
            fill.style.width = `${((value(part) / top) * 100).toFixed(1)}%`;
            bar.appendChild(fill);
            rows.appendChild(label);
            rows.appendChild(bar);
        }

        const byMass = [...ARMOUR_PARTS].sort(
            (a, b) => b.health / b.mass - a.health / a.mass
        );
        const best = byMass[0];
        const worst = byMass[byMass.length - 1];
        bestStat.textContent = `${best.name}, ${(best.health / best.mass).toFixed(0)}`;
        worstStat.textContent = `${worst.name}, ${(worst.health / worst.mass).toFixed(0)}`;
        spreadStat.textContent = `x${(best.health / best.mass / (worst.health / worst.mass)).toFixed(1)}`;

        const cargoaNose = CARGOA_NOSE;
        const racerTail = RACER_TAIL;
        readout.textContent = perMass
            ? `Ranked by what it costs to carry, the ${racerTail.name} beats ` +
              `the ${cargoaNose.name}: ` +
              `${(racerTail.health / racerTail.mass).toFixed(0)} against ` +
              `${(cargoaNose.health / cargoaNose.mass).toFixed(0)}. The nose ` +
              `is ${cargoaNose.mass.toFixed(2)} of mass to the tail's ` +
              `${racerTail.mass.toFixed(2)}, and every bit of that mass is ` +
              "acceleration you do not get."
            : `Ranked by health alone the ${cargoaNose.name} (` +
              `${cargoaNose.health}) looks like better armour than the ` +
              `${racerTail.name} (${racerTail.health}). It is nearly three ` +
              "times the box, so per unit of mass it is barely half as good. " +
              "This is the order the catalog table gives you.";
    };

    const keys = el("div", "widget__keys");
    const buttons: HTMLButtonElement[] = [];
    for (const [label, wanted] of [
        ["BY HEALTH", false],
        ["PER MASS", true],
    ] as [string, boolean][]) {
        const btn = el("button", "widget__btn", label);
        btn.type = "button";
        btn.addEventListener("click", () => {
            perMass = wanted;
            for (const other of buttons) {
                const on = other === btn;
                other.classList.toggle("is-on", on);
                other.setAttribute("aria-pressed", String(on));
            }
            update();
        });
        buttons.push(btn);
        keys.appendChild(btn);
    }
    buttons[0].classList.add("is-on");
    buttons[0].setAttribute("aria-pressed", "true");
    buttons[1].setAttribute("aria-pressed", "false");

    const note = el(
        "p",
        "widget__note",
        "Mass is the volume of the authored collider box, exactly - the " +
            "shape the part is hit on, never its art. It is the whole of " +
            "what a hull cell costs you in a straight line; what it costs " +
            "you in a turn depends on WHERE you bolt it, because the turn " +
            "ceiling is set by the reach to the ship's furthest face."
    );

    host.appendChild(keys);
    host.appendChild(rows);
    host.appendChild(stats);
    host.appendChild(readout);
    host.appendChild(note);
    update();
}

// The release post uses four small comparisons that do not need a scope or a
// simulation clock. They still hydrate through the same fallback-first contract
// as the reference widgets above.

function initDamageLevels(host: HTMLElement): void {
    // damage_cracks.rs:102-127 - eight nearest-value buckets, pristine included.
    const buckets = 8;
    header(
        host,
        "A section wears eight damage levels",
        "Move missing health. The surface snaps to a shared level; pristine keeps the ordinary material."
    );

    const cells = el("div", "widget__stack");
    const levelCells: HTMLElement[] = [];
    for (let i = 0; i < buckets; i++) {
        const cell = sectionCell(
            String(i),
            i === 0
                ? "pristine"
                : i === buckets - 1
                  ? "burnt"
                  : `${Math.round((i / (buckets - 1)) * 100)}%`,
            ""
        );
        levelCells.push(cell);
        cells.appendChild(cell);
    }
    const stats = el("div", "widget__stats");
    const bucketStat = stat(stats, "visible level");
    const materialStat = stat(stats, "render material");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const damage = Number(damageControl.input.value) / 100;
        const bucket = Math.round(damage * (buckets - 1));
        for (const [index, cell] of levelCells.entries()) {
            cell.classList.remove("is-hit", "is-dead", "is-clear");
            if (index === bucket)
                cell.classList.add(
                    bucket === buckets - 1 ? "is-dead" : "is-hit"
                );
            else cell.classList.add("is-clear");
        }
        bucketStat.textContent = `${bucket} / ${buckets - 1}`;
        materialStat.textContent =
            bucket === 0 ? "source / no cracks" : "shared crack bucket";
        readout.textContent =
            bucket === 0
                ? "Pristine means no effect pipeline at all. The section draws with the material it already owned."
                : `This section shares level ${bucket} with every section using the same source material. Fleet size does not mint another copy.`;
    };
    const damageControl = control(
        "Health missing",
        0,
        100,
        1,
        0,
        (v) => `${v.toFixed(0)}%`,
        update
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(damageControl.row);
    host.appendChild(controls);
    host.appendChild(cells);
    host.appendChild(stats);
    host.appendChild(readout);
    update();
}

function initPointDefense(host: HTMLElement): void {
    // assignment.rs:245-303 - idle mounts prefer an unclaimed reachable threat.
    header(
        host,
        "One battery, four independent answers",
        "Switch between the old ship-wide pick and v0.11.0's per-mount assignment."
    );
    const mounts = el("div");
    const readout = el("p", "widget__readout");
    const buttons: HTMLButtonElement[] = [];
    let perMount = true;

    const update = (): void => {
        mounts.replaceChildren();
        mounts.appendChild(
            el(
                "p",
                "widget__rowlabel",
                perMount ? "v0.11.0 - per mount" : "v0.10.0 - ship pick"
            )
        );
        const row = el("div", "widget__stack");
        for (let i = 0; i < 4; i++) {
            const target = perMount ? i + 1 : 1;
            row.appendChild(
                sectionCell(
                    `MOUNT ${i + 1}`,
                    `THREAT ${target}`,
                    perMount ? "is-hit" : i === 0 ? "is-dead" : "is-clear"
                )
            );
        }
        mounts.appendChild(row);
        readout.textContent = perMount
            ? "Four mounts cover four threats. Reachability is still local: a gun with nothing in its arc returns to the primary target."
            : "Every mount inherits THREAT 1. The first torpedo is over-served while three equally reachable threats fly on.";
        readout.classList.toggle("is-fault", !perMount);
    };

    for (const [label, wanted] of [
        ["SHIP-WIDE", false],
        ["PER MOUNT", true],
    ] as [string, boolean][]) {
        const button = el("button", "widget__btn", label);
        button.type = "button";
        button.setAttribute("aria-pressed", String(wanted === perMount));
        button.classList.toggle("is-on", wanted === perMount);
        button.addEventListener("click", () => {
            perMount = wanted;
            for (const other of buttons) {
                const on = other === button;
                other.classList.toggle("is-on", on);
                other.setAttribute("aria-pressed", String(on));
            }
            update();
        });
        buttons.push(button);
    }
    const keys = el("div", "widget__keys");
    for (const button of buttons) keys.appendChild(button);
    host.appendChild(keys);
    host.appendChild(mounts);
    host.appendChild(readout);
    update();
}

function initStyleExplorer(host: HTMLElement): void {
    // styles.rs:1-83 - four authored styles and their stable ids.
    const styles = [
        ["INDUSTRIAL", "services outside", "ducts / radiators / hazard bands"],
        ["ARMOURED", "an unbroken belt", "applique / hatches / sensors"],
        ["CIVILIAN", "a finished vehicle", "windows / fairings / livery"],
        ["SALVAGE", "repair is the finish", "patches / welds / drums"],
    ] as const;
    header(
        host,
        "Four styles, four ideas about a hull",
        "Choose a kit. A style changes materials, fixtures, and where those fixtures may stand."
    );
    const keys = el("div", "widget__keys");
    const cards = el("div", "widget__stack");
    const readout = el("p", "widget__readout");
    const buttons: HTMLButtonElement[] = [];
    let selected = 0;
    const update = (): void => {
        cards.replaceChildren();
        const [name, thesis, fixtures] = styles[selected];
        cards.appendChild(sectionCell(name, thesis, "is-hit"));
        for (const fixture of fixtures.split(" / "))
            cards.appendChild(
                sectionCell(fixture.toUpperCase(), "fixture", "")
            );
        readout.textContent = `${name}: ${thesis}. Its ${fixtures.split(" / ").join(", ")} are deterministic parts of the ship, not a texture pass.`;
    };
    styles.forEach(([name], index) => {
        const button = el("button", "widget__btn", name);
        button.type = "button";
        button.classList.toggle("is-on", index === selected);
        button.setAttribute("aria-pressed", String(index === selected));
        button.addEventListener("click", () => {
            selected = index;
            for (const [i, other] of buttons.entries()) {
                other.classList.toggle("is-on", i === selected);
                other.setAttribute("aria-pressed", String(i === selected));
            }
            update();
        });
        buttons.push(button);
        keys.appendChild(button);
    });
    host.appendChild(keys);
    host.appendChild(cards);
    host.appendChild(readout);
    update();
}

function initBattlefieldLoad(host: HTMLElement): void {
    // Paired census: tasks/20260818-220812/DECISIONS.md:1172-1175.
    const oldScene = { rounds: 1000, bodies: 1035, colliders: 1046 };
    const newScene = { rounds: 400, bodies: 35, colliders: 46 };
    header(
        host,
        "A round left the physics world",
        "Compare scene totals, then isolate what one gun round adds. The fight sizes differ; the per-round invariant does not."
    );
    const rows = el("div");
    const readout = el("p", "widget__readout");
    const keys = el("div", "widget__keys");
    const buttons: HTMLButtonElement[] = [];
    let perRound = false;

    const meter = (label: string, value: number, max: number): HTMLElement => {
        const wrap = el("div");
        wrap.appendChild(el("p", "widget__rowlabel", `${label}: ${value}`));
        const bar = el("div", "widget__bar");
        const fill = el("div", "widget__bar-fill");
        fill.style.width = `${max === 0 ? 0 : (value / max) * 100}%`;
        bar.appendChild(fill);
        wrap.appendChild(bar);
        return wrap;
    };
    const update = (): void => {
        rows.replaceChildren();
        if (perRound) {
            rows.appendChild(meter("v0.10 body / round", 1, 1));
            rows.appendChild(meter("v0.11 body / round", 0, 1));
            rows.appendChild(meter("v0.10 collider / round", 1, 1));
            rows.appendChild(meter("v0.11 collider / round", 0, 1));
            readout.textContent =
                "The swept round keeps gameplay state and a render entity, but contributes zero rigid bodies and zero colliders.";
        } else {
            const max = oldScene.colliders;
            rows.appendChild(meter("v0.10 rounds", oldScene.rounds, max));
            rows.appendChild(meter("v0.10 rigid bodies", oldScene.bodies, max));
            rows.appendChild(meter("v0.10 colliders", oldScene.colliders, max));
            rows.appendChild(meter("v0.11 rounds", newScene.rounds, max));
            rows.appendChild(meter("v0.11 rigid bodies", newScene.bodies, max));
            rows.appendChild(meter("v0.11 colliders", newScene.colliders, max));
            readout.textContent =
                "Scene totals include ships and torpedoes. Shorter reach also leaves fewer rounds alive, so these totals are evidence of shape, not a universal speed ratio.";
        }
    };
    for (const [label, wanted] of [
        ["SCENE TOTALS", false],
        ["PER ROUND", true],
    ] as [string, boolean][]) {
        const button = el("button", "widget__btn", label);
        button.type = "button";
        button.classList.toggle("is-on", wanted === perRound);
        button.setAttribute("aria-pressed", String(wanted === perRound));
        button.addEventListener("click", () => {
            perRound = wanted;
            for (const other of buttons) {
                const on = other === button;
                other.classList.toggle("is-on", on);
                other.setAttribute("aria-pressed", String(on));
            }
            update();
        });
        buttons.push(button);
        keys.appendChild(button);
    }
    host.appendChild(keys);
    host.appendChild(rows);
    host.appendChild(readout);
    update();
}

// ---- v0.12.0: the cold torpedo launch ------------------------------------

// A bay ejects its torpedo inert and lights the drive `ignition_delay` seconds
// later. Damping is avian's per-substep `v /= 1 + dt*k`, which at the fixed
// step is exponential decay to within a rounding error, so the widget models
// the coast as `v(t) = v0 * exp(-k t)`.
const TORPEDO_LINEAR_DAMPING = 0.8; // torpedo_section/mod.rs:265 (1/s)
const TORPEDO_EJECT_SPEED = 80; // standard.rs:1190 (MetersPerSecond)
const TORPEDO_IGNITION_DELAY = 0.6; // torpedo_section/mod.rs:451
// The shipped warhead mesh is `nose_cone_mesh(0.16, 0.65, 0.35)` - a 0.65 body
// under a 0.35 nose, so one world unit end to end, which is 10 m
// (torpedo_section/render.rs:131-134).
const TORPEDO_BODY_LENGTH = 10;

function coastDistance(speed: number, seconds: number): number {
    const k = TORPEDO_LINEAR_DAMPING;
    return (speed / k) * (1 - Math.exp(-k * seconds));
}

function initIgnitionDelay(host: HTMLElement): void {
    header(
        host,
        "Dropped, then lit",
        "The bay kicks the torpedo clear inert and the motor catches out in the open. Move the coast and the ejection charge to see how far the warhead gets before the drive has it."
    );

    const controls = el("div", "widget__controls");
    const delay = control(
        "ignition delay",
        0,
        2,
        0.05,
        TORPEDO_IGNITION_DELAY,
        (v) => `${v.toFixed(2)} s`,
        () => update()
    );
    const charge = control(
        "ejection charge",
        10,
        200,
        5,
        TORPEDO_EJECT_SPEED,
        (v) => metersPerSec(v),
        () => update()
    );
    controls.appendChild(delay.row);
    controls.appendChild(charge.row);
    host.appendChild(controls);

    // A side elevation: the firing hull at the left, the tube's axis running
    // right, and the drive lighting wherever the coast ends.
    const W = 640;
    const H = 150;
    const X0 = 70;
    const X1 = 610;
    const AXIS = 84;
    const FULL_SCALE = 120; // meters of travel across the drawn axis
    const svg = svgEl("svg", { viewBox: `0 0 ${W} ${H}` });
    const px = (m: number): number =>
        X0 + Math.min(m / FULL_SCALE, 1) * (X1 - X0);

    svg.appendChild(
        svgEl("line", {
            x1: String(X0),
            y1: String(AXIS),
            x2: String(X1),
            y2: String(AXIS),
            class: "widget-mark--axis",
        })
    );
    for (let m = 0; m <= FULL_SCALE; m += 20) {
        svg.appendChild(
            svgEl("line", {
                x1: String(px(m)),
                y1: String(AXIS - 5),
                x2: String(px(m)),
                y2: String(AXIS + 5),
                class: "widget-mark--grid",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(px(m)),
                    y: String(AXIS + 20),
                    "text-anchor": "middle",
                    class: "widget-mark--detail",
                },
                meters(m)
            )
        );
    }
    // The hull the bay is mounted in.
    svg.appendChild(
        svgEl("rect", {
            x: String(X0 - 54),
            y: String(AXIS - 26),
            width: "48",
            height: "52",
            rx: "4",
            class: "widget-mark--ship",
        })
    );
    const coastBand = svgEl("rect", {
        y: String(AXIS - 10),
        height: "20",
        class: "widget-mark--band",
    });
    svg.appendChild(coastBand);
    const burn = svgEl("line", {
        y1: String(AXIS),
        y2: String(AXIS),
        class: "widget-mark--plume",
    });
    svg.appendChild(burn);
    const gate = svgEl("line", {
        y1: String(AXIS - 34),
        y2: String(AXIS + 34),
        class: "widget-mark--gate",
    });
    svg.appendChild(gate);
    const gateLabel = svgEl(
        "text",
        {
            y: String(AXIS - 42),
            "text-anchor": "middle",
            class: "widget-mark--label-gate",
        },
        "IGNITION"
    );
    svg.appendChild(gateLabel);
    const dart = svgEl("polygon", { class: "widget-mark--dart" });
    svg.appendChild(dart);

    const plot = el("div", "widget__plot");
    plot.appendChild(svg);
    host.appendChild(plot);

    const stats = el("div", "widget__stats");
    const coastStat = stat(stats, "coast");
    const lengthStat = stat(stats, "body lengths");
    const catchStat = stat(stats, "speed at ignition");
    host.appendChild(stats);
    const readout = el("p", "widget__readout");
    host.appendChild(readout);

    const update = (): void => {
        const seconds = Number(delay.input.value);
        const speed = Number(charge.input.value);
        const distance = coastDistance(speed, seconds);
        const catchSpeed = speed * Math.exp(-TORPEDO_LINEAR_DAMPING * seconds);
        const x = px(distance);

        coastBand.setAttribute("x", String(X0));
        coastBand.setAttribute("width", String(Math.max(0, x - X0)));
        burn.setAttribute("x1", String(x));
        burn.setAttribute("x2", String(X1));
        gate.setAttribute("x1", String(x));
        gate.setAttribute("x2", String(x));
        gateLabel.setAttribute("x", String(x));
        dart.setAttribute(
            "points",
            `${x + 14},${AXIS} ${x - 10},${AXIS - 7} ${x - 10},${AXIS + 7}`
        );

        coastStat.textContent = `${meters(distance, 1)}`;
        lengthStat.textContent = (distance / TORPEDO_BODY_LENGTH).toFixed(1);
        catchStat.textContent = `${metersPerSec(catchSpeed, 1)}`;

        readout.classList.remove("is-warn", "is-fault");
        if (seconds === 0) {
            readout.classList.add("is-warn");
            readout.textContent =
                "Zero is the old behaviour: the drive is already lit when the torpedo leaves the tube, so there is no inert phase to see or to shoot at.";
        } else if (distance < TORPEDO_BODY_LENGTH * 1.5) {
            readout.classList.add("is-fault");
            readout.textContent =
                "The warhead is still alongside its own hull when the motor catches. The coast has to clear the ship to read as a launch rather than as a muzzle.";
        } else {
            readout.textContent =
                "Through the shaded coast the torpedo is inert: it cannot be shot down and it cannot damage anything. The drive picks it up already moving.";
        }
    };
    update();
}

// ---- v0.12.0: the sleeping OnUpdate pulse --------------------------------

function initPulseSleep(host: HTMLElement): void {
    header(
        host,
        "A pulse that sleeps",
        "An OnUpdate handler used to evaluate its filters on every frame. It now wakes only when a variable it reads is written, or when the clock crosses a threshold it compares against."
    );

    const controls = el("div", "widget__controls");
    const writes = control(
        "variable writes",
        0,
        60,
        1,
        1,
        (v) => `${v} / s`,
        () => update()
    );
    const gates = control(
        "clock thresholds",
        0,
        6,
        1,
        0,
        (v) => `${v} / s`,
        () => update()
    );
    controls.appendChild(writes.row);
    controls.appendChild(gates.row);
    host.appendChild(controls);

    const keys = el("div", "widget__keys");
    const buttons: HTMLButtonElement[] = [];
    let sleeping = true;

    // One second of frames at 60 fps, one tick each.
    const FRAMES = 60;
    const W = 640;
    const H = 56;
    const svg = svgEl("svg", { viewBox: `0 0 ${W} ${H}` });
    svg.appendChild(
        svgEl("rect", {
            x: "1",
            y: "1",
            width: String(W - 2),
            height: String(H - 2),
            rx: "3",
            class: "widget-mark--barframe",
        })
    );
    const ticks: SVGRectElement[] = [];
    const slotWidth = (W - 12) / FRAMES;
    for (let i = 0; i < FRAMES; i += 1) {
        const tick = svgEl("rect", {
            x: String(6 + i * slotWidth + 1),
            y: "8",
            width: String(Math.max(2, slotWidth - 2)),
            height: String(H - 16),
            class: "widget-mark--old",
        });
        ticks.push(tick);
        svg.appendChild(tick);
    }
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stats");
    const wokeStat = stat(stats, "frames woken");
    const shareStat = stat(stats, "share of the second");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const perSecond = sleeping
            ? Math.min(
                  FRAMES,
                  Number(writes.input.value) + Number(gates.input.value)
              )
            : FRAMES;
        // Spread the wakes evenly through the second so the strip reads as a
        // rate rather than as a burst at the front.
        const lit = new Set<number>();
        for (let i = 0; i < perSecond; i += 1) {
            lit.add(Math.floor((i * FRAMES) / Math.max(1, perSecond)));
        }
        ticks.forEach((tick, i) => {
            tick.setAttribute(
                "class",
                lit.has(i) ? "widget-mark--now" : "widget-mark--old"
            );
        });
        wokeStat.textContent = `${lit.size} / ${FRAMES}`;
        shareStat.textContent = `${((lit.size / FRAMES) * 100).toFixed(1)}%`;
        readout.classList.remove("is-warn");
        if (!sleeping) {
            readout.classList.add("is-warn");
            readout.textContent =
                "Every frame: the handler's filters run 60 times a second whether or not anything they read has changed. This is what the pulse used to cost.";
        } else if (lit.size === 0) {
            readout.textContent =
                "Nothing writes and no threshold is crossed, so the handler costs nothing at all this second. It is still armed.";
        } else {
            readout.textContent =
                "The handler evaluates only on the frames something it reads actually moved. A value-gated scenario lands near the bottom of this range.";
        }
    };

    for (const [label, wanted] of [
        ["SLEEPING PULSE", true],
        ["EVERY FRAME", false],
    ] as [string, boolean][]) {
        const button = el("button", "widget__btn", label);
        button.type = "button";
        button.classList.toggle("is-on", wanted === sleeping);
        button.setAttribute("aria-pressed", String(wanted === sleeping));
        button.addEventListener("click", () => {
            sleeping = wanted;
            for (const other of buttons) {
                const on = other === button;
                other.classList.toggle("is-on", on);
                other.setAttribute("aria-pressed", String(on));
            }
            update();
        });
        buttons.push(button);
        keys.appendChild(button);
    }
    host.appendChild(keys);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
    update();
}

// ---- v0.12.0: the transient-light budget ---------------------------------

// GraphicsBudget::transient_lights per quality tier
// (crates/nova_gameplay/src/settings.rs:251,260,273).
const TRANSIENT_LIGHT_BUDGET: [string, number][] = [
    ["HIGH", 6],
    ["MEDIUM", 3],
    ["LOW", 0],
];

function initTransientLights(host: HTMLElement): void {
    header(
        host,
        "Six lights, then no more",
        "A detonation lights the hulls around it, and brief lights are capped per graphics tier. Ask for more than the tier allows and the extra requests are refused outright rather than dimmed."
    );

    const keys = el("div", "widget__keys");
    const buttons: HTMLButtonElement[] = [];
    let tier = 0;

    const controls = el("div", "widget__controls");
    const salvo = control(
        "detonations at once",
        1,
        12,
        1,
        8,
        (v) => String(v),
        () => update()
    );
    controls.appendChild(salvo.row);

    const stack = el("div", "widget__stack");
    const stats = el("div", "widget__stats");
    const capStat = stat(stats, "tier cap");
    const litStat = stat(stats, "lit");
    const refusedStat = stat(stats, "refused");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const [name, cap] = TRANSIENT_LIGHT_BUDGET[tier];
        const asked = Number(salvo.input.value);
        const lit = Math.min(asked, cap);
        stack.replaceChildren();
        for (let i = 0; i < asked; i += 1) {
            const granted = i < lit;
            stack.appendChild(
                sectionCell(
                    granted ? "LIT" : "OFF",
                    granted ? " burning" : " refused",
                    granted ? "is-live" : "is-dead"
                )
            );
        }
        capStat.textContent = cap === 0 ? "none" : String(cap);
        litStat.textContent = String(lit);
        refusedStat.textContent = String(asked - lit);
        readout.classList.remove("is-warn", "is-fault");
        if (cap === 0) {
            readout.classList.add("is-fault");
            readout.textContent = `${name} draws no brief lights at all. The fireball still reads; the hulls around it simply do not catch it.`;
        } else if (asked > cap) {
            readout.classList.add("is-warn");
            readout.textContent = `${name} burns ${cap} and refuses the rest. A light that faded in as the tier filled would make the tier visible, and the point of a budget is that you do not see it.`;
        } else {
            readout.textContent = `${name} has room for all ${asked}. The cap only bites once a salvo lands together.`;
        }
    };

    for (const [index, [label]] of TRANSIENT_LIGHT_BUDGET.entries()) {
        const button = el("button", "widget__btn", label);
        button.type = "button";
        button.classList.toggle("is-on", index === tier);
        button.setAttribute("aria-pressed", String(index === tier));
        button.addEventListener("click", () => {
            tier = index;
            for (const [other, otherButton] of buttons.entries()) {
                const on = other === index;
                otherButton.classList.toggle("is-on", on);
                otherButton.setAttribute("aria-pressed", String(on));
            }
            update();
        });
        buttons.push(button);
        keys.appendChild(button);
    }
    host.appendChild(keys);
    host.appendChild(controls);
    host.appendChild(stack);
    host.appendChild(stats);
    host.appendChild(readout);
    update();
}

// ---- activation -----------------------------------------------------------

// ---- lance-corridor -------------------------------------------------------

// The corridor scope: one lance shot into a block of hull cells, replayed in
// scope time. A slice through the bore on the left shows the tip, the sphere
// trailing it and the cylinder it has swept; the entry face on the right
// counts the layers each column lost. data-radius, data-hp, data-width,
// data-height and data-depth seed the faders.
function initLanceCorridor(host: HTMLElement): void {
    // The faders and the lattice count CELLS; `data-radius` seeds cells too.
    const seedRadius = Number(host.dataset.radius);
    const radius0 =
        Number.isFinite(seedRadius) && seedRadius >= 0
            ? seedRadius
            : LANCE_RAKE_RADIUS_CELLS;
    const hp0 = numAttr(host, "hp", REINFORCED_HULL_HP);
    const width0 = numAttr(host, "width", 5);
    const height0 = numAttr(host, "height", 5);
    const depth0 = numAttr(host, "depth", 4);
    const power0 = numAttr(host, "power", LANCE_SLUG_POWER);
    header(
        host,
        "Corridor scope: one railgun shot into a hull block",
        "A block of hull cells on the build lattice - a cell is 10 m on a " +
            `side - shot down its centre at ${metersPerSec(LANCE_SLUG_SPEED)}. ` +
            "The tip cuts the bore column; the " +
            "sphere trailing it widens that cut into a corridor, and every " +
            `cell in the corridor takes the flat ${LANCE_SLUG_DAMAGE} and ` +
            `pays a third of its max health out of the one ${LANCE_SLUG_POWER}` +
            "-point budget. Play the tape, then drag the radius: wider is " +
            "not more, it is elsewhere."
    );

    // Scope geometry. The slice is drawn cell for cell; the face grid is the
    // same cell size, so a column's count reads against the cut beside it.
    const S = 22;
    const CY = 150;
    const X0 = 110;
    const Z_START = -1.6;
    const TIP_SPEED = 1.5;
    const FX = 460;
    const FY = 150;
    const BAR = { x: 16, y: 14, w: 140, h: 8 };
    const HP_NAMES: Record<number, string> = {
        60: "light hull",
        100: "controller / bay",
        130: "PDC turret",
        180: "railgun",
        200: "reinforced hull",
        480: "vector drive",
    };

    const svg = svgEl("svg", {
        viewBox: "0 0 560 270",
        role: "img",
        "aria-label":
            "Corridor scope: a railgun slug entering a block of hull cells in " +
            "side profile with a sphere trailing its tip, cells lighting as " +
            "the corridor takes them, and the block's entry face beside it " +
            "counting the layers each column lost.",
    });
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    // Static furniture: the budget bar and the two panel labels.
    svg.appendChild(
        svgEl("rect", {
            x: String(BAR.x),
            y: String(BAR.y),
            width: String(BAR.w),
            height: String(BAR.h),
            rx: "2",
            class: "widget-mark--barframe",
        })
    );
    const barFill = svgEl("rect", {
        x: String(BAR.x),
        y: String(BAR.y),
        width: String(BAR.w),
        height: String(BAR.h),
        rx: "2",
        class: "widget-mark--barfill",
    });
    svg.appendChild(barFill);
    const barText = svgEl(
        "text",
        {
            x: String(BAR.x),
            y: String(BAR.y + 22),
            class: "widget-mark--detail",
        },
        ""
    );
    svg.appendChild(barText);
    svg.appendChild(
        svgEl(
            "text",
            { x: String(X0), y: "58", class: "widget-mark--word" },
            "SLICE THROUGH THE BORE"
        )
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(FX),
                y: "58",
                "text-anchor": "middle",
                class: "widget-mark--word",
            },
            "ENTRY FACE"
        )
    );
    const verdict = svgEl(
        "text",
        {
            x: String(X0),
            y: "262",
            class: "widget-mark--detail",
        },
        ""
    );
    svg.appendChild(verdict);

    // Everything the block's shape decides is rebuilt per parameter change.
    const dynamic = svgEl("g", {});
    svg.appendChild(dynamic);

    interface SliceCell {
        cell: CorridorCell;
        rect: SVGRectElement;
    }
    interface FaceCell {
        x: number;
        y: number;
        rect: SVGRectElement;
        cut: SVGRectElement;
        count: SVGTextElement;
    }
    let result = lanceCorridor(radius0, hp0, width0, height0, depth0, power0);
    let radius = radius0;
    let hp = hp0;
    let depth = depth0;
    let power = power0;
    let slice: SliceCell[] = [];
    let face: FaceCell[] = [];
    let profileText: SVGTextElement[] = [];
    let corridor = svgEl("rect", {});
    let sphere = svgEl("circle", {});
    let dart = svgEl("rect", {});
    const reachT = (cell: CorridorCell): number =>
        (cell.reach - Z_START) / TIP_SPEED;
    const duration = (): number =>
        (depth + radius + 0.8 - Z_START) / TIP_SPEED + 0.3;

    const rebuild = (): void => {
        radius = Number(radiusControl.input.value);
        hp = Number(hpControl.input.value);
        const width = Number(widthControl.input.value);
        const height = Number(heightControl.input.value);
        depth = Number(depthControl.input.value);
        power = Number(powerControl.input.value);
        result = lanceCorridor(radius, hp, width, height, depth, power);
        dynamic.replaceChildren();
        slice = [];
        face = [];
        profileText = [];
        // The cylinder the sphere has swept so far, drawn under the cells.
        corridor = svgEl("rect", {
            y: String(CY - radius * S),
            height: String(2 * radius * S),
            class: "widget-mark--corridor",
        });
        dynamic.appendChild(corridor);
        // The bore line, so the axis reads even before the slug arrives.
        const sight = svgEl("line", {
            x1: "0",
            y1: String(CY),
            x2: String(X0 + (depth + 3) * S),
            y2: String(CY),
            class: "widget-mark--ray",
        });
        dynamic.appendChild(sight);
        // The slice: the row of cells through the bore, one column per
        // layer, `x` running down the screen.
        for (const cell of result.cells) {
            if (cell.y !== 0) continue;
            const rect = svgEl("rect", {
                x: String(X0 + cell.layer * S + 1),
                y: String(CY + cell.x * S - S / 2 + 1),
                width: String(S - 2),
                height: String(S - 2),
                rx: "2",
                class: "widget-mark--section",
            });
            dynamic.appendChild(rect);
            slice.push({ cell, rect });
        }
        // Per-layer counts under the slice.
        const bottom = CY + (Math.floor(width / 2) + 0.5) * S + 16;
        for (let layer = 0; layer < depth; layer++) {
            const text = svgEl(
                "text",
                {
                    x: String(X0 + layer * S + S / 2),
                    y: String(bottom),
                    "text-anchor": "middle",
                    class: "widget-mark--count",
                },
                "0"
            );
            dynamic.appendChild(text);
            profileText.push(text);
        }
        // The entry face: every column once, counting the layers it lost.
        for (const cell of result.cells) {
            if (cell.layer !== 0) continue;
            const px = FX + cell.x * S - S / 2 + 1;
            const py = FY - cell.y * S - S / 2 + 1;
            const rect = svgEl("rect", {
                x: String(px),
                y: String(py),
                width: String(S - 2),
                height: String(S - 2),
                rx: "2",
                class: "widget-mark--section",
            });
            const cut = svgEl("rect", {
                x: String(px),
                y: String(py),
                width: String(S - 2),
                height: String(S - 2),
                rx: "2",
                class: "widget-mark--cut",
                "fill-opacity": "0",
            });
            const count = svgEl(
                "text",
                {
                    x: String(px + S / 2 - 1),
                    y: String(py + S / 2 + 3),
                    class: "widget-mark--count",
                },
                ""
            );
            dynamic.appendChild(rect);
            dynamic.appendChild(cut);
            dynamic.appendChild(count);
            face.push({ x: cell.x, y: cell.y, rect, cut, count });
        }
        // The rake's footprint on the face, and the bore itself.
        if (radius > 0) {
            dynamic.appendChild(
                svgEl("circle", {
                    cx: String(FX),
                    cy: String(FY),
                    r: String(radius * S),
                    class: "widget-mark--rake",
                })
            );
        }
        dynamic.appendChild(
            svgEl("circle", {
                cx: String(FX),
                cy: String(FY),
                r: "2.5",
                class: "widget-mark--bore",
            })
        );
        // The sphere and the slug, over everything.
        sphere = svgEl("circle", {
            cy: String(CY),
            r: String(radius * S),
            class: "widget-mark--rake",
            visibility: radius > 0 ? "visible" : "hidden",
        });
        dynamic.appendChild(sphere);
        dart = svgEl("rect", {
            y: String(CY - 1.5),
            width: "18",
            height: "3",
            rx: "1.5",
            class: "widget-mark--dart",
        });
        dynamic.appendChild(dart);

        taken.textContent = `${result.taken} cells`;
        spent.textContent = `${Math.round(result.spent)} of ${power}`;
        removed.textContent = `${Math.round(result.removed)} hp`;
        perCycle.textContent = `${(result.removed / LANCE_CYCLE_SECS).toFixed(0)} hp/s`;
        profile.textContent = result.profile.join(" / ");
    };

    const setState = (
        rect: SVGRectElement,
        state: string,
        flash: boolean
    ): void => {
        rect.setAttribute(
            "class",
            `widget-mark--section${state ? ` ${state}` : ""}${flash ? " is-flash" : ""}`
        );
    };
    const renderFrame = (t: number): void => {
        const z = Z_START + t * TIP_SPEED;
        const tipX = X0 + z * S;
        const centreX = tipX - radius * S;
        const gone = z > depth + radius + 1.2;
        dart.setAttribute("x", String(tipX - 18));
        dart.setAttribute("visibility", gone ? "hidden" : "visible");
        sphere.setAttribute("cx", String(centreX));
        sphere.setAttribute(
            "visibility",
            radius > 0 && !gone ? "visible" : "hidden"
        );
        const sweptFrom = X0 + (Z_START - radius) * S;
        corridor.setAttribute("x", String(sweptFrom));
        corridor.setAttribute(
            "width",
            String(
                Math.max(0, Math.min(centreX, X0 + (depth + 2) * S) - sweptFrom)
            )
        );
        const dead = LANCE_SLUG_DAMAGE >= hp;
        for (const { cell, rect } of slice) {
            const at = reachT(cell);
            if (cell.charged && t >= at) {
                setState(rect, dead ? "is-dead" : "is-hit", t - at < 0.18);
            } else if (!cell.charged && cell.reach !== Infinity && t >= at) {
                setState(rect, "is-spared", false);
            } else {
                setState(rect, "", false);
            }
        }
        // Counts so far: per layer for the slice's footer, per column for the
        // face, and the budget the charged crossings have spent.
        const perLayer = new Array<number>(depth).fill(0);
        const perColumn = new Map<string, number>();
        let charged = 0;
        for (const cell of result.cells) {
            if (!cell.charged || t < reachT(cell)) continue;
            charged += 1;
            perLayer[cell.layer] += 1;
            const key = `${cell.x},${cell.y}`;
            perColumn.set(key, (perColumn.get(key) ?? 0) + 1);
        }
        perLayer.forEach((n, layer) => {
            profileText[layer].textContent = String(n);
        });
        for (const column of face) {
            const n = perColumn.get(`${column.x},${column.y}`) ?? 0;
            column.count.textContent = n > 0 ? String(n) : "";
            column.cut.setAttribute(
                "fill-opacity",
                (n > 0 ? 0.18 + 0.62 * (n / depth) : 0).toFixed(2)
            );
            column.cut.setAttribute(
                "class",
                `widget-mark--cut${dead ? "" : " is-hit"}`
            );
        }
        const left = Math.max(0, power - charged * result.cost);
        barFill.setAttribute("width", String((left / power) * BAR.w));
        barText.textContent = `power ${Math.round(left)}`;
        const done = t >= duration() - 0.05;
        verdict.textContent = !done
            ? ""
            : result.taken <
                result.cells.filter((c) => c.reach !== Infinity).length
              ? "POWER SPENT - the corridor stops here"
              : "CLEAN THROUGH - power to spare";
    };

    const transport = makeTransport(duration, renderFrame);
    const onParam = (): void => {
        rebuild();
        transport.seekEnd();
    };
    const radiusControl = control(
        "Rake radius",
        0,
        4,
        0.5,
        radius0,
        (v) =>
            engineMeters(v) +
            (v === 0
                ? " (needle)"
                : v === LANCE_RAKE_RADIUS_CELLS
                  ? " (shipped)"
                  : ""),
        onParam
    );
    const hpControl = control(
        "Cell health",
        60,
        480,
        10,
        hp0,
        (v) => `${v} hp${HP_NAMES[v] ? ` (${HP_NAMES[v]})` : ""}`,
        onParam
    );
    const widthControl = control(
        "Block width",
        1,
        7,
        2,
        width0,
        (v) => `${v} across`,
        onParam
    );
    const heightControl = control(
        "Block height",
        1,
        7,
        2,
        height0,
        (v) => `${v} tall`,
        onParam
    );
    const depthControl = control(
        "Block depth",
        1,
        8,
        1,
        depth0,
        (v) => `${v} deep`,
        onParam
    );
    const powerControl = control(
        "Slug power",
        300,
        4800,
        100,
        power0,
        (v) => `${v}${v === LANCE_SLUG_POWER ? " (shipped)" : ""}`,
        onParam
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(radiusControl.row);
    controls.appendChild(powerControl.row);
    controls.appendChild(hpControl.row);
    controls.appendChild(widthControl.row);
    controls.appendChild(heightControl.row);
    controls.appendChild(depthControl.row);

    const stats = el("div", "widget__stats");
    const taken = stat(stats, "cells taken");
    const spent = stat(stats, "power spent");
    const removed = stat(stats, "hull removed");
    const perCycle = stat(stats, `per ${LANCE_CYCLE_SECS} s cycle`);
    const profileRow = el("div", "widget__stats");
    const profile = stat(profileRow, "corridor profile, entry face first");
    const note = el(
        "p",
        "widget__note",
        "The block is the range the game measures this on: the shipped " +
            `${meters(LANCE_RAKE_RADIUS)} rake against a 5 x 5 x 4 wall of ` +
            `${REINFORCED_HULL_HP} hp cells took 28 cells as 9 / 9 / 9 / 1, ` +
            "and the scope replays that exact walk. For scale, a kinetic " +
            `PDC sustains 40 rounds/s x ${KINETIC_PDC_BULLET_DAMAGE} = ` +
            `${Math.round(40 * KINETIC_PDC_BULLET_DAMAGE)} hp/s at ` +
            "1,000 m/s closing, and only at a ninth of the reach."
    );

    host.appendChild(controls);
    host.appendChild(transport.row);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(profileRow);
    host.appendChild(note);
    rebuild();
    if (reducedMotion()) transport.seekEnd();
    else transport.play();
}

// ---- weapon-reach ---------------------------------------------------------

// The engagement ladder: the three weapon families' reaches on one range
// axis with a target cursor, and what each of them can do about a target
// that far out. data-range seeds the cursor; the default is a hostile still
// burning in, past the guns and inside the slug.
function initWeaponReach(host: HTMLElement): void {
    const range0 = numAttr(host, "range", 6000);
    header(
        host,
        "Engagement ladder: who reaches whom",
        "Three weapons on one range axis. Drag the target out and read " +
            "which of them can still touch it, how long the shot takes to " +
            "arrive, and what the other side can do about it while it does."
    );

    const AXIS_MAX = 34000; // meters
    const X0 = 96;
    const X1 = 540;
    const AXIS_Y = 156;
    const px = (m: number): number => X0 + (m / AXIS_MAX) * (X1 - X0);
    const LANES = [
        { name: "PDC", y: 40, reach: PDC_REACH, ext: 0 },
        { name: "RAILGUN", y: 78, reach: LANCE_REACH, ext: 0 },
        {
            name: "TORPEDO",
            y: 116,
            reach: SERPENT_REACH,
            ext: LANCE_TORPEDO_REACH,
        },
    ];

    const svg = svgEl("svg", {
        viewBox: "0 0 560 180",
        role: "img",
        "aria-label":
            "Engagement ladder: three horizontal reach bands on one range " +
            "axis - the PDC's short one, the railgun's long one and the " +
            "torpedoes' longer still - with a cursor at the chosen target " +
            "range showing which of them can touch it.",
    });
    for (const m of [0, 5000, 10000, 15000, 20000, 25000, 30000]) {
        svg.appendChild(
            svgEl("line", {
                x1: String(px(m)),
                y1: "28",
                x2: String(px(m)),
                y2: String(AXIS_Y),
                class: "widget-mark--grid",
            })
        );
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(px(m)),
                    y: String(AXIS_Y + 14),
                    "text-anchor": "middle",
                    class: "widget-mark--axis",
                },
                m === 0 ? "0 km" : numText(m / 1000, 0)
            )
        );
    }
    interface LaneMarks {
        band: SVGRectElement;
        dot: SVGCircleElement;
        reach: number;
    }
    const lanes: LaneMarks[] = [];
    for (const lane of LANES) {
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(X0 - 10),
                    y: String(lane.y + 15),
                    "text-anchor": "end",
                    class: "widget-mark--word",
                },
                lane.name
            )
        );
        const band = svgEl("rect", {
            x: String(px(0)),
            y: String(lane.y),
            width: String(px(lane.reach) - px(0)),
            height: "22",
            rx: "2",
            class: "widget-mark--reach",
        });
        svg.appendChild(band);
        if (lane.ext > lane.reach) {
            svg.appendChild(
                svgEl("rect", {
                    x: String(px(lane.reach)),
                    y: String(lane.y),
                    width: String(px(lane.ext) - px(lane.reach)),
                    height: "22",
                    rx: "2",
                    class: "widget-mark--reach is-ext",
                })
            );
        }
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(px(lane.ext || lane.reach) + 6),
                    y: String(lane.y + 15),
                    class: "widget-mark--detail",
                },
                `${meters(lane.ext || lane.reach)}`
            )
        );
        if (lane.ext) {
            svg.appendChild(
                svgEl(
                    "text",
                    {
                        x: String(px(lane.ext)),
                        y: String(lane.y + 33),
                        "text-anchor": "end",
                        class: "widget-mark--detail",
                    },
                    `Serpent ${meters(lane.reach)}, Lance ${meters(lane.ext)}`
                )
            );
        }
        const dot = svgEl("circle", {
            cy: String(lane.y + 11),
            r: "4",
            class: "widget-mark--dot-now",
        });
        svg.appendChild(dot);
        lanes.push({ band, dot, reach: lane.reach });
    }
    const cursor = svgEl("line", {
        y1: "24",
        y2: String(AXIS_Y),
        class: "widget-mark--cursor",
    });
    svg.appendChild(cursor);
    const contact = svgEl("path", {
        class: "widget-mark--contact",
    });
    svg.appendChild(contact);
    const cursorText = svgEl(
        "text",
        { y: "18", "text-anchor": "middle", class: "widget-mark--label-now" },
        ""
    );
    svg.appendChild(cursorText);
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stack");
    const rows: {
        name: HTMLElement;
        flight: HTMLElement;
        answer: HTMLElement;
    }[] = [];
    const ANSWERS = [
        `nothing: a round cannot be shot down. ${PDC_FIRE_RATE}/s cyclic, ` +
            `${Math.round(sustainedRate({ capacity: PDC_CAPACITY, rate: PDC_FIRE_RATE, delay: PDC_RELOAD_DELAY, amount: PDC_RELOAD_AMOUNT }))}/s sustained`,
        `nothing in flight: the ${LANCE_CHARGE_SECONDS} s charge is the ` +
            `only tell. One corridor per ${LANCE_CYCLE_SECS} s`,
        `point defense: about ${ROUNDS_PER_SERPENT} rounds a Serpent, ` +
            `${ROUNDS_PER_LANCE_TORPEDO} a Lance. Six in the rack`,
    ];
    for (const lane of LANES) {
        const row = el("div", "widget__cell");
        const name = el("b", undefined, lane.name);
        const flight = el("span", "widget__value", "");
        const answer = el("span", undefined, "");
        row.appendChild(name);
        row.appendChild(document.createTextNode(" "));
        row.appendChild(flight);
        row.appendChild(document.createTextNode(" - "));
        row.appendChild(answer);
        stats.appendChild(row);
        rows.push({ name, flight, answer });
    }

    const render = (): void => {
        const range = Number(rangeControl.input.value);
        const ladder = reachLadder(range);
        const x = px(range);
        cursor.setAttribute("x1", String(x));
        cursor.setAttribute("x2", String(x));
        contact.setAttribute(
            "d",
            `M${x - 6},${AXIS_Y + 1} L${x + 6},${AXIS_Y + 1} L${x},${AXIS_Y - 7} Z`
        );
        cursorText.setAttribute("x", String(x));
        cursorText.textContent = `target ${meters(range)}`;
        const flight = (secs: number): string =>
            secs === Infinity
                ? "out of reach"
                : secs < 10
                  ? `${secs.toFixed(2)} s of flight`
                  : `${secs.toFixed(0)} s of flight`;
        // PDC and lance rungs, then the two torpedo types on one row.
        lanes.forEach((lane, i) => {
            const live = range <= lane.reach;
            lane.band.setAttribute(
                "class",
                `widget-mark--reach${live ? " is-live" : ""}`
            );
            lane.dot.setAttribute("cx", String(x));
            lane.dot.setAttribute(
                "class",
                live ? "widget-mark--dot-now" : "widget-mark--dot-old"
            );
            rows[i].answer.textContent = ANSWERS[i];
        });
        rows[0].flight.textContent = flight(ladder[0].flightSecs);
        rows[1].flight.textContent = flight(ladder[1].flightSecs);
        rows[2].flight.textContent =
            ladder[2].flightSecs === Infinity &&
            ladder[3].flightSecs === Infinity
                ? "out of reach"
                : `${flight(ladder[3].flightSecs).replace(" of flight", "")} Lance, ` +
                  `${flight(ladder[2].flightSecs).replace(" of flight", "")} Serpent`;
    };
    const rangeControl = control(
        "Target range",
        500,
        34000,
        500,
        range0,
        (v) => meters(v),
        render
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(rangeControl.row);
    const note = el(
        "p",
        "widget__note",
        "Reach is never an authored number: it is muzzle speed times how " +
            "long the round lives, and a torpedo's is the speed it settles " +
            "at along the line over the bay's lifetime. Enemy gunships " +
            "close to about 1 km and fight there, inside everyone's " +
            "guns; between a gun's 2 km and the slug's 18 km only the " +
            "railgun and a torpedo reach, and the slug gets there in " +
            "about a second where the torpedo takes tens of them."
    );
    host.appendChild(controls);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(note);
    render();
}

const WIDGETS: Record<string, (host: HTMLElement) => void> = {
    "aim-decay": initAimDecay,
    "round-travel": initRoundTravel,
    "blast-layers": initBlastLayers,
    "ammo-rhythm": initAmmoRhythm,
    "turret-arc": initTurretArc,
    "torpedo-run": initTorpedoRun,
    "lance-corridor": initLanceCorridor,
    "weapon-reach": initWeaponReach,
    "controller-arm": initControllerArm,
    "controller-margin": initControllerMargin,
    "thruster-mass": initThrusterMass,
    "hull-armour": initHullArmour,
    "damage-levels": initDamageLevels,
    "point-defense": initPointDefense,
    "style-explorer": initStyleExplorer,
    "battlefield-load": initBattlefieldLoad,
    "gravity-well": initGravityWell,
    "dominant-well": initDominantWell,
    "goto-verb": initGotoVerb,
    "lock-sweep": initLockSweep,
    "relation-matrix": initRelationMatrix,
    "hud-context": initHudContext,
    "nova-os-surfaces": initNovaOsSurfaces,
    "ignition-delay": initIgnitionDelay,
    "pulse-sleep": initPulseSleep,
    "transient-lights": initTransientLights,
};

// Hydrate every declared widget on the page. The static fallback content is
// dropped only once a registered initializer is about to replace it, so an
// unknown key (or a future widget on an old bundle) degrades to the prose.
export function initWidgets(root: ParentNode = document): void {
    root.querySelectorAll<HTMLElement>("[data-widget]").forEach((host) => {
        const key = host.dataset.widget ?? "";
        const init = WIDGETS[key];
        if (!init) return;
        host.replaceChildren();
        host.classList.add("widget");
        init(host);
    });
}
