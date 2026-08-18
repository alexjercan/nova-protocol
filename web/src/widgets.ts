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
// (damage.rs:436-442); each destroyed structural layer transmits 65%, a
// surviving layer stops the wave (damage.rs:445-447 with the ray walk).

// Flight computer stacking (crates/nova_ship/src/sections/controller_section.rs).
const STACK_AUTHORITY_LIMIT = 2.0; // controller_section.rs:239
const STACK_PRECISION_LIMIT = 1.5; // controller_section.rs:246

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

// Kinetic walk (damage.rs:318-330 rule): the round spends its damage budget;
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

// Pierce walk (damage.rs:332-339 rule): full authored damage to every section
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

// controller_section.rs:257-259.
export function stackCurve(n: number, limit: number): number {
    return limit - (limit - 1) / Math.max(1, n);
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

function statsRow(host: HTMLElement): HTMLElement {
    const row = el("div", "widget__stats");
    host.appendChild(row);
    return row;
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

// One round of each damage type against the same stack of sections: kinetic
// spends damage, pierce spends power on thickness. data-sections / data-hp
// override the fixture.
function initRoundTravel(host: HTMLElement): void {
    const sections = numAttr(host, "sections", 5);
    const hp = numAttr(host, "hp", LIGHT_HULL_HP);
    header(
        host,
        "One round vs a section stack",
        `${sections} light hull sections, ${hp} hp each (the catalog value), ` +
            "at full health. Kinetic spends its damage and stops at the " +
            "first section it cannot destroy; Pierce deals its full damage " +
            "to every section it crosses and spends a separate " +
            `${PIERCE_BASE_POWER}-point power budget on thickness, at most ` +
            `${MAX_PIERCE_LAYERS} sections deep.`
    );

    const controls = el("div", "widget__controls");
    const kinLabel = el("p", "widget__rowlabel", "kinetic (slug)");
    const kinStack = el("div", "widget__stack");
    const kinStats = el("div", "widget__stats");
    const kinScale = stat(kinStats, "punch");
    const kinDead = stat(kinStats, "destroyed");
    const kinLeft = stat(kinStats, "carries on with");
    const prcLabel = el("p", "widget__rowlabel", "pierce (dart)");
    const prcStack = el("div", "widget__stack");
    const prcStats = el("div", "widget__stats");
    const prcCost = stat(prcStats, "crossing cost");
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

    const fill = (stack: HTMLElement, results: SectionResult[]): void => {
        stack.replaceChildren();
        for (const r of results) {
            if (r.state === "dead") {
                stack.appendChild(
                    sectionCell("DEAD", `-${Math.round(r.dealt)} hp`, "is-dead")
                );
            } else if (r.state === "hit") {
                stack.appendChild(
                    sectionCell("HIT", `-${Math.round(r.dealt)} hp`, "is-hit")
                );
            } else {
                stack.appendChild(sectionCell("CLEAR", `${hp} hp`, "is-clear"));
            }
        }
    };

    const update = (): void => {
        const speed = Number(speedControl.input.value);
        const damage = Number(damageControl.input.value);
        const kin = kineticWalk(damage, speed, sections, hp);
        fill(kinStack, kin.results);
        kinScale.textContent = `x${kineticDamageMultiplier(speed).toFixed(2)}`;
        kinDead.textContent = String(
            kin.results.filter((r) => r.state === "dead").length
        );
        kinLeft.textContent =
            kin.leftover > 0 ? `${Math.round(kin.leftover)} dmg` : "nothing";
        const prc = pierceWalk(damage, speed, sections, hp);
        fill(prcStack, prc.results);
        prcCost.textContent = `${Math.round(prc.cost)} of ${PIERCE_BASE_POWER} power`;
        prcRaked.textContent = String(prc.raked);
        prcTotal.textContent = `${Math.round(prc.raked * damage)} dmg`;
    };
    const speedControl = control(
        "Closing speed",
        10,
        400,
        10,
        REFERENCE_CLOSING_SPEED,
        (v) => `${v} u/s`,
        update
    );
    const damageControl = control(
        "Authored damage",
        20,
        300,
        10,
        100,
        (v) => `${v} hp`,
        update
    );
    controls.appendChild(speedControl.row);
    controls.appendChild(damageControl.row);

    host.appendChild(controls);
    host.appendChild(kinLabel);
    host.appendChild(kinStack);
    host.appendChild(kinStats);
    host.appendChild(prcLabel);
    host.appendChild(prcStack);
    host.appendChild(prcStats);
    host.appendChild(note);
    update();
}

// ---- blast-layers ---------------------------------------------------------

// Blast pressure walking structural layers on the centre ray. The slider
// defaults are the shipped Serpent/Lance warhead.
function initBlastLayers(host: HTMLElement): void {
    const hp = numAttr(host, "hp", LIGHT_HULL_HP);
    const LAYER_DISTANCES = [10, 12, 14];
    const TARGET_DISTANCE = 16;
    header(
        host,
        "Pressure through a hull",
        `A blast at 0 u; three light hull layers (${hp} hp, the catalog ` +
            `value) at 10, 12 and 14 u; the section you care about at ` +
            `${TARGET_DISTANCE} u. Pressure falls off linearly to zero at ` +
            "the radius, every destroyed layer passes 65% on, and a layer " +
            "that survives stops the wave dead. Defaults are the shipped " +
            "torpedo warhead."
    );

    const controls = el("div", "widget__controls");
    const stack = el("div", "widget__stack");
    const readout = el("p", "widget__readout");

    const update = (): void => {
        const damage = Number(damageControl.input.value);
        const radius = Number(radiusControl.input.value);
        const blast = blastWalk(
            damage,
            radius,
            LAYER_DISTANCES,
            hp,
            TARGET_DISTANCE
        );
        stack.replaceChildren();
        blast.layers.forEach((layer, i) => {
            const at = `@ ${LAYER_DISTANCES[i]} u`;
            if (layer.state === "dead") {
                stack.appendChild(
                    sectionCell(
                        "DEAD",
                        `${at}: in ${Math.round(layer.incoming)}, passes 65%`,
                        "is-dead"
                    )
                );
            } else if (layer.state === "holds") {
                stack.appendChild(
                    sectionCell(
                        "HOLDS",
                        `${at}: in ${Math.round(layer.incoming)} vs ${hp} hp`,
                        "is-hit"
                    )
                );
            } else {
                stack.appendChild(
                    sectionCell("SHIELDED", `${at}: 0`, "is-clear")
                );
            }
        });
        const destroyed = blast.layers.filter((l) => l.state === "dead").length;
        readout.classList.remove("is-warn");
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
    };
    const damageControl = control(
        "Blast damage",
        100,
        900,
        25,
        TORPEDO_BLAST_DAMAGE,
        (v) => `${v} hp`,
        update
    );
    const radiusControl = control(
        "Blast radius",
        16,
        60,
        2,
        TORPEDO_BLAST_RADIUS,
        (v) => `${v} u`,
        update
    );
    controls.appendChild(damageControl.row);
    controls.appendChild(radiusControl.row);

    host.appendChild(controls);
    host.appendChild(stack);
    host.appendChild(readout);
    update();
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

// ---- activation -----------------------------------------------------------

const WIDGETS: Record<string, (host: HTMLElement) => void> = {
    "aim-decay": initAimDecay,
    "round-travel": initRoundTravel,
    "blast-layers": initBlastLayers,
    "controller-stacking": initControllerStacking,
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
