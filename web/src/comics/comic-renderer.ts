/**
 * Renders typed comic pages into DOM.
 *
 * All text enters the document through `textContent`. Every attribute is a
 * number, an enumerated token, or validated path data, so a page module has
 * no way to emit markup, URLs outside the comic's asset folder, or styles.
 */

import { ComicNode, ComicPage, ComicTone, SvgNode } from "./comic-page";
import { letterPage } from "./comic-lettering";
import { composePanels } from "./comic-panels";

export interface RenderPageContext {
    id: string;
    number: number;
    comicPath: string;
    basePath: string;
}

const SVG_NAMESPACE = "http://www.w3.org/2000/svg";
const PATH_DATA = /^[MmLlHhVvCcSsQqTtAaZz0-9 ,.\-+eE]+$/;
const ASSET_SOURCE = /^[a-z0-9][a-z0-9/-]*\.(svg|png|webp)$/;
const MAX_GRID_TRACKS = 4;
const MAX_GRID_ROWS = 6;

function element<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className?: string,
    text?: string
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
}

/** `prefix` plus one `prefix--modifier` per modifier; the default tone adds none. */
function classNames(prefix: string, ...modifiers: string[]): string {
    return [
        prefix,
        ...modifiers
            .filter((modifier) => modifier !== "default")
            .map((modifier) => `${prefix}--${modifier}`),
    ].join(" ");
}

function number(value: number, what: string): string {
    if (!Number.isFinite(value)) {
        throw new Error(`Comic art needs a finite ${what}, got ${value}`);
    }
    return String(Math.round(value * 100) / 100);
}

function assetUrl(context: RenderPageContext, source: string): string {
    if (!ASSET_SOURCE.test(source) || source.includes("..")) {
        throw new Error(`Comic asset source is not allowed: ${source}`);
    }
    return `${context.basePath}story/assets/${context.comicPath}/${source}`;
}

function image(
    context: RenderPageContext,
    source: string,
    alt: string,
    focus?: string
): HTMLImageElement {
    const picture = element("img");
    picture.src = assetUrl(context, source);
    picture.alt = alt;
    if (focus) picture.dataset.focus = focus;
    return picture;
}

function svgElement(tag: string): SVGElement {
    return document.createElementNS(SVG_NAMESPACE, tag);
}

function strokeAttributes(
    shape: SVGElement,
    tone: ComicTone,
    width: number,
    fill: boolean,
    dash?: [number, number]
): void {
    const classes = ["comic-svg__shape", `comic-svg__shape--${tone}`];
    if (width > 0) shape.setAttribute("stroke-width", number(width, "width"));
    else classes.push("comic-svg__shape--unstroked");
    if (fill)
        classes.push(
            width > 0 ? "comic-svg__shape--filled" : "comic-svg__shape--solid"
        );
    shape.setAttribute("class", classes.join(" "));
    if (dash) {
        shape.setAttribute(
            "stroke-dasharray",
            `${number(dash[0], "dash")} ${number(dash[1], "dash")}`
        );
    }
}

function points(list: [number, number][]): string {
    return list
        .map(([x, y]) => `${number(x, "x")},${number(y, "y")}`)
        .join(" ");
}

