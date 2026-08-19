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

// ---- constants verified against the game source ---------------------------

// Turret aim servo (crates/nova_ship/src/sections/turret_section/aim.rs).
// The servo closes `1 - exp(-rate * dt)` of the aim error per frame - the
// same fraction per unit TIME at any frame rate (aim.rs:312). The rate is
// derived from the old flat 0.35-of-the-error-per-frame gain at 60 fps
// (aim.rs:254-258), which is the "old servo" curve shown for contrast.
const AIM_CORRECTION_RATE = 25.847; // aim.rs:259
const OLD_PER_FRAME_GAIN = 0.35; // aim.rs:255 (historical, documented)

// The fire gate: a round fired `e` radians off misses laterally by
// `range * sin(e)`, so the widest usable error is hull half-beam over range,
// 1.6 u / 100 u = 0.016 rad = 0.92 deg (aim.rs:19,24,47).
const FIRE_GATE_RAD = 1.6 / 100;
const FIRE_GATE_DEG = FIRE_GATE_RAD * (180 / Math.PI);

// Damage travel rules (crates/nova_gameplay/src/damage.rs).
const REFERENCE_CLOSING_SPEED = 100; // damage.rs:156 (PDC muzzle speed)
const KINETIC_DAMAGE_FLOOR = 0.25; // damage.rs:162
const KINETIC_DAMAGE_CEILING = 2.0; // damage.rs:168
const PIERCE_POWER_FLOOR = 0.5; // damage.rs:172
const PIERCE_POWER_CEILING = 3.0; // damage.rs:177
const PIERCE_BASE_POWER = 300; // damage.rs:188
const MAX_PIERCE_LAYERS = 6; // damage.rs:196
const EXPLOSIVE_SECTION_TRANSMISSION = 0.65; // damage.rs:394
// Blast free pressure falls off linearly to zero at the radius
// (damage.rs:435-442); each destroyed structural layer transmits 65%, a
// surviving layer stops the wave (damage.rs:445-447; ray walk 494-551).

// Flight computer stacking (crates/nova_ship/src/sections/controller_section.rs).
const STACK_AUTHORITY_LIMIT = 2.0; // controller_section.rs:239
const STACK_PRECISION_LIMIT = 1.5; // controller_section.rs:246

// Gravity wells (crates/nova_gameplay/src/gravity.rs). Mass (`mu`) is the ONLY
// authored gravity quantity: both the pull `a = mu / r^2` and the reach (the
// SOI, where that pull decays to the cutoff) fall out of it (gravity.rs:22-28).
const SOI_CUTOFF_ACCEL = 0.25; // gravity.rs:202
const GRAVITY_FADE_FRACTION = 0.15; // gravity.rs:203
const GRAVITY_SURFACE_MARGIN = 1.0; // gravity.rs:204
const WELL_SWITCH_HYSTERESIS = 1.1; // gravity.rs:205
// ORBIT's trusted band (crates/nova_ship/src/flight/state.rs).
const ORBIT_CLEARANCE_FACTOR = 1.5; // state.rs:385
const ORBIT_BAND_SAFETY = 0.9; // state.rs:386
// Shipped fixture: the Shakedown Run planetoid (crates/nova_authoring/src/
// base_content/scenarios/nova_protocol/shakedown/mod.rs).
const SHAKEDOWN_PLANETOID_MU = 27000; // shakedown/mod.rs:85
const ANCHOR_ROCK_MU = 45000; // scenarios/nova_protocol/final_tally.rs:183

// Radar locking (crates/nova_ship/src/input/targeting/).
const RADAR_TAP_SECS = 0.25; // gesture.rs:18
const TARGETING_CONE_HALF_ANGLE_DEG = 18.0; // radar.rs:20
const LOCK_DWELL_BASE = 0.6; // state.rs:72
const LOCK_DWELL_RANGE_FACTOR = 1.5; // state.rs:73
const LOCK_DWELL_REFERENCE_RANGE = 2000; // state.rs:74
const LOCK_DWELL_MIN = 0.25; // state.rs:75
const LOCK_DWELL_MAX = 2.5; // state.rs:76
const COMBAT_DECAY_SECS = 30; // contacts.rs:24

