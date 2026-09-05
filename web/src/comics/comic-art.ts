/**
 * Semantic drawing helpers for inline comic art.
 *
 * Every helper returns plain `SvgNode` data built from numbers, so pages
 * describe ships, marks and effects by name and the renderer stays the only
 * place that touches the document.
 */

import {
    ComicTone,
    Point,
    SvgNode,
    circle,
    ellipse,
    group,
    line,
    polyline,
    rect,
} from "./comic-page";

export type ShipKind =
    | "cutter"
    | "tender"
    | "carrier"
    | "corvette"
    | "warship"
    | "picket"
    | "claw"
    | "skiff"
    | "tug"
    | "leader"
    | "boat";

interface Placement {
    at: Point;
    scale?: number;
    rotate?: number;
    tone?: ComicTone;
    opacity?: number;
}

/** A deterministic generator so debris and stars are the same on every render. */
function sequence(seed: number): () => number {
    let state = seed >>> 0;
    return () => {
        state = (state + 0x6d2b79f5) >>> 0;
        let t = state;
        t = Math.imul(t ^ (t >>> 15), t | 1);
        t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}

function placed(placement: Placement, ...children: SvgNode[]): SvgNode {
    return group(
        {
            translate: placement.at,
            scale: placement.scale ?? 1,
            rotate: placement.rotate ?? 0,
            opacity: placement.opacity ?? 1,
        },
        ...children
    );
}

function hull(kind: ShipKind, tone: ComicTone): SvgNode[] {
    const solid = { tone, fill: true, closed: true, width: 3 };
    switch (kind) {
        case "cutter":
            return [
                polyline({
                    ...solid,
                    points: [
                        [-40, -12],
                        [28, -12],
                        [48, 0],
                        [28, 12],
                        [-40, 12],
                    ],
                }),
                rect({ at: [-6, -7], size: [22, 14], tone, fill: true }),
                rect({
                    at: [-48, -6],
                    size: [8, 12],
                    tone: "amber",
                    fill: true,
                }),
            ];
        case "tender":
            return [
                rect({
                    at: [-50, -16],
                    size: [90, 32],
                    tone,
                    fill: true,
                    width: 3,
                }),
                rect({ at: [40, -8], size: [14, 16], tone, fill: true }),
                rect({ at: [30, -24], size: [16, 8], tone, fill: true }),
                rect({ at: [30, 16], size: [16, 8], tone, fill: true }),
                circle({ center: [-56, 0], radius: 14, tone, width: 3 }),
                line({ from: [-56, -14], to: [-56, 14], tone }),
                rect({
                    at: [-62, -6],
                    size: [6, 12],
                    tone: "amber",
                    fill: true,
                }),
            ];
        case "carrier":
            return [
                polyline({
                    ...solid,
                    points: [
                        [-120, -22],
                        [-86, -40],
                        [86, -40],
                        [120, -22],
                        [120, 22],
                        [86, 40],
                        [-86, 40],
                        [-120, 22],
                    ],
                }),
                rect({
                    at: [-30, -60],
                    size: [60, 120],
                    tone,
                    fill: true,
                    width: 3,
                }),
                line({
                    from: [-24, -52],
                    to: [24, -52],
                    tone: "amber",
                    width: 6,
                }),
                line({
                    from: [-24, 52],
                    to: [24, 52],
                    tone: "amber",
                    width: 6,
                }),
                line({ from: [-70, -30], to: [-70, 30], tone, width: 2 }),
                line({ from: [70, -30], to: [70, 30], tone, width: 2 }),
            ];
        case "corvette":
            return [
                polyline({
                    ...solid,
                    points: [
                        [-70, -14],
                        [36, -14],
                        [80, 0],
                        [36, 14],
                        [-70, 14],
                    ],
                }),
                polyline({
                    ...solid,
                    points: [
                        [-40, -14],
                        [-20, -30],
                        [0, -14],
                    ],
                }),
                rect({
                    at: [-78, -8],
                    size: [8, 16],
                    tone: "blue",
                    fill: true,
                }),
            ];
        case "warship":
            return [
                polyline({
                    ...solid,
                    points: [
                        [-110, -22],
                        [-60, -36],
                        [70, -36],
                        [120, 0],
                        [70, 36],
                        [-60, 36],
                        [-110, 22],
                    ],
                }),
                rect({ at: [20, -46], size: [70, 12], tone, fill: true }),
                rect({ at: [20, 34], size: [70, 12], tone, fill: true }),
                rect({ at: [-40, -14], size: [50, 28], tone, fill: true }),
                line({
                    from: [-120, -10],
                    to: [-110, -10],
                    tone: "danger",
                    width: 4,
                }),
                line({
                    from: [-120, 10],
                    to: [-110, 10],
                    tone: "danger",
                    width: 4,
                }),
            ];
        case "picket":
            return [
                rect({
                    at: [-22, -18],
                    size: [44, 36],
                    tone,
                    fill: true,
                    width: 3,
                }),
                rect({ at: [22, -6], size: [14, 12], tone, fill: true }),
                line({ from: [0, -18], to: [0, -30], tone, width: 3 }),
            ];
        case "claw":
            return [
                polyline({
                    ...solid,
                    points: [
                        [-40, -14],
                        [20, -14],
                        [36, 0],
                        [20, 14],
                        [-40, 14],
                    ],
                }),
                polyline({
                    points: [
                        [20, -14],
                        [50, -34],
                        [72, -26],
                    ],
                    tone,
                    width: 4,
                }),
                polyline({
                    points: [
                        [50, -34],
                        [60, -12],
                    ],
                    tone,
                    width: 4,
                }),
            ];
        case "skiff":
            return [
                polyline({
                    ...solid,
                    points: [
                        [-18, -8],
                        [14, -8],
                        [24, 0],
                        [14, 8],
                        [-18, 8],
                    ],
                }),
            ];
        case "tug":
            return [
                rect({
                    at: [-20, -12],
                    size: [32, 24],
                    tone,
                    fill: true,
                    width: 3,
                }),
                rect({ at: [12, -6], size: [16, 12], tone, fill: true }),
            ];
        case "leader":
            return [
                polyline({
                    ...solid,
                    points: [
                        [-60, -16],
                        [30, -16],
                        [60, 0],
                        [30, 16],
                        [-60, 16],
                    ],
                }),
                rect({ at: [-30, -30], size: [40, 14], tone, fill: true }),
                rect({ at: [-30, 16], size: [40, 14], tone, fill: true }),
            ];
        case "boat":
            return [
                ellipse({
                    center: [0, 0],
                    radii: [16, 8],
                    tone,
                    fill: true,
                    width: 2,
                }),
                circle({
                    center: [-18, 0],
                    radius: 3,
                    tone: "amber",
                    fill: true,
                }),
            ];
    }
}

/** A block-fleet ship silhouette, nose to the right in its own frame. */
export function ship(kind: ShipKind, placement: Placement): SvgNode {
    return placed(placement, ...hull(kind, placement.tone ?? "default"));
}

/** Speed lines trailing from a point toward the left. */
export function motionLines(options: {
    from: Point;
    count?: number;
    length?: number;
    spread?: number;
    tone?: ComicTone;
}): SvgNode[] {
    const count = options.count ?? 4;
    const length = options.length ?? 60;
    const spread = options.spread ?? 24;
    const tone = options.tone ?? "muted";
    const [x, y] = options.from;
    return Array.from({ length: count }, (_, index) => {
        const offset =
            (index - (count - 1) / 2) * (spread / Math.max(count - 1, 1));
        const shrink = 1 - Math.abs(offset) / (spread + 1);
        return line({
            from: [x, y + offset],
            to: [x - length * (0.5 + shrink / 2), y + offset],
            tone,
            width: 2,
        });
    });
}

/** An explosion burst: rays around a core. */
export function burst(options: {
    center: Point;
    radius: number;
    rays?: number;
    tone?: ComicTone;
}): SvgNode[] {
    const rays = options.rays ?? 10;
    const tone = options.tone ?? "amber";
    const [cx, cy] = options.center;
    const nodes: SvgNode[] = [
        circle({
            center: [cx, cy],
            radius: options.radius * 0.3,
            tone,
            fill: true,
            width: 0,
        }),
    ];
    for (let index = 0; index < rays; index += 1) {
        const angle = (index / rays) * Math.PI * 2;
        const reach = options.radius * (index % 2 === 0 ? 1 : 0.62);
        nodes.push(
            line({
                from: [
                    cx + Math.cos(angle) * options.radius * 0.35,
                    cy + Math.sin(angle) * options.radius * 0.35,
                ],
                to: [
                    cx + Math.cos(angle) * reach,
                    cy + Math.sin(angle) * reach,
                ],
                tone,
                width: index % 2 === 0 ? 4 : 2,
            })
        );
    }
    return nodes;
}

/** Scattered wreck fragments inside a radius, fixed by the seed. */
export function debris(options: {
    center: Point;
    radius: number;
    count?: number;
    seed?: number;
    tone?: ComicTone;
}): SvgNode[] {
    const next = sequence(options.seed ?? 7);
    const tone = options.tone ?? "muted";
    const [cx, cy] = options.center;
    return Array.from({ length: options.count ?? 14 }, () => {
        const angle = next() * Math.PI * 2;
        const distance = Math.sqrt(next()) * options.radius;
        const size = 4 + next() * 10;
        return group(
            {
                translate: [
                    cx + Math.cos(angle) * distance,
                    cy + Math.sin(angle) * distance,
                ],
                rotate: next() * 360,
            },
            polyline({
                points: [
                    [-size, -size * 0.4],
                    [size * 0.6, -size * 0.7],
                    [size, size * 0.3],
                    [-size * 0.3, size * 0.8],
                ],
                tone,
                width: 1.5,
                closed: true,
                fill: true,
            })
        );
    });
}

/** A dashed orbit ring with a bright marker at an angle in degrees. */
export function orbitPath(options: {
    center: Point;
    radius: number;
    marker?: number;
    tone?: ComicTone;
}): SvgNode[] {
    const tone = options.tone ?? "amber";
    const nodes: SvgNode[] = [
        circle({
            center: options.center,
            radius: options.radius,
            tone,
            width: 2,
        }),
    ];
    if (options.marker !== undefined) {
        const angle = (options.marker * Math.PI) / 180;
        nodes.push(
            circle({
                center: [
                    options.center[0] + Math.cos(angle) * options.radius,
                    options.center[1] + Math.sin(angle) * options.radius,
                ],
                radius: 5,
                tone,
                fill: true,
                width: 0,
            })
        );
    }
    return nodes;
}

/** A dashed search lane with an arrowhead at its end. */
export function searchLane(options: {
    from: Point;
    to: Point;
    tone?: ComicTone;
}): SvgNode[] {
    const tone = options.tone ?? "danger";
    const [x1, y1] = options.from;
    const [x2, y2] = options.to;
    const angle = Math.atan2(y2 - y1, x2 - x1);
    const head = 10;
    return [
        line({
            from: options.from,
            to: options.to,
            tone,
            width: 2,
            dash: [10, 8],
        }),
        polyline({
            points: [
                [
                    x2 - Math.cos(angle - 0.5) * head,
                    y2 - Math.sin(angle - 0.5) * head,
                ],
                [x2, y2],
                [
                    x2 - Math.cos(angle + 0.5) * head,
                    y2 - Math.sin(angle + 0.5) * head,
                ],
            ],
            tone,
            width: 2,
        }),
    ];
}

/** Concentric rings spreading from a point: a signal or a beacon. */
export function signalRings(options: {
    center: Point;
    count?: number;
    step?: number;
    start?: number;
    tone?: ComicTone;
}): SvgNode[] {
    const count = options.count ?? 3;
    const step = options.step ?? 18;
    const start = options.start ?? 10;
    const tone = options.tone ?? "default";
    return Array.from({ length: count }, (_, index) =>
        circle({
            center: options.center,
            radius: start + step * index,
            tone,
            width: Math.max(0.6, 2.4 - index * 0.5),
        })
    );
}

/** A diagonal hazard band inside a box. */
export function hazardBand(options: {
    at: Point;
    size: Point;
    tone?: ComicTone;
    pitch?: number;
}): SvgNode[] {
    const tone = options.tone ?? "amber";
    const pitch = options.pitch ?? 16;
    const [x, y] = options.at;
    const [w, h] = options.size;
    const nodes: SvgNode[] = [
        rect({ at: options.at, size: options.size, tone, width: 2 }),
    ];
    for (let offset = -h; offset < w; offset += pitch) {
        const x1 = Math.max(x, x + offset);
        const y1 = y + (x1 - (x + offset));
        const x2 = Math.min(x + w, x + offset + h);
        const y2 = y + (x2 - (x + offset));
        if (x2 > x1)
            nodes.push(line({ from: [x1, y1], to: [x2, y2], tone, width: 3 }));
    }
    return nodes;
}

/** A field of small stars inside a box, fixed by the seed. */
export function starfield(options: {
    size: Point;
    count?: number;
    seed?: number;
    tone?: ComicTone;
}): SvgNode[] {
    const next = sequence(options.seed ?? 1);
    const tone = options.tone ?? "muted";
    return Array.from({ length: options.count ?? 24 }, () =>
        circle({
            center: [next() * options.size[0], next() * options.size[1]],
            radius: 0.8 + next() * 1.4,
            tone,
            fill: true,
            width: 0,
        })
    );
}

/** A CRT grid over a box. */
export function crtGrid(options: {
    size: Point;
    step?: number;
    tone?: ComicTone;
}): SvgNode[] {
    const step = options.step ?? 30;
    const tone = options.tone ?? "muted";
    const nodes: SvgNode[] = [];
    for (let x = step; x < options.size[0]; x += step) {
        nodes.push(
            line({ from: [x, 0], to: [x, options.size[1]], tone, width: 0.5 })
        );
    }
    for (let y = step; y < options.size[1]; y += step) {
        nodes.push(
            line({ from: [0, y], to: [options.size[0], y], tone, width: 0.5 })
        );
    }
    return nodes;
}

/** A rounded planetoid with a terminator shadow. */
export function planetoid(options: {
    center: Point;
    radius: number;
    tone?: ComicTone;
}): SvgNode[] {
    const tone = options.tone ?? "muted";
    const [cx, cy] = options.center;
    return [
        circle({
            center: options.center,
            radius: options.radius,
            tone,
            fill: true,
            width: 3,
        }),
        ellipse({
            center: [cx + options.radius * 0.35, cy],
            radii: [options.radius * 0.55, options.radius * 0.98],
            tone,
            fill: true,
            width: 0,
        }),
    ];
}

/** A torpedo track: a short bright line with a warhead dot. */
export function torpedo(options: {
    from: Point;
    to: Point;
    tone?: ComicTone;
}): SvgNode[] {
    const tone = options.tone ?? "danger";
    return [
        line({
            from: options.from,
            to: options.to,
            tone,
            width: 2,
            dash: [6, 4],
        }),
        circle({ center: options.to, radius: 4, tone, fill: true, width: 0 }),
    ];
}

/** A point-defence tracer fan from a muzzle toward a target. */
export function tracerFan(options: {
    from: Point;
    to: Point;
    count?: number;
    tone?: ComicTone;
}): SvgNode[] {
    const count = options.count ?? 5;
    const tone = options.tone ?? "amber";
    const [x1, y1] = options.from;
    const [x2, y2] = options.to;
    return Array.from({ length: count }, (_, index) => {
        const t = 0.25 + (index / Math.max(count - 1, 1)) * 0.7;
        const jitter = (index - (count - 1) / 2) * 5;
        return line({
            from: options.from,
            to: [x1 + (x2 - x1) * t, y1 + (y2 - y1) * t + jitter],
            tone,
            width: 1.5,
        });
    });
}