function renderSvgNode(node: SvgNode): SVGElement {
    switch (node.kind) {
        case "circle": {
            const shape = svgElement("circle");
            shape.setAttribute("cx", number(node.center[0], "cx"));
            shape.setAttribute("cy", number(node.center[1], "cy"));
            shape.setAttribute("r", number(node.radius, "radius"));
            strokeAttributes(shape, node.tone, node.width, node.fill);
            return shape;
        }
        case "ellipse": {
            const shape = svgElement("ellipse");
            shape.setAttribute("cx", number(node.center[0], "cx"));
            shape.setAttribute("cy", number(node.center[1], "cy"));
            shape.setAttribute("rx", number(node.radii[0], "rx"));
            shape.setAttribute("ry", number(node.radii[1], "ry"));
            strokeAttributes(shape, node.tone, node.width, node.fill);
            return shape;
        }
        case "rect": {
            const shape = svgElement("rect");
            shape.setAttribute("x", number(node.at[0], "x"));
            shape.setAttribute("y", number(node.at[1], "y"));
            shape.setAttribute("width", number(node.size[0], "width"));
            shape.setAttribute("height", number(node.size[1], "height"));
            if (node.radius > 0) {
                shape.setAttribute("rx", number(node.radius, "radius"));
            }
            strokeAttributes(shape, node.tone, node.width, node.fill);
            return shape;
        }
        case "line": {
            const shape = svgElement("line");
            shape.setAttribute("x1", number(node.from[0], "x1"));
            shape.setAttribute("y1", number(node.from[1], "y1"));
            shape.setAttribute("x2", number(node.to[0], "x2"));
            shape.setAttribute("y2", number(node.to[1], "y2"));
            strokeAttributes(shape, node.tone, node.width, false, node.dash);
            return shape;
        }
        case "polyline": {
            const shape = svgElement(node.closed ? "polygon" : "polyline");
            shape.setAttribute("points", points(node.points));
            strokeAttributes(
                shape,
                node.tone,
                node.width,
                node.fill && node.closed,
                node.dash
            );
            return shape;
        }
        case "path": {
            if (!PATH_DATA.test(node.d)) {
                throw new Error(`Comic path data is not allowed: ${node.d}`);
            }
            const shape = svgElement("path");
            shape.setAttribute("d", node.d);
            strokeAttributes(
                shape,
                node.tone,
                node.width,
                node.fill,
                node.dash
            );
            return shape;
        }
        case "svgText": {
            const text = svgElement("text");
            text.setAttribute("x", number(node.at[0], "x"));
            text.setAttribute("y", number(node.at[1], "y"));
            text.setAttribute(
                "class",
                `comic-svg__text comic-svg__text--${node.tone}`
            );
            text.setAttribute("font-size", number(node.size, "size"));
            text.setAttribute("text-anchor", node.anchor);
            if (node.spacing) {
                text.setAttribute(
                    "letter-spacing",
                    number(node.spacing, "spacing")
                );
            }
            text.textContent = node.text;
            return text;
        }
        case "group": {
            const wrapper = svgElement("g");
            wrapper.setAttribute(
                "transform",
                `translate(${number(node.translate[0], "x")} ${number(node.translate[1], "y")}) rotate(${number(node.rotate, "rotate")}) scale(${number(node.scale, "scale")})`
            );
            if (node.opacity !== 1) {
                wrapper.setAttribute(
                    "opacity",
                    number(node.opacity, "opacity")
                );
            }
            wrapper.append(...node.children.map(renderSvgNode));
            return wrapper;
        }
    }
}

function gridTemplate(node: Extract<ComicNode, { kind: "grid" }>): {
    rows: string;
} {
    if (
        !Number.isInteger(node.columns) ||
        node.columns < 1 ||
        node.columns > MAX_GRID_TRACKS
    ) {
        throw new Error(`Comic grid columns must be 1 to ${MAX_GRID_TRACKS}`);
    }
    if (
        node.rows.length < 1 ||
        node.rows.length > MAX_GRID_ROWS ||
        node.rows.some(
            (row) => row !== "auto" && (!(row > 0) || !Number.isFinite(row))
        )
    ) {
        throw new Error(
            `Comic grid rows must be 1 to ${MAX_GRID_ROWS} positive weights`
        );
    }
    return {
        rows: node.rows
            .map((row) =>
                row === "auto" ? "auto" : `minmax(0, ${number(row, "row")}fr)`
            )
            .join(" "),
    };
}

function renderChildren(
    children: ComicNode[],
    context: RenderPageContext
): (HTMLElement | SVGElement)[] {
    return children.map((child) => renderNode(child, context));
}

