import type { LetteredPage } from "./comic-page";

const NS = "http://www.w3.org/2000/svg";
const FONT = "DejaVu Sans, sans-serif";

/** Preserve authored line breaks; otherwise wrap with the browser's actual font metrics. */
export function wrapDialogue(
    text: string,
    width: number,
    measure: (text: string) => number
): string[] {
    const lines: string[] = [];
    for (const paragraph of text.split("\n")) {
        let line = "";
        for (const word of paragraph.trim().split(/\s+/)) {
            const next = line ? `${line} ${word}` : word;
            if (line && measure(next) > width) {
                lines.push(line);
                line = word;
            } else line = next;
        }
        lines.push(line);
    }
    return lines;
}

/** Construct engine-owned SVG geometry; callers supply validated data. */
export function svgNode<K extends keyof SVGElementTagNameMap>(
    tag: K,
    attributes: Record<string, string | number> = {}
): SVGElementTagNameMap[K] {
    const result = document.createElementNS(NS, tag);
    for (const [key, value] of Object.entries(attributes))
        result.setAttribute(key, String(value));
    return result;
}

/** Reject executable/external SVG content before importing generated art into the reader. */
export function readScene(source: string): SVGSVGElement {
    if (source.length > 2_000_000 || /<!DOCTYPE|<!ENTITY/i.test(source))
        throw new Error("Invalid comic scene document");
    const doc = new DOMParser().parseFromString(source, "image/svg+xml");
    const tags = new Set([
        "svg",
        "title",
        "desc",
        "defs",
        "g",
        "path",
        "rect",
        "ellipse",
        "circle",
        "polygon",
        "polyline",
        "line",
        "text",
        "linearGradient",
        "radialGradient",
        "stop",
        "clipPath",
    ]);
    const ids = new Set<string>();
    for (const element of Array.from(doc.querySelectorAll("*"))) {
        if (element.namespaceURI !== NS || !tags.has(element.localName))
            throw new Error(`Unsafe comic SVG element: ${element.localName}`);
        if (element.id) {
            if (ids.has(element.id)) throw new Error("Duplicate comic SVG id");
            ids.add(element.id);
        }
        for (const attribute of Array.from(element.attributes)) {
            if (
                /^on/i.test(attribute.localName) ||
                ["href", "style"].includes(attribute.localName.toLowerCase()) ||
                (/url\(/i.test(attribute.value) &&
                    !/^url\(#[a-zA-Z0-9_-]+\)$/.test(attribute.value))
            )
                throw new Error("Unsafe comic SVG attribute");
        }
    }
    if (doc.documentElement.localName !== "svg")
        throw new Error("Comic scene needs an SVG root");
    return document.importNode(
        doc.documentElement,
        true
    ) as unknown as SVGSVGElement;
}

/** Measured boxes retain their own coordinate system for panel and page checks. */
export interface LetteringLayer {
    lettering: SVGGraphicsElement;
    boxes: {
        x: number;
        y: number;
        width: number;
        height: number;
        element: SVGElement;
    }[];
    width: number;
    height: number;
    label: string;
}

const node = svgNode;

/** The corner radius the outline turns on, so the straight runs are inset by it. */
const CORNER = 24;

/**
 * How far a tail's notch reaches either side of where it attaches, in the
 * direction the outline is being drawn. Asymmetric on purpose: the tail leans
 * into the reading direction, so the far lip is the long one.
 */
const NOTCH = {
    top: { lead: 5, trail: 18 },
    bottom: { lead: 5, trail: 18 },
    left: { lead: 12, trail: 10 },
    right: { lead: 12, trail: 10 },
} as const;

/**
 * Where a tail cuts its notch out of one straight run of the outline.
 *
 * Both lips have to land INSIDE the run. Outside it the outline reverses - the
 * path doubles back over itself and the stroke draws a spur across the balloon
 * - and it does not fail loudly, it just draws wrong. Two things can push a lip
 * out: an attachment point near a corner, which the clamp handles, and a run
 * shorter than the notch itself, which no clamp can. A balloon is
 * `42 + 27 * lines` tall, so a one-line balloon leaves `69 - 2 * 24` = 21 px of
 * vertical run against a 22 px notch; the notch shrinks to fit rather than the
 * balloon growing to hold it, because the balloon's height is the text's.
 *
 * Returns the two lips in drawing order, low coordinate first; a run drawn
 * backwards emits them the other way round.
 */
function tailCut(
    anchor: number,
    runStart: number,
    runEnd: number,
    lead: number,
    trail: number
): [number, number] {
    const run = Math.max(0, runEnd - runStart);
    const fit = Math.min(1, (run * 0.8) / (lead + trail));
    const [near, far] = [lead * fit, trail * fit];
    const at = Math.min(Math.max(anchor, runStart + near), runEnd - far);
    return [at - near, at + far];
}

/**
 * The balloon's closed outline, and the tail tip the outline actually reaches.
 *
 * Pure geometry, separate from the DOM the rest of this module builds, so the
 * one thing about a balloon that can be wrong without looking wrong in a test
 * fixture - a reversed outline - can be asserted directly.
 */
export function balloonOutline(
    x: number,
    y: number,
    w: number,
    h: number,
    side: "top" | "bottom" | "left" | "right",
    tip: [number, number]
): { d: string; tip: [number, number] } {
    let [tx, ty] = tip;
    const ax =
        side === "left"
            ? x
            : side === "right"
              ? x + w
              : Math.max(x + 25, Math.min(x + w - 30, tx));
    const ay = side === "top" ? y : side === "bottom" ? y + h : y + h * 0.65;
    const length = Math.hypot(tx - ax, ty - ay);
    if (length > 65) {
        tx = ax + ((tx - ax) * 65) / length;
        ty = ay + ((ty - ay) * 65) / length;
    }
    const across = tailCut(
        ax,
        x + CORNER,
        x + w - CORNER,
        NOTCH.top.lead,
        NOTCH.top.trail
    );
    const down = tailCut(
        ay,
        y + CORNER,
        y + h - CORNER,
        NOTCH.right.lead,
        NOTCH.right.trail
    );
    let d = `M${x + CORNER} ${y}`;
    if (side === "top") d += `H${across[0]}L${tx} ${ty}L${across[1]} ${y}`;
    d += `H${x + w - CORNER}Q${x + w} ${y} ${x + w} ${y + CORNER}`;
    if (side === "right") d += `V${down[0]}L${tx} ${ty}L${x + w} ${down[1]}`;
    d += `V${y + h - CORNER}Q${x + w} ${y + h} ${x + w - CORNER} ${y + h}`;
    if (side === "bottom")
        d += `H${across[1]}L${tx} ${ty}L${across[0]} ${y + h}`;
    d += `H${x + CORNER}Q${x} ${y + h} ${x} ${y + h - CORNER}`;
    if (side === "left") d += `V${down[1]}L${tx} ${ty}L${x} ${down[0]}`;
    d += `V${y + CORNER}Q${x} ${y} ${x + CORNER} ${y}Z`;
    return { d, tip: [tx, ty] };
}

/** Letter literal text in panel-local or page-local drawing coordinates. */
export function drawBalloons(
    page: Pick<LetteredPage, "balloons" | "palette">
): Pick<LetteringLayer, "lettering" | "boxes"> {
    const lettering = node("g", { class: "comic-lettering" });
    const canvas = document.createElement("canvas").getContext("2d");
    if (!canvas) throw new Error("Comic lettering needs font measurement");
    canvas.font = `22px ${FONT}`;
    const boxes: LetteringLayer["boxes"] = [];
    for (const balloon of page.balloons) {
        const { x, y, width: w, side } = balloon;
        const lines = wrapDialogue(
            balloon.text,
            w - 36,
            (text) => canvas.measureText(text).width
        );
        const h = 42 + 27 * lines.length;
        const { d } = balloonOutline(x, y, w, h, side, balloon.tip);
        const group = node("g", { "data-balloon": boxes.length });
        group.append(
            node("path", {
                d,
                fill: page.palette.balloon,
                stroke: page.palette.ink,
                "stroke-width": 2.5,
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
            })
        );
        const label = node("text", {
            x: x + 18,
            y: y + 25,
            "font-family": FONT,
            "font-size": 13,
            "font-weight": "bold",
            "letter-spacing": 1.3,
            fill: page.palette.speaker,
        });
        label.textContent = balloon.speaker.toUpperCase();
        group.append(label);
        lines.forEach((line, index) => {
            const text = node("text", {
                x: x + 18,
                y: y + 52 + 27 * index,
                "font-family": FONT,
                "font-size": 22,
                fill: page.palette.ink,
            });
            text.textContent = line;
            group.append(text);
        });
        boxes.push({ x, y, width: w, height: h, element: group });
        lettering.append(group);
    }
    return { lettering, boxes };
}

/** Serialize the actual composition without a second layout pass. */
export function exportSvg(svg: SVGSVGElement): string {
    return URL.createObjectURL(
        new Blob([new XMLSerializer().serializeToString(svg)], {
            type: "image/svg+xml",
        })
    );
}

/** Export the page and expose measured results to automated checks, without review UI. */
export function finishPage(
    host: HTMLElement,
    svg: SVGSVGElement,
    layers: LetteringLayer[]
): void {
    host.replaceChildren(svg);
    host.dataset.artUrl = exportSvg(svg);
    host.dataset.layoutReady = "true";
    const check = () => {
        if (host.hidden || !svg.getScreenCTM()) return;
        const warnings: string[] = [];
        const seen: { a: DOMPoint; b: DOMPoint; name: string }[] = [];
        for (const layer of layers)
            layer.boxes.forEach((box, i) => {
                const name = `${layer.label}balloon ${i + 1}`;
                for (const text of Array.from(
                    box.element.querySelectorAll("text")
                )) {
                    const b = text.getBBox();
                    if (
                        b.x < box.x + 8 ||
                        b.x + b.width > box.x + box.width - 10 ||
                        b.y + b.height > box.y + box.height - 8
                    )
                        warnings.push(`${name}: text overflow`);
                }
                if (
                    box.x < 0 ||
                    box.y < 0 ||
                    box.x + box.width > layer.width ||
                    box.y + box.height > layer.height
                )
                    warnings.push(`${name}: outside panel/page`);
                const local = layer.lettering.getScreenCTM();
                const a = new DOMPoint(box.x, box.y).matrixTransform(local),
                    b = new DOMPoint(
                        box.x + box.width,
                        box.y + box.height
                    ).matrixTransform(local);
                for (const face of Array.from(
                    svg.querySelectorAll("[data-protect-face]")
                )) {
                    const f = face.getBoundingClientRect();
                    if (
                        Math.min(f.right, b.x) - Math.max(f.left, a.x) > 2 &&
                        Math.min(f.bottom, b.y) - Math.max(f.top, a.y) > 2
                    )
                        warnings.push(
                            `${name}: overlaps ${face.getAttribute("data-protect-face")}`
                        );
                }
                for (const other of seen)
                    if (
                        Math.min(b.x, other.b.x) - Math.max(a.x, other.a.x) >
                            0.01 &&
                        Math.min(b.y, other.b.y) - Math.max(a.y, other.a.y) >
                            0.01
                    )
                        warnings.push(`${other.name}/${name}: overlap`);
                seen.push({ a, b, name });
            });
        host.dataset.layoutIssues = JSON.stringify(warnings);
    };
    host.addEventListener("comic:show", () => requestAnimationFrame(check));
    new ResizeObserver(check).observe(host);
    host.dispatchEvent(new Event("comic:ready", { bubbles: true }));
    requestAnimationFrame(check);
}

/** Compose a complete scene asset with page-local measured balloons. */
export async function letterPage(
    host: HTMLElement,
    page: LetteredPage,
    imageUrl: string
): Promise<void> {
    const response = await fetch(imageUrl);
    if (!response.ok) throw new Error(`Comic scene failed: ${response.status}`);
    const scene = readScene(await response.text());
    await document.fonts.ready;
    const svg = node("svg", {
        viewBox: `0 0 ${page.width} ${page.height}`,
        width: page.width,
        height: page.height,
        role: "img",
        "aria-label": page.alt,
    });
    const layer = drawBalloons(page);
    svg.append(scene, layer.lettering);
    finishPage(host, svg, [
        { ...layer, width: page.width, height: page.height, label: "" },
    ]);
}