// GOTO flight controller (crates/nova_ship/src/flight/state.rs defaults;
// the speed-envelope and flip rules are ported from flight/guidance.rs).
const ARRIVAL_STANDOFF = 50; // state.rs:360
const DECEL_MARGIN = 0.85; // state.rs:359
const MIN_APPROACH_SPEED = 1.5; // state.rs:363
const ARRIVAL_SPOOL_PAD = 0.5; // state.rs:370
const STOP_SPEED_EPSILON = 0.2; // state.rs:362
const TURN_RATE_SCALE = 0.9; // state.rs:367
const TURN_RATE_MIN_DEG = 10; // state.rs:368
const TURN_RATE_MAX_DEG = 240; // state.rs:369
const RCS_ACCEL = 1.5; // state.rs:392
const RCS_SPEED_CAP = 2.0; // state.rs:389
const CONTROLLER_ANGULAR_ACCEL = 0.5; // rad/s^2, standard.rs:359
// Display policy: 1 world unit reads as 10 m on the HUD
// (crates/nova_ui/src/units.rs:13). Widgets keep raw u like the game code.
const METRES_PER_UNIT = 10; // units.rs:13

// Catalog fixtures (crates/nova_authoring/src/base_content/sections/standard.rs).
const LIGHT_HULL_HP = 60; // standard.rs:428-434 (light_hull_section)
const TORPEDO_BLAST_DAMAGE = 750; // standard.rs:658 (Serpent/Lance warhead)
const TORPEDO_BLAST_RADIUS = 30; // standard.rs:650

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

// damage.rs:226-228.
export function kineticDamageMultiplier(closingSpeed: number): number {
    return clamp(
        closingSpeed / REFERENCE_CLOSING_SPEED,
        KINETIC_DAMAGE_FLOOR,
        KINETIC_DAMAGE_CEILING
    );
}
// damage.rs:238-240.
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

// Kinetic walk (damage.rs:319-331 rule): the round spends its damage budget;
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

// Pierce walk (damage.rs:332-340 rule): full authored damage to every section
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

// Blast ray walk (damage.rs:519-551 rule) over structural layers at fixed
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

// controller_section.rs:257-259.
export function stackCurve(n: number, limit: number): number {
    return limit - (limit - 1) / Math.max(1, n);
}

// SOI from mass alone: the distance where the raw inverse-square pull decays
// to the cutoff, floored at the body radius (gravity.rs:97-99).
export function soiRadius(mu: number, bodyRadius: number): number {
    return Math.max(Math.sqrt(mu / SOI_CUTOFF_ACCEL), bodyRadius);
}

// The pull at distance r (gravity.rs:310-337): inverse square off `mu`,
// clamped at the surface margin (no singularity slingshots), smoothstepped to
// exactly zero across the outer 15% of the SOI.
export function wellAccel(
    mu: number,
    r: number,
    bodyRadius: number,
    soi: number
): number {
    if (mu <= 0 || soi <= 0 || r >= soi) return 0; // gravity.rs:318-320
    const rEff = Math.max(r, bodyRadius + GRAVITY_SURFACE_MARGIN); // :323
    const base = mu / (rEff * rEff); // gravity.rs:324
    const fadeStart = soi * (1 - GRAVITY_FADE_FRACTION); // gravity.rs:328
    let fade = 1;
    if (r > fadeStart) {
        const t = clamp((soi - r) / Math.max(soi - fadeStart, 1e-12), 0, 1);
        fade = t * t * (3 - 2 * t); // gravity.rs:329-334
    }
    return base * fade; // gravity.rs:336
}

// gravity.rs:341-346. The ORBIT verb burns to this tangentially.
export function circularOrbitSpeed(mu: number, r: number): number {
    if (mu <= 0 || r <= 0) return 0;
    return Math.sqrt(mu / r);
}

// The band ORBIT will accept a ring in (guidance.rs:218-234): clear of the
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

