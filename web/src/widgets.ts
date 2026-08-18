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
