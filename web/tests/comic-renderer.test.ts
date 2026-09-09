import { strict as assert } from "node:assert";
import {
    bleedPage,
    circle,
    comicPage,
    artPage,
    chapterHeader,
    coverPage,
    endPage,
    grid,
    panel,
    path,
    speech,
    svg,
    svgAsset,
    widePanel,
} from "../src/comics/comic-page";
import {
    RenderPageContext,
    renderComicPage,
} from "../src/comics/comic-renderer";

/** Inline style as the renderer uses it: custom properties only. */
class FakeStyle {
    private readonly properties = new Map<string, string>();

    setProperty(name: string, value: string): void {
        this.properties.set(name, value);
    }

    get(name: string): string | undefined {
        return this.properties.get(name);
    }
}

/** The slice of the DOM the renderer touches, recorded for assertions. */
class FakeNode {
    readonly children: FakeNode[] = [];
    readonly attributes = new Map<string, string>();
    readonly dataset: Record<string, string> = {};
    readonly style = new FakeStyle();
    className = "";
    id = "";
    src = "";
    alt = "";
    href = "";
    private text: string | null;

    constructor(
        readonly tagName: string,
        readonly namespace: string | null = null,
        text: string | null = null
    ) {
        this.text = text;
    }

    get textContent(): string {
        return (
            (this.text ?? "") +
            this.children.map((child) => child.textContent).join("")
        );
    }

    set textContent(value: string) {
        this.children.length = 0;
        this.text = value;
    }

    setAttribute(name: string, value: string): void {
        if (name === "class") this.className = value;
        else this.attributes.set(name, value);
    }

    getAttribute(name: string): string | null {
        if (name === "class") return this.className;
        return this.attributes.get(name) ?? null;
    }

    append(...nodes: FakeNode[]): void {
        this.children.push(...nodes);
    }

    classes(): string[] {
        return this.className.split(" ").filter(Boolean);
    }
}

class FakeDocument {
    createElement(tag: string): FakeNode {
        return new FakeNode(tag);
    }

    createElementNS(namespace: string, tag: string): FakeNode {
        return new FakeNode(tag, namespace);
    }

    createTextNode(text: string): FakeNode {
        return new FakeNode("#text", null, text);
    }
}

(globalThis as unknown as { document: FakeDocument }).document =
    new FakeDocument();

function render(
    page: Parameters<typeof renderComicPage>[0],
    overrides: Partial<RenderPageContext> = {}
): FakeNode {
    const context: RenderPageContext = {
        id: "the-page",
        number: 7,
        comicPath: "example",
        basePath: "/nova/",
        ...overrides,
    };
    return renderComicPage(page, context) as unknown as FakeNode;
}

function all(root: FakeNode, match: (node: FakeNode) => boolean): FakeNode[] {
    const found = root.children.flatMap((child) => all(child, match));
    return match(root) ? [root, ...found] : found;
}

function one(root: FakeNode, match: (node: FakeNode) => boolean): FakeNode {
    const found = all(root, match);
    assert.equal(found.length, 1, "exactly one matching node");
    return found[0];
}

const byClass = (name: string) => (node: FakeNode) =>
    node.classes().includes(name);

// A finished comic page has reading metadata, not game availability.
{
    const article = render(
        comicPage(chapterHeader({ number: "03", title: "T", subtitle: "S" }))
    );
    assert.equal(article.tagName, "article");
    assert.deepEqual(article.classes(), ["comic-page"]);
    assert.equal(article.id, "the-page");
    assert.equal(article.dataset.page, "");
    assert.equal(article.dataset.state, undefined);
    assert.equal(all(article, byClass("comic-page__planned")).length, 0);
    assert.equal(one(article, byClass("comic-page__folio")).textContent, "07");
    const header = one(article, byClass("comic-page__header"));
    assert.ok(
        header.children[0].textContent.startsWith("Chapter 03"),
        "the eyebrow reads the chapter number first"
    );
    assert.equal(header.children[1].tagName, "h2");
    assert.equal(header.children[1].textContent, "T");
}

