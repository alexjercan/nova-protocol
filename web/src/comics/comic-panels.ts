import type { CompiledPanel, PanelPage } from "./comic-script";
import {
    drawBalloons,
    exportSvg,
    finishPage,
    readScene,
    svgNode,
} from "./comic-lettering";
import type { LetteringLayer } from "./comic-lettering";

function words(
    text: string,
    x: number,
    y: number,
    size: number,
    fill: string,
    attributes: Record<string, string | number> = {}
): SVGElement {
    const node = svgNode("text", {
        x,
        y,
        "font-family": "DejaVu Sans, sans-serif",
        "font-size": size,
        fill,
        ...attributes,
    });
    node.textContent = text;
    return node;
}
function rect(
    x: number,
    y: number,
    width: number,
    height: number,
    fill: string,
    stroke = "none",
    line = 2,
    radius = 0
): SVGElement {
    return svgNode("rect", {
        x,
        y,
        width,
        height,
        fill,
        stroke,
        "stroke-width": line,
        rx: radius,
    });
}
function scopeScene(scene: SVGSVGElement, prefix: string): void {
    const ids = new Map<string, string>();
    for (const n of [scene, ...Array.from(scene.querySelectorAll("[id]"))])
        if (n.id) ids.set(n.id, `${prefix}-${n.id}`);
    for (const n of [scene, ...Array.from(scene.querySelectorAll("*"))]) {
        if (ids.has(n.id)) n.id = ids.get(n.id);
        for (const attribute of Array.from(n.attributes)) {
            const match = /^url\(#(.+)\)$/.exec(attribute.value);
            if (match) {
                const id = ids.get(match[1]);
                if (!id) throw new Error("Missing scene paint/clip reference");
                n.setAttribute(attribute.name, `url(#${id})`);
            }
        }
    }
}
function letteredText(text: string, breaks: number[]): string {
    if (!breaks.length) return text;
    return text
        .trim()
        .split(/\s+/)
        .map((word, i) => word + (breaks.includes(i + 1) ? "\n" : " "))
        .join("")
        .trim();
}
function panelArt(
    panel: CompiledPanel,
    scene: SVGSVGElement,
    palette: Record<string, string>,
    prefix: string
): { svg: SVGSVGElement; layer: LetteringLayer } {
    scopeScene(scene, prefix);
    const [width, height] = panel.sceneSize;
    const svg = svgNode("svg", {
        width,
        height,
        viewBox: `0 0 ${width} ${height}`,
        overflow: "visible",
        role: "img",
        "aria-label": panel.action,
        "data-panel": panel.id,
    });
    scene.setAttribute("width", String(width));
    scene.setAttribute("height", String(height));
    for (const label of panel.labels) {
        const slot = scene.querySelector(`[data-story-slot="${label.slot}"]`);
        if (!slot) throw new Error(`Missing scene text slot: ${label.slot}`);
        slot.append(
            words(label.text, ...label.at, label.size, palette[label.color], {
                "font-weight": label.weight,
                "letter-spacing": label.spacing,
                "text-anchor": label.anchor,
            })
        );
    }
    const clipId = `${prefix}-panel-clip`;
    const clip = svgNode("clipPath", { id: clipId });
    clip.append(rect(0, 0, width, height, "white"));
    const defs = svgNode("defs");
    defs.append(clip);
    const clipped = svgNode("g", { "clip-path": `url(#${clipId})` });
    clipped.append(scene);
    for (const card of panel.cards) {
        const [x, y] = card.at,
            w = card.width;
        const group = svgNode("g", { class: "title-card" });
        const back = rect(
            x,
            y,
            w,
            137,
            palette["work-screen"],
            palette["work-card-border"],
            1,
            2
        );
        back.setAttribute("opacity", ".97");
        group.append(
            back,
            rect(x, y, 5, 137, palette.mint),
            words(card.name, x + 21, y + 34, 26, palette["work-card-title"], {
                "font-weight": "bold",
                "letter-spacing": 2.5,
            }),
            words(card.place, x + 21, y + 62, 16, palette.mint),
            words(card.date, x + 21, y + 89, 13, palette["work-warning"], {
                "letter-spacing": 0.4,
            }),
            words(card.note, x + 21, y + 116, 15, palette["work-card-text"])
        );
        clipped.append(group);
    }
    const layer = drawBalloons({
        palette: {
            ink: palette.ink,
            balloon: palette.balloon,
            speaker: palette.speaker,
        },
        balloons: panel.dialogue.map((d) => {
            const b = panel.lettering[d.id];
            return {
                x: b.at[0],
                y: b.at[1],
                width: b.width,
                tip: b.tail,
                side: b.side,
                speaker: d.speaker,
                text: letteredText(d.text, b.breakAfter),
            };
        }),
    });
    svg.append(
        defs,
        clipped,
        rect(0, 0, width, height, "none", palette.ink, 2.5),
        layer.lettering
    );
    return {
        svg,
        layer: { ...layer, width, height, label: `Panel ${panel.id}: ` },
    };
}

/** Compose selected panel assets, story labels, cards, and measured dialogue in one engine. */
export async function composePanels(
    host: HTMLElement,
    page: PanelPage,
    assetUrl: (name: string) => string
): Promise<void> {
    const scenes = await Promise.all(
        page.panels.map(async (panel) => {
            const response = await fetch(assetUrl(panel.image));
            if (!response.ok)
                throw new Error(`Comic scene failed: ${response.status}`);
            return readScene(await response.text());
        })
    );
    await document.fonts.ready;
    const svg = svgNode("svg", {
        width: page.width,
        height: page.height,
        viewBox: `0 0 ${page.width} ${page.height}`,
        role: "img",
        "aria-label": page.alt,
    });
    const p = page.palette;
    svg.append(
        rect(0, 0, page.width, page.height, p.paper),
        words("NOVA PROTOCOL", 43, 48, 21, p["folio-title"], {
            "font-weight": "bold",
            "letter-spacing": 3.5,
        }),
        words(page.heading, 412, 48, 13, p["folio-muted"], {
            "letter-spacing": 2,
        }),
        words(String(page.number).padStart(2, "0"), 1456, 48, 24, p.ink, {
            "text-anchor": "end",
            "font-weight": "bold",
        })
    );
    const layers: LetteringLayer[] = [];
    const images = page.panels.map((panel, index) => {
        const composed = panelArt(
            panel,
            scenes[index],
            p,
            `${host.id}-${panel.id}`
        );
        const image = {
            id: panel.id,
            art: assetUrl(panel.image),
            composed: exportSvg(composed.svg),
        };
        const [x, y] = panel.placement.at,
            [width, height] = panel.placement.size;
        for (const [key, value] of Object.entries({ x, y, width, height }))
            composed.svg.setAttribute(key, String(value));
        svg.append(composed.svg);
        layers.push(composed.layer);
        return image;
    });
    svg.append(
        words(page.footer, 43, 977, 12, p["folio-muted"], {
            "letter-spacing": 1.2,
        }),
        words(page.title.toUpperCase(), 1456, 977, 12, p["folio-title"], {
            "text-anchor": "end",
            "letter-spacing": 1.5,
        })
    );
    host.dataset.panelExports = JSON.stringify(images);
    finishPage(host, svg, layers);
}