// Dominant-well pick over (id, pull) candidates (gravity.rs:353-376): the
// strongest wins, but an incumbent holds until a challenger clearly beats it
// - strictly more than `hysteresis x` the incumbent's pull (gravity.rs:369).
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

// Lock-on dwell before a radar lock commits (radar.rs:245-260): base time
// stretched by range up to the reference, hard-clamped either side.
export function lockDwellSecs(distance: number): number {
    const reach = clamp(distance / LOCK_DWELL_REFERENCE_RANGE, 0, 1);
    const raw = LOCK_DWELL_BASE * (1 + LOCK_DWELL_RANGE_FACTOR * reach);
    return clamp(raw, LOCK_DWELL_MIN, LOCK_DWELL_MAX);
}

// Staged clearing on a tap (gesture.rs:152-175): one lock per tap, combat
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

// The hull's average turn rate from its authored angular acceleration
// (guidance.rs:306-314): a bang-bang 180 at `alpha` averages
// `sqrt(pi * alpha) / 2`, scaled and clamped by the flight settings.
export function hullTurnRate(alpha: number): number {
    const optimum = Math.sqrt(Math.PI * Math.max(alpha, 0)) * 0.5;
    const lo = (TURN_RATE_MIN_DEG * Math.PI) / 180;
    const hi = (TURN_RATE_MAX_DEG * Math.PI) / 180;
    return clamp(optimum * TURN_RATE_SCALE, lo, hi);
}

// The arrival speed envelope (guidance.rs:20-41), gravity-free form: the
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

// The flip line (guidance.rs:86-108), gravity-free: GOTO swings retrograde
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
    x: number; // travelled, u from the start point
    v: number; // closing speed, u/s
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
    const standoff = ARRIVAL_STANDOFF + Math.max(targetRadius, 0); // autopilot.rs:296
    const park = targetDistance - standoff;
    const turnRate = hullTurnRate(CONTROLLER_ANGULAR_ACCEL);
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
            // 1.5 u/s; inside it the drive brakes for zero, and only under
            // the 2.0 u/s RCS cap do the fine jets take over
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

// ---- DOM helpers ----------------------------------------------------------

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
    name.appendChild(val);
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

    // Inset: the two closing-speed clamp curves (damage.rs:227 and :239).
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
                lastTick ? "400 u/s" : String(v)
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
        (v) => `${v} u/s`,
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