// An eyebrow override is printed as authored.
{
    const article = render(
        comicPage(
            chapterHeader({
                number: "00",
                title: "T",
                subtitle: "S",
                eyebrow: "Record",
            })
        )
    );
    assert.equal(all(article, byClass("comic-page__planned")).length, 0);
    assert.equal(article.dataset.state, undefined);
    assert.equal(
        one(article, byClass("comic-page__header")).children[0].textContent,
        "Record"
    );
}

// A complete authored page gets no shade, frame treatment, or extra folio.
{
    const article = render(
        artPage({ image: "page-01.svg", alt: "A full-color page." })
    );
    assert.deepEqual(article.classes(), ["comic-page", "comic-art-page"]);
    assert.equal(article.children.length, 1);
    assert.equal(article.children[0].tagName, "img");
    assert.equal(
        article.children[0].src,
        "/nova/assets/story/example/page-01.svg"
    );
    assert.equal(article.id, "the-page");
}

// Speech text is a text node, never parsed as markup.
{
    const article = render(
        comicPage(
            speech("Control", "<b>Nobody</b> answered.", {
                tone: "amber",
                at: "top-left",
                channel: "rec",
            })
        )
    );
    const box = one(article, byClass("speech"));
    assert.equal(box.tagName, "figcaption");
    assert.deepEqual(box.classes(), [
        "speech",
        "speech--top-left",
        "speech--amber",
    ]);
    assert.equal(box.dataset.speaker, "Control");
    assert.deepEqual(
        box.children.map((child) => child.tagName),
        ["b", "i", "#text"]
    );
    assert.equal(box.children[0].textContent, "Control");
    assert.equal(box.children[1].textContent, "rec");
    assert.equal(box.children[2].textContent, "<b>Nobody</b> answered.");
}

// Grid geometry reaches CSS as custom properties built from numbers.
{
    const article = render(
        comicPage(
            grid(
                { columns: 3, rows: [2, 1] },
                widePanel({ variant: "dark", frame: "amber" }),
                panel({ span: 2, rowSpan: 2, label: "The yard" })
            )
        )
    );
    const layout = one(article, byClass("comic-grid"));
    assert.equal(layout.style.get("--comic-columns"), "3");
    assert.equal(
        layout.style.get("--comic-rows"),
        "minmax(0, 2fr) minmax(0, 1fr)"
    );
    const [wide, tall] = layout.children;
    assert.equal(wide.tagName, "figure");
    assert.deepEqual(wide.classes(), [
        "comic-panel",
        "comic-panel--dark",
        "comic-panel--frame-amber",
    ]);
    assert.equal(wide.style.get("--comic-column"), "1 / -1");
    assert.equal(tall.style.get("--comic-column"), "span 2");
    assert.equal(tall.style.get("--comic-row"), "span 2");
    assert.equal(tall.getAttribute("aria-label"), "The yard");
}

// An "auto" row sizes to its content.
{
    const article = render(comicPage(grid({ rows: [1, "auto"] })));
    assert.equal(
        one(article, byClass("comic-grid")).style.get("--comic-rows"),
        "minmax(0, 1fr) auto"
    );
}

// Out-of-range grids are refused.
assert.throws(
    () => render(comicPage(grid({ columns: 5 }))),
    /columns must be 1 to 4/,
    "too many columns"
);
assert.throws(
    () => render(comicPage(grid({ rows: [1, 0] }))),
    /positive weights/,
    "a zero row weight"
);

// Assets resolve under the comic's own folder and nowhere else.
{
    const article = render(
        comicPage(
            svgAsset("boards/the-yard.svg", { alt: "The yard.", focus: "left" })
        )
    );
    const picture = one(article, (node) => node.tagName === "img");
    assert.equal(picture.src, "/nova/assets/story/example/boards/the-yard.svg");
    assert.equal(picture.alt, "The yard.");
    assert.equal(picture.dataset.focus, "left");
}
assert.throws(
    () => render(comicPage(svgAsset("../secret.svg", { alt: "x" }))),
    /not allowed/,
    "a parent path is refused"
);
assert.throws(
    () => render(comicPage(svgAsset("https://x.test/a.svg", { alt: "x" }))),
    /not allowed/,
    "a URL is refused"
);

