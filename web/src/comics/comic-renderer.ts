import { ComicNode, ComicPage, ComicTone, SvgNode } from "./comic-page";

export interface RenderPageContext {
    id: string;
    number: number;
    comicPath: string;
    basePath: string;
}

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

function toneClass(prefix: string, tone: ComicTone): string {
    return tone === "default" ? prefix : `${prefix} ${prefix}--${tone}`;
}

function assetUrl(context: RenderPageContext, source: string): string {
    return `${context.basePath}assets/story/${context.comicPath}/${source}`;
}

function renderSvgNode(node: SvgNode): SVGElement {
    const namespace = "http://www.w3.org/2000/svg";
    if (node.kind === "circle") {
        const shape = document.createElementNS(namespace, "circle");
        shape.setAttribute("cx", String(node.center[0]));
        shape.setAttribute("cy", String(node.center[1]));
        shape.setAttribute("r", String(node.radius));
        shape.setAttribute(
            "class",
            `comic-svg__shape comic-svg__shape--${node.tone}`
        );
        if (!node.fill) shape.setAttribute("fill", "none");
        return shape;
    }
    if (node.kind === "line") {
        const shape = document.createElementNS(namespace, "line");
        shape.setAttribute("x1", String(node.from[0]));
        shape.setAttribute("y1", String(node.from[1]));
        shape.setAttribute("x2", String(node.to[0]));
        shape.setAttribute("y2", String(node.to[1]));
        shape.setAttribute("stroke-width", String(node.width));
        shape.setAttribute(
            "class",
            `comic-svg__shape comic-svg__shape--${node.tone}`
        );
        return shape;
    }
    const text = document.createElementNS(namespace, "text");
    text.setAttribute("x", String(node.at[0]));
    text.setAttribute("y", String(node.at[1]));
    text.setAttribute("class", `comic-svg__text comic-svg__text--${node.tone}`);
    text.textContent = node.text;
    return text;
}

function renderNode(
    node: ComicNode,
    context: RenderPageContext
): HTMLElement | SVGElement {
    if (node.kind === "header") {
        const header = element("header", "comic-page__header");
        header.append(
            element("p", undefined, `Chapter ${node.number}`),
            element("h2", undefined, node.title),
            element("span", undefined, node.subtitle)
        );
        return header;
    }
    if (node.kind === "grid") {
        const grid = element("div", "comic-grid");
        grid.append(
            ...node.children.map((child) => renderNode(child, context))
        );
        return grid;
    }
    if (node.kind === "panel") {
        const classes = ["comic-panel"];
        if (node.size === "wide") classes.push("comic-panel--wide");
        if (node.variant !== "default")
            classes.push(`comic-panel--${node.variant}`);
        const panel = element("figure", classes.join(" "));
        if (node.label) panel.setAttribute("aria-label", node.label);
        panel.append(
            ...node.children.map((child) => renderNode(child, context))
        );
        return panel;
    }
    if (node.kind === "svgAsset") {
        const image = element("img");
        image.src = assetUrl(context, node.source);
        image.alt = node.alt;
        if (node.focus) image.dataset.focus = node.focus;
        return image;
    }
    if (node.kind === "speech") {
        const speech = element(
            "figcaption",
            toneClass("speech", node.tone),
            node.text
        );
        speech.dataset.speaker = node.speaker;
        if (node.speaker === "control") speech.classList.add("speech--control");
        return speech;
    }
    if (node.kind === "caption") {
        return element("div", toneClass("comic-caption", node.tone), node.text);
    }
    if (node.kind === "readout") {
        const readout = element("div", toneClass("comic-readout", node.tone));
        readout.append(
            ...node.lines.map((line) => element("span", undefined, line))
        );
        return readout;
    }
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("class", "comic-inline-svg");
    svg.setAttribute("viewBox", node.viewBox.join(" "));
    svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", node.label);
    svg.append(...node.children.map(renderSvgNode));
    return svg;
}

function folio(number: number): HTMLParagraphElement {
    return element("p", "comic-page__folio", String(number).padStart(2, "0"));
}

export function renderComicPage(
    page: ComicPage,
    context: RenderPageContext
): HTMLElement {
    if (page.layout === "cover") {
        const article = element("article", "comic-page comic-cover");
        const image = element("img");
        image.src = assetUrl(context, page.image);
        image.alt = page.alt;
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
        article.append(image, shade, copy, folio(context.number));
        article.id = context.id;
        article.dataset.page = "";
        return article;
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
        article.append(folio(context.number));
        article.id = context.id;
        article.dataset.page = "";
        return article;
    }
    const article = element("article", "comic-page");
    article.append(
        ...page.children.map((child) => renderNode(child, context)),
        folio(context.number)
    );
    article.id = context.id;
    article.dataset.page = "";
    return article;
}