function renderNode(
    node: ComicNode,
    context: RenderPageContext
): HTMLElement | SVGElement {
    switch (node.kind) {
        case "header": {
            const header = element("header", "comic-page__header");
            const eyebrow = element(
                "p",
                undefined,
                node.eyebrow ?? `Chapter ${node.number}`
            );
            header.append(
                eyebrow,
                element("h2", undefined, node.title),
                element("span", undefined, node.subtitle)
            );
            return header;
        }
        case "grid": {
            const grid = element("div", "comic-grid");
            const template = gridTemplate(node);
            grid.style.setProperty("--comic-columns", String(node.columns));
            grid.style.setProperty("--comic-rows", template.rows);
            grid.append(...renderChildren(node.children, context));
            return grid;
        }
        case "panel": {
            const classes = ["comic-panel"];
            if (node.variant !== "default") {
                classes.push(`comic-panel--${node.variant}`);
            }
            if (node.frame !== "default") {
                classes.push(`comic-panel--frame-${node.frame}`);
            }
            const panel = element("figure", classes.join(" "));
            if (node.label) panel.setAttribute("aria-label", node.label);
            if (node.span === Number.POSITIVE_INFINITY) {
                panel.style.setProperty("--comic-column", "1 / -1");
            } else if (Number.isInteger(node.span) && node.span > 1) {
                panel.style.setProperty("--comic-column", `span ${node.span}`);
            }
            if (Number.isInteger(node.rowSpan) && node.rowSpan > 1) {
                panel.style.setProperty("--comic-row", `span ${node.rowSpan}`);
            }
            panel.append(...renderChildren(node.children, context));
            return panel;
        }
        case "svgAsset":
            return image(context, node.source, node.alt, node.focus);
        case "speech": {
            const speech = element(
                "figcaption",
                classNames("speech", node.at, node.tone)
            );
            speech.dataset.speaker = node.speaker;
            const label = element("b", "speech__speaker", node.speaker);
            speech.append(label);
            if (node.channel) {
                speech.append(element("i", "speech__channel", node.channel));
            }
            speech.append(document.createTextNode(node.text));
            return speech;
        }
        case "narration":
            return element(
                "div",
                classNames("comic-narration", node.at, node.tone),
                node.text
            );
        case "locationCard": {
            const card = element("div", classNames("comic-location", node.at));
            card.append(
                element("b", undefined, node.place),
                element("span", undefined, node.time)
            );
            return card;
        }
        case "caption":
            return element(
                "div",
                classNames("comic-caption", node.tone),
                node.text
            );
        case "readout": {
            const readout = element(
                "div",
                classNames("comic-readout", node.tone)
            );
            readout.append(
                ...node.lines.map((line) => element("span", undefined, line))
            );
            return readout;
        }
        case "transcript": {
            const block = element("div", "comic-transcript");
            if (node.title) {
                block.append(
                    element("p", "comic-transcript__title", node.title)
                );
            }
            for (const line of node.lines) {
                const row = element(
                    "div",
                    classNames("comic-transcript__line", line.tone)
                );
                row.append(
                    element("b", undefined, line.speaker),
                    element("span", undefined, line.text)
                );
                block.append(row);
            }
            return block;
        }
        case "portrait": {
            const card = element(
                "div",
                classNames("comic-portrait", node.tone)
            );
            card.append(
                image(context, node.source, node.alt),
                element("b", undefined, node.name),
                element("span", undefined, node.role)
            );
            return card;
        }
        case "divider":
            return element(
                "div",
                classNames("comic-divider", node.tone),
                node.text
            );
        case "hud": {
            const strip = element("div", classNames("comic-hud", node.at));
            for (const item of node.items) {
                const chip = element(
                    "span",
                    classNames("comic-hud__item", item.tone)
                );
                chip.append(
                    element("i", undefined, item.label),
                    element("b", undefined, item.value)
                );
                strip.append(chip);
            }
            return strip;
        }
        case "feed": {
            const block = element("div", "comic-feed");
            block.append(element("p", "comic-feed__source", node.source));
            block.append(
                ...node.lines.map((line) => element("span", undefined, line))
            );
            return block;
        }
        case "terminal": {
            const block = element("div", "comic-terminal");
            block.append(element("p", "comic-terminal__prompt", node.prompt));
            const list = element("ol");
            for (const option of node.options) {
                const item = element(
                    "li",
                    classNames("comic-terminal__option", option.tone)
                );
                item.append(
                    element("b", undefined, option.key),
                    element("span", undefined, option.label),
                    element("em", undefined, option.detail)
                );
                list.append(item);
            }
            block.append(list);
            return block;
        }
        case "inset": {
            const box = element("div", classNames("comic-inset", node.at));
            box.append(...renderChildren(node.children, context));
            return box;
        }
        case "svg": {
            const svg = svgElement("svg");
            svg.setAttribute("class", "comic-inline-svg");
            svg.setAttribute(
                "viewBox",
                node.viewBox.map((value) => number(value, "viewBox")).join(" ")
            );
            svg.setAttribute("role", "img");
            svg.setAttribute("aria-label", node.label);
            svg.append(...node.children.map(renderSvgNode));
            return svg;
        }
    }
}