// A PPI blast scope: detonation at the origin, range rings in u, the
// structural layers as arcs on one bearing, the shock front replayed in
// scope time, and a pressure-vs-distance profile of the centre ray. The
// slider defaults are the shipped Serpent/Lance warhead.
function initBlastLayers(host: HTMLElement): void {
    const hp = numAttr(host, "hp", LIGHT_HULL_HP);
    const LAYER_DISTANCES = [10, 12, 14];
    const TARGET_DISTANCE = 16;
    // Presentation only: the game resolves a blast in one fixed tick; the
    // scope replays it at a legible sweep speed.
    const WAVE_SPEED = 12; // u of front travel per scope second
    const RING_STEP = 10;
    header(
        host,
        "Blast scope: pressure through a hull",
        `Detonation at the scope origin; three light hull layers (${hp} hp, ` +
            `the catalog value) at 10, 12 and 14 u on the bearing; the ` +
            `section you care about at ${TARGET_DISTANCE} u. Pressure falls ` +
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

    // Everything scale-dependent is rebuilt per parameter change (the u->px
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
            Math.ceil(Math.max(radius, TARGET_DISTANCE + 4) / RING_STEP) *
            RING_STEP;
        ppu = R_PX / scopeR;
        svg.replaceChildren();
        // Range rings, labeled in u along the vertical.
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
                    r === scopeR ? `${r} u` : String(r)
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
                `r ${radius} u`
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
            (LAYER_DISTANCES[2] + 4) * ppu,
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
                `${scopeR} u`
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
            const cell = sectionCell("STANDBY", `@ ${d} u: ${hp} hp`, "");
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
        det.setAttribute("opacity", front < 2.5 ? "1" : "0.45");
        blast.layers.forEach((layer, i) => {
            const d = LAYER_DISTANCES[i];
            const crossed = front >= d;
            const flash = crossed && front < d + 2.2;
            let cls = "widget-mark--layer";
            if (crossed) {
                if (layer.state === "dead") cls += " is-dead";
                else if (layer.state === "holds") cls += " is-hold";
                else cls += " is-shielded";
                if (flash) cls += " is-flash";
            }
            layerArcs[i].setAttribute("class", cls);
            const at = `@ ${d} u`;
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
            if (front < TARGET_DISTANCE + 2.2) tCls += " is-flash";
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
        frontStat.textContent = `${front.toFixed(1)} u`;
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
                    `Target section at ${TARGET_DISTANCE} u takes ` +
                    `${Math.round(blast.target)} hp, through ${destroyed} ` +
                    `destroyed layer${destroyed === 1 ? "" : "s"}.`;
            } else {
                readout.textContent =
                    `Target section at ${TARGET_DISTANCE} u takes 0 - a ` +
                    "surviving layer stopped the wave.";
                readout.classList.add("is-warn");
            }
        } else if (frontInfo.stopped) {
            readout.textContent =
                `Wave stopped at the layer holding at ${holdDist} u - ` +
                "everything behind it on the bearing is shielded.";
            readout.classList.add("is-warn");
        } else {
            readout.textContent =
                `Shock front at ${front.toFixed(1)} u - carrying ` +
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
        16,
        60,
        2,
        TORPEDO_BLAST_RADIUS,
        (v) => `${v} u`,
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

// ---- controller-stacking --------------------------------------------------

// The stacking curve: authority(n) = limit - (limit - 1) / n. Peak turn rate
// grows with the square root of the budget (the wiki table's own numbers).
function initControllerStacking(host: HTMLElement): void {
    header(
        host,
        "The stacking curve",
        "Each extra controller grows the steering budget by " +
            "limit - (limit - 1) / n toward a hard x2.00 ceiling: the " +
            "second is worth half the first, the tenth is nearly dead " +
            "weight."
    );

    const N_MAX = 10;
    const X0 = 44;
    const X1 = 550;
    const Y0 = 174;
    const Y1 = 12;
    const A_MIN = 0.9;
    const A_MAX = 2.1;
    const x = (n: number): number => X0 + ((n - 1) / (N_MAX - 1)) * (X1 - X0);
    const y = (a: number): number =>
        Y0 - ((a - A_MIN) / (A_MAX - A_MIN)) * (Y0 - Y1);

    const svg = svgEl("svg", {
        viewBox: "0 0 560 200",
        role: "img",
        "aria-label":
            "Steering budget against controller count: a curve rising from " +
            "1.0 toward a ceiling of 2.0 with sharply diminishing returns.",
    });
    for (const a of [1.0, 1.5, 2.0]) {
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
                `x${a.toFixed(1)}`
            )
        );
    }
    for (let n = 1; n <= N_MAX; n++) {
        svg.appendChild(
            svgEl(
                "text",
                {
                    x: String(x(n)),
                    y: String(Y0 + 16),
                    "text-anchor": "middle",
                    class: "widget-mark--axis",
                },
                String(n)
            )
        );
    }
    // The ceiling is a limit, not a fault: quiet dashed line, named.
    svg.appendChild(
        svgEl("line", {
            x1: String(X0),
            y1: String(y(STACK_AUTHORITY_LIMIT)),
            x2: String(X1),
            y2: String(y(STACK_AUTHORITY_LIMIT)),
            class: "widget-mark--old",
        })
    );
    svg.appendChild(
        svgEl(
            "text",
            {
                x: String(X1 - 2),
                y: String(y(STACK_AUTHORITY_LIMIT) - 6),
                "text-anchor": "end",
                class: "widget-mark--label-old",
            },
            "ceiling x2.00 - never reached"
        )
    );
    const points: string[] = [];
    for (let n = 1; n <= N_MAX; n += 0.25) {
        const a = stackCurve(n, STACK_AUTHORITY_LIMIT);
        points.push(`${x(n).toFixed(1)},${y(a).toFixed(1)}`);
    }
    svg.appendChild(
        svgEl("path", { d: `M${points.join(" L")}`, class: "widget-mark--now" })
    );
    for (let n = 1; n <= N_MAX; n++) {
        svg.appendChild(
            svgEl("circle", {
                cx: String(x(n)),
                cy: String(y(stackCurve(n, STACK_AUTHORITY_LIMIT))),
                r: "3",
                class: "widget-mark--dot-now",
            })
        );
    }
    const cursorDot = svgEl("circle", {
        r: "6",
        class: "widget-mark--dot-now",
    });
    svg.appendChild(cursorDot);
    const plot = el("div", "widget__plot");
    plot.appendChild(svg);

    const stats = el("div", "widget__stats");
    const budget = stat(stats, "steering budget");
    const peak = stat(stats, "peak turn rate");
    const precision = stat(stats, "stop-on-heading precision");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const n = Number(nControl.input.value);
        const authority = stackCurve(n, STACK_AUTHORITY_LIMIT);
        cursorDot.setAttribute("cx", String(x(n)));
        cursorDot.setAttribute("cy", String(y(authority)));
        budget.textContent = `x${authority.toFixed(2)}`;
        peak.textContent = `~x${Math.sqrt(authority).toFixed(2)}`;
        precision.textContent = `x${stackCurve(n, STACK_PRECISION_LIMIT).toFixed(2)}`;
        if (n === 1) {
            readout.textContent =
                "One controller: the baseline. The second is the biggest " +
                "single gain you can bolt on.";
        } else {
            const marginal = 1 / (n * (n - 1));
            readout.textContent =
                `Controller #${n} added +${marginal.toFixed(3)} to the ` +
                "budget. Past a pair, the honest reason to stack is " +
                "redundancy: lose one and the ship keeps steering.";
        }
    };
    const nControl = control(
        "Controllers",
        1,
        N_MAX,
        1,
        1,
        (v) => String(v),
        update
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(nControl.row);

    host.appendChild(controls);
    host.appendChild(plot);
    host.appendChild(stats);
    host.appendChild(readout);
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
    // a unit ("3.3 u/s^2") and must not clip at the viewBox edge.
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

    let mu = SHAKEDOWN_PLANETOID_MU;
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
                    a === surface ? `${a.toFixed(1)} u/s^2` : a.toFixed(1)
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
                    last ? `${r} u` : String(r)
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
                `SOI ${Math.round(soi)} u`
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
        pullStat.textContent = `${a.toFixed(2)} u/s^2`;
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
        soiStat.textContent = `${Math.round(soi)} u (${(
            (soi * METRES_PER_UNIT) /
            1000
        ).toFixed(1)} km on the HUD)`;
        const band = orbitBand(bodyR, soi);
        const inBand = band !== null && r >= band.min && r <= band.max;
        orbitStat.textContent =
            r >= soi || r <= bodyR
                ? "--"
                : `${circularOrbitSpeed(mu, r).toFixed(1)} u/s` +
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
                `${circularOrbitSpeed(mu, r).toFixed(1)} u/s tangential.`;
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
        SHAKEDOWN_PLANETOID_MU,
        (v) => `${v} u^3/s^2`,
        onParam
    );
    const radiusControl = control(
        "Body radius",
        20,
        120,
        5,
        90,
        (v) => `${v} u`,
        onParam
    );
    const rControl = control(
        "Ship distance",
        0,
        100,
        1,
        40,
        (v) => `${Math.round((v / 100) * xMax)} u`,
        update
    );
    // The distance fader spans the live scope range, so its readout re-labels
    // when mass changes the SOI.
    const relabel = (): void => {
        const val = rControl.row.querySelector(".widget__value");
        if (val)
            val.textContent = `${Math.round(
                (Number(rControl.input.value) / 100) * xMax
            )} u`;
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
        "Defaults are the Shakedown Run planetoid (mass 27000, drawn rock " +
            "~90 u): a 329 u sphere of influence. The Final Tally anchorage " +
            "rock authors 45000. The drawn rock is bigger than the body's " +
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
    const D = 600; // separation of the two well centers, u
    const RADIUS_A = 60;
    const RADIUS_B = 60;
    header(
        host,
        "Handoff scope: the dominant well",
        `Two wells ${D} u apart. Where their spheres of influence overlap ` +
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

    const muA = SHAKEDOWN_PLANETOID_MU;
    const muB = ANCHOR_ROCK_MU;
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
                    p === 0 ? "WELL A" : p === D ? "WELL B" : `${p} u`
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
        pullAStat.textContent = `${a.toFixed(2)} u/s^2`;
        pullBStat.textContent = `${b.toFixed(2)} u/s^2`;
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
        (v) => `${v} u`,
        update
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(posControl.row);

    const note = el(
        "p",
        "widget__note",
        "Well A is the Shakedown planetoid (mass 27000), well B the Final " +
            "Tally anchorage rock (mass 45000); both drawn at 60 u here so " +
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
    const TARGET_RADIUS = 20; // the widget's fixture body, u
    header(
        host,
        "Autopilot scope: one GOTO leg",
        "GOTO flies the real hull: it burns toward the lock while the " +
            "arrival envelope allows, swings retrograde one flip early, " +
            "brakes at 85% of what the drive can do, and eases the last " +
            "stretch onto a standoff 50 u off the surface with the fine " +
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
                    `${v} u/s`
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
                `${targetDistance} u`
            )
        );
        // The arrival envelope: the fastest speed the flip still recovers
        // from, drawn against distance travelled.
        const turnRate = hullTurnRate(CONTROLLER_ANGULAR_ACCEL);
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
        peakStat.textContent = `${sim.peakV.toFixed(1)} u/s`;
        flipStat.textContent = `${Math.round(sim.flipX)} u out, T+${sim.flipT.toFixed(1)}s`;
        etaStat.textContent = `${sim.duration.toFixed(1)} s`;
        standoffStat.textContent =
            `${Math.round(sim.standoff)} u off the center ` +
            `(${ARRIVAL_STANDOFF} u + the body)`;
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
                `Burning out at ${s.v.toFixed(1)} u/s - under the envelope, ` +
                "so the flip still recovers all of it.";
        } else if (s.phase === "flip") {
            readout.textContent =
                "On the flip line: engines cold while the hull swings " +
                "retrograde - the envelope already budgeted this coast.";
        } else if (s.phase === "brake") {
            readout.textContent =
                `Braking at 85% authority, riding the envelope down - the ` +
                "floor is the 1.5 u/s minimum approach.";
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
        (v) => `${v} u`,
        onParam
    );
    const accelControl = control(
        "Drive acceleration",
        2,
        20,
        1,
        8,
        (v) => `${v} u/s^2`,
        onParam
    );
    const controls = el("div", "widget__controls");
    controls.appendChild(distControl.row);
    controls.appendChild(accelControl.row);

    const note = el(
        "p",
        "widget__note",
        "Simplified to one dimension: no gravity, one forward drive group " +
            "(so the brake angle is a full 180 at the shipped controller's " +
            "0.5 rad/s^2), a stationary target. The envelope, flip line, " +
            "85% brake margin, 1.5 u/s approach floor, standoff and RCS " +
            "settle are the game's own rules."
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
        dist: number; // u
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
                `${c.dist} u - dwell ${lockDwellSecs(c.dist).toFixed(2)} s`
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
            // A tap: staged clearing (gesture.rs:152-175).
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
            "2000 u (the HUD reads that as 20 km), staged clearing. " +
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

// ---- activation -----------------------------------------------------------

const WIDGETS: Record<string, (host: HTMLElement) => void> = {
    "aim-decay": initAimDecay,
    "round-travel": initRoundTravel,
    "blast-layers": initBlastLayers,
    "controller-stacking": initControllerStacking,
    "gravity-well": initGravityWell,
    "dominant-well": initDominantWell,
    "goto-verb": initGotoVerb,
    "lock-sweep": initLockSweep,
    "relation-matrix": initRelationMatrix,
    "hud-context": initHudContext,
    "nova-os-surfaces": initNovaOsSurfaces,
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
