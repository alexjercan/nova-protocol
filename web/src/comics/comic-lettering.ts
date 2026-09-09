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
        let [tx, ty] = balloon.tip;
        const ax =
            side === "left"
                ? x
                : side === "right"
                  ? x + w
                  : Math.max(x + 25, Math.min(x + w - 30, tx));
        const ay =
            side === "top" ? y : side === "bottom" ? y + h : y + h * 0.65;
        const length = Math.hypot(tx - ax, ty - ay);
        if (length > 65) {
            tx = ax + ((tx - ax) * 65) / length;
            ty = ay + ((ty - ay) * 65) / length;
        }
        let d = `M${x + 24} ${y}`;
        if (side === "top") d += `H${ax - 5}L${tx} ${ty}L${ax + 18} ${y}`;
        d += `H${x + w - 24}Q${x + w} ${y} ${x + w} ${y + 24}`;
        if (side === "right")
            d += `V${ay - 12}L${tx} ${ty}L${x + w} ${ay + 10}`;
        d += `V${y + h - 24}Q${x + w} ${y + h} ${x + w - 24} ${y + h}`;
        if (side === "bottom")
            d += `H${ax + 18}L${tx} ${ty}L${ax - 5} ${y + h}`;
        d += `H${x + 24}Q${x} ${y + h} ${x} ${y + h - 24}`;
        if (side === "left") d += `V${ay + 10}L${tx} ${ty}L${x} ${ay - 12}`;
        d += `V${y + 24}Q${x} ${y} ${x + 24} ${y}Z`;
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