// Inline SVG allows only path data and finite numbers.
{
    const article = render(
        comicPage(
            svg(
                { viewBox: [0, 0, 100, 50], label: "A dot." },
                circle({
                    center: [10.004, 20],
                    radius: 5,
                    tone: "amber",
                    fill: true,
                    width: 0,
                })
            )
        )
    );
    const drawing = one(article, byClass("comic-inline-svg"));
    assert.equal(drawing.namespace, "http://www.w3.org/2000/svg");
    assert.equal(drawing.getAttribute("viewBox"), "0 0 100 50");
    assert.equal(drawing.getAttribute("aria-label"), "A dot.");
    const dot = drawing.children[0];
    assert.equal(
        dot.getAttribute("cx"),
        "10",
        "numbers are rounded to two places"
    );
    assert.deepEqual(dot.classes(), [
        "comic-svg__shape",
        "comic-svg__shape--amber",
        "comic-svg__shape--unstroked",
        "comic-svg__shape--solid",
    ]);
}
assert.throws(
    () =>
        render(
            comicPage(
                svg(
                    { viewBox: [0, 0, 10, 10], label: "x" },
                    path({
                        d: "M0 0 L10 10 url(#x)",
                        tone: "default",
                        width: 1,
                        fill: false,
                    })
                )
            )
        ),
    /path data is not allowed/,
    "path data outside the command alphabet is refused"
);
assert.throws(
    () =>
        render(
            comicPage(
                svg(
                    { viewBox: [0, 0, 10, 10], label: "x" },
                    circle({
                        center: [Number.NaN, 0],
                        radius: 5,
                        tone: "default",
                        fill: false,
                        width: 1,
                    })
                )
            )
        ),
    /finite/,
    "a non-finite coordinate is refused"
);

// Bleed, cover, and end layouts.
{
    const article = render(
        bleedPage(
            { image: "board.svg", alt: "A board.", focus: "right" },
            speech("Captain", "Hold.")
        )
    );
    assert.deepEqual(article.classes(), ["comic-page", "comic-bleed"]);
    assert.equal(article.children[0].tagName, "img");
    assert.equal(article.children[0].dataset.focus, "right");
    assert.ok(byClass("comic-bleed__shade")(article.children[1]));
    const overlay = article.children[2];
    assert.ok(byClass("comic-bleed__overlay")(overlay));
    assert.equal(overlay.children.length, 1);
}
{
    const article = render(
        coverPage({
            image: "cover.svg",
            alt: "The cover.",
            eyebrow: "Campaign record",
            title: "First",
            accent: "Light",
            tagline: ["One line.", "Two lines."],
        })
    );
    assert.deepEqual(article.classes(), ["comic-page", "comic-cover"]);
    const title = one(article, (node) => node.tagName === "h1");
    assert.deepEqual(
        title.children.map((child) => child.tagName),
        ["#text", "br", "span"]
    );
    assert.equal(title.textContent, "FirstLight");
    const quote = one(article, (node) => node.tagName === "blockquote");
    assert.equal(quote.children.filter((c) => c.tagName === "br").length, 1);
}
{
    const article = render(
        endPage({
            eyebrow: "End",
            title: "Signal lost",
            body: "Closed.",
            action: { label: "Play", href: "/play/" },
        })
    );
    assert.deepEqual(article.classes(), ["comic-page", "comic-end"]);
    const signal = one(article, byClass("comic-end__signal"));
    assert.equal(signal.children.length, 7, "seven signal bars");
    const action = one(article, (node) => node.tagName === "a");
    assert.equal(action.href, "/nova/play/", "the action joins the base path");
    assert.equal(action.textContent, "Play");
}

// eslint-disable-next-line no-console
console.log("comic-renderer.test.ts: all assertions passed");

// A CRT panel is dressed by its variant class; the frame tone still applies.
{
    const article = render(
        comicPage(grid({}, panel({ variant: "crt", frame: "danger" })))
    );
    const screen = one(article, byClass("comic-panel"));
    assert.deepEqual(screen.classes(), [
        "comic-panel",
        "comic-panel--crt",
        "comic-panel--frame-danger",
    ]);
}