function folio(number: number): HTMLParagraphElement {
    return element("p", "comic-page__folio", String(number).padStart(2, "0"));
}

function finish(
    article: HTMLElement,
    context: RenderPageContext,
    addFolio = true
): HTMLElement {
    if (addFolio) article.append(folio(context.number));
    article.id = context.id;
    article.dataset.page = "";
    return article;
}

export function renderComicPage(
    page: ComicPage,
    context: RenderPageContext
): HTMLElement {
    if (page.layout === "lettered" || page.layout === "panels") {
        const article = element("article", "comic-page comic-art-page");
        article.append(element("p", undefined, "Loading page..."));
        const composition =
            page.layout === "panels"
                ? composePanels(article, page, (source) =>
                      assetUrl(context, source)
                  )
                : letterPage(article, page, assetUrl(context, page.image));
        void composition.catch((error) => {
            article.dataset.layoutError = String(error);
            article.replaceChildren(
                element(
                    "p",
                    undefined,
                    "This page could not load. Its transcript remains available below."
                )
            );
        });
        return finish(article, context, false);
    }
    if (page.layout === "art") {
        const article = element("article", "comic-page comic-art-page");
        article.append(image(context, page.image, page.alt));
        return finish(article, context, false);
    }
    if (page.layout === "cover") {
        const article = element("article", "comic-page comic-cover");
        const shade = element("div", "comic-cover__shade");
        const copy = element("div", "comic-cover__copy");
        copy.append(element("p", undefined, page.eyebrow));
        const title = element("h1");
        title.append(document.createTextNode(page.title), element("br"));
        title.append(element("span", undefined, page.accent));
        copy.append(title);
        const tagline = element("blockquote");
        page.tagline.forEach((line, index) => {
            if (index > 0) tagline.append(element("br"));
            tagline.append(document.createTextNode(line));
        });
        copy.append(tagline);
        article.append(image(context, page.image, page.alt), shade, copy);
        return finish(article, context);
    }
    if (page.layout === "end") {
        const article = element("article", "comic-page comic-end");
        const signal = element("div", "comic-end__signal");
        signal.setAttribute("aria-hidden", "true");
        for (let index = 0; index < 7; index += 1) signal.append(element("i"));
        article.append(
            signal,
            element("p", "section__eyebrow", page.eyebrow),
            element("h2", undefined, page.title),
            element("p", undefined, page.body)
        );
        if (page.action) {
            const action = element("a", "btn btn--primary", page.action.label);
            action.href = `${context.basePath}${page.action.href.replace(/^\//, "")}`;
            article.append(action);
        }
        return finish(article, context);
    }
    if (page.layout === "bleed") {
        const article = element("article", "comic-page comic-bleed");
        const overlay = element("div", "comic-bleed__overlay");
        overlay.append(...renderChildren(page.children, context));
        article.append(
            image(context, page.image, page.alt, page.focus),
            element("div", "comic-bleed__shade"),
            overlay
        );
        return finish(article, context);
    }
    const article = element("article", "comic-page");
    article.append(...renderChildren(page.children, context));
    return finish(article, context);
}
