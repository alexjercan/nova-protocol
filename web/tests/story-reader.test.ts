// Behaviour checks for the comic reader. The reader is DOM-shaped but the site
// has no browser in test, so this file brings its OWN minimal document: enough
// element, event and history surface for `ComicPlayer` to run unmodified. The
// fake is deliberately small - it implements only what the reader reaches for,
// so a reader that starts reaching for something else fails loudly here rather
// than passing against a permissive stub. Run with `npm test`.
import { strict as assert } from "node:assert";
import { ComicPlayer, relativePageIndex } from "../src/story-reader";

assert.equal(relativePageIndex(1, 1, 5), 2, "next page advances");
assert.equal(relativePageIndex(0, -1, 5), 0, "previous stops at cover");
assert.equal(relativePageIndex(4, 1, 5), 4, "next stops at final page");
assert.equal(relativePageIndex(2, -1, 5), 1, "previous page retreats");
assert.equal(relativePageIndex(0, 1, 0), 0, "empty reader is stable");

// --- the smallest document the reader can run in -------------------------

const kebab = (key: string): string =>
    key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);

/** One selector of the shapes the reader uses: `#id`, `[attr]`, `[attr="v"]`, `tag`. */
function matchesOne(element: FakeElement, selector: string): boolean {
    if (selector.startsWith("#")) return element.id === selector.slice(1);
    const attribute = /^\[([a-z-]+)(?:="([^"]*)")?\]$/.exec(selector);
    if (attribute) {
        const value = element.getAttribute(attribute[1]);
        return (
            value !== null &&
            (attribute[2] === undefined || value === attribute[2])
        );
    }
    return element.tag === selector;
}

const matches = (element: FakeElement, selector: string): boolean =>
    selector.split(",").some((one) => matchesOne(element, one.trim()));

interface FakeEvent {
    type: string;
    target?: FakeElement;
    currentTarget?: FakeElement;
    preventDefault: () => void;
    [key: string]: unknown;
}

class FakeElement {
    id = "";
    hidden = false;
    disabled = false;
    href = "";
    src = "";
    target = "";
    rel = "";
    parent: FakeElement | null = null;
    children: FakeElement[] = [];
    modals = 0;
    focused = 0;
    readonly attributes = new Map<string, string>();
    readonly classes = new Set<string>();
    readonly listeners = new Map<string, ((event: FakeEvent) => void)[]>();
    readonly dataset: Record<string, string | undefined>;
    readonly classList: {
        toggle: (name: string, force?: boolean) => void;
        contains: (name: string) => boolean;
    };
    private text = "";

    constructor(readonly tag: string) {
        this.dataset = new Proxy(
            {},
            {
                get: (_t, key) =>
                    this.attributes.get(`data-${kebab(String(key))}`),
                set: (_t, key, value) => {
                    this.attributes.set(
                        `data-${kebab(String(key))}`,
                        String(value)
                    );
                    return true;
                },
            }
        );
        this.classList = {
            toggle: (name, force) => {
                const on = force ?? !this.classes.has(name);
                if (on) this.classes.add(name);
                else this.classes.delete(name);
            },
            contains: (name) => this.classes.has(name),
        };
    }

    get textContent(): string {
        if (this.tag === "#text") return this.text;
        return this.children.map((child) => child.textContent).join("");
    }
    set textContent(value: string) {
        if (this.tag === "#text") {
            this.text = String(value);
            return;
        }
        this.children = [];
        if (value !== "") this.append(String(value));
    }

    getAttribute(name: string): string | null {
        return this.attributes.get(name) ?? null;
    }
    setAttribute(name: string, value: string): void {
        this.attributes.set(name, String(value));
    }
    removeAttribute(name: string): void {
        this.attributes.delete(name);
    }

    append(...nodes: (FakeElement | string)[]): void {
        for (const node of nodes) {
            const child =
                typeof node === "string"
                    ? Object.assign(new FakeElement("#text"), {
                          textContent: node,
                      })
                    : node;
            child.parent = this;
            this.children.push(child);
        }
    }
    replaceChildren(...nodes: (FakeElement | string)[]): void {
        this.children = [];
        this.append(...nodes);
    }

    /** Descendants in document order - the order a real `querySelectorAll` gives. */
    descendants(): FakeElement[] {
        return this.children.flatMap((child) => [
            child,
            ...child.descendants(),
        ]);
    }
    querySelector(selector: string): FakeElement | null {
        return (
            this.descendants().find((node) => matches(node, selector)) ?? null
        );
    }
    querySelectorAll(selector: string): FakeElement[] {
        return this.descendants().filter((node) => matches(node, selector));
    }
    closest(selector: string): FakeElement | null {
        if (matches(this, selector)) return this;
        return this.parent?.closest(selector) ?? null;
    }

    addEventListener(type: string, handler: (event: FakeEvent) => void): void {
        const handlers = this.listeners.get(type) ?? [];
        handlers.push(handler);
        this.listeners.set(type, handlers);
    }
    dispatchEvent(event: { type: string }): boolean {
        this.fire(event.type);
        return true;
    }
    /** Deliver an event of `type` the way a browser would, with `extra` on it. */
    fire(type: string, extra: Record<string, unknown> = {}): void {
        const event: FakeEvent = {
            type,
            currentTarget: this,
            target: this,
            preventDefault: () => undefined,
            ...extra,
        };
        for (const handler of this.listeners.get(type) ?? []) handler(event);
    }

    showModal(): void {
        this.modals += 1;
        this.hidden = false;
    }
    close(): void {
        this.fire("close");
    }
    focus(): void {
        this.focused += 1;
    }
}

/** Build an element the way the page templates do: `tag#id[attr=value]`. */
function el(spec: string, ...children: FakeElement[]): FakeElement {
    const [head, ...attributes] = spec.split("[");
    const [tag, id] = head.split("#");
    const node = new FakeElement(tag);
    if (id) node.id = id;
    for (const attribute of attributes) {
        const [name, value] = attribute.replace("]", "").split("=");
        node.setAttribute(name, value ?? "");
    }
    node.append(...children);
    return node;
}

const warnings: string[] = [];
const hashes: string[] = [];
const media = { narrow: false };
const world = globalThis as unknown as Record<string, unknown>;
world.HTMLElement = FakeElement;
world.document = {
    createElement: (tag: string) => new FakeElement(tag),
};
world.window = {
    location: { hash: "" },
    history: {
        replaceState: (_s: unknown, _t: string, url: string) => {
            hashes.push(url);
            (world.window as { location: { hash: string } }).location.hash =
                url.slice(1);
        },
    },
    matchMedia: (query: string) => ({
        matches: query === "(max-width: 760px)" && media.narrow,
    }),
    addEventListener: () => undefined,
    setTimeout: (fn: () => void, ms: number) => setTimeout(fn, ms),
    clearTimeout: (handle: number) => clearTimeout(handle),
};
console.warn = (message: string) => warnings.push(message);

// --- a reader over three pages -------------------------------------------

const EXPORTS = JSON.stringify([
    { id: "2a", art: "/art/2a.svg", composed: "/art/2a.lettered.svg" },
]);

interface Reader {
    player: ComicPlayer;
    root: FakeElement;
    pages: FakeElement[];
    find: (selector: string) => FakeElement;
}

function reader(initialPage?: string): Reader {
    const pages = [1, 2, 3].map((number) => {
        const page = el(`section#page-${number}`);
        page.append(
            Object.assign(new FakeElement("img"), {
                src: `/art/page-${number}.png`,
            })
        );
        if (number === 2) page.dataset.panelExports = EXPORTS;
        if (number === 3) page.dataset.artUrl = "/art/page-3.full.png";
        return page;
    });
    const root = el(
        "div[data-contents-open=true]",
        el("div[data-page-viewport]"),
        el("p[data-page-transcript]"),
        el("span[data-page-current]"),
        el("a[data-page-art]"),
        el("button[data-art-toggle]"),
        el("button[data-page-previous]"),
        el("button[data-page-next]"),
        el("button[data-contents-toggle]"),
        el(
            "nav#comic-contents",
            ...pages.map((page) => el(`a[data-page-link=${page.id}]`))
        ),
        el("button[data-reader-dialog=panels][aria-controls=panels-dialog]"),
        el(
            "dialog#panels-dialog",
            el("button[data-dialog-close]"),
            el("span[data-dialog-page]"),
            el("ul[data-panel-image-list]")
        )
    );
    const player = new ComicPlayer({
        root: root as unknown as HTMLElement,
        pages: pages as unknown as HTMLElement[],
        transcripts: ["One.", "Two.", "Three."],
        initialPage,
    });
    return {
        player,
        root,
        pages,
        find: (selector) => {
            const found = root.querySelector(selector);
            assert.ok(found, `the fixture has no ${selector}`);
            return found;
        },
    };
}

const shown = (r: Reader): string =>
    r.pages
        .filter((page) => !page.hidden)
        .map((page) => page.id)
        .join(",");

// A fresh reader opens on page one, and page one alone is visible.
{
    const r = reader();
    assert.equal(shown(r), "page-1", "one page at a time");
    assert.equal(r.find("[data-page-transcript]").textContent, "One.");
    assert.equal(r.find("[data-page-current]").textContent, "01");
    assert.equal(
        r.find("[data-dialog-page]").textContent,
        "Page 01 / 3",
        "the dialog counts pages for a screen reader"
    );
    assert.equal(r.find("[data-page-previous]").disabled, true, "no page zero");
    assert.equal(r.find("[data-page-next]").disabled, false);
    assert.equal(hashes.length, 0, "opening page one writes no fragment");
}

// A fragment naming a page opens it, and leaves the fragment alone - the
// address bar already agrees with the reader.
{
    hashes.length = 0;
    const r = reader("page-3");
    assert.equal(shown(r), "page-3", "the fragment chooses the page");
    assert.equal(r.find("[data-page-next]").disabled, true, "no page four");
    assert.equal(hashes.length, 0, "a fragment that resolves is not rewritten");
}

// An unknown fragment - a bookmark from before a re-pagination - still opens
// the comic, but it SAYS SO and corrects the address bar to the page shown.
{
    hashes.length = 0;
    warnings.length = 0;
    const r = reader("page-99");
    assert.equal(shown(r), "page-1", "an unknown id falls back to page one");
    assert.deepEqual(
        warnings,
        ['No comic page "page-99"; opening the first page.'],
        "the fallback is announced"
    );
    assert.deepEqual(
        hashes,
        ["#page-1"],
        "the fragment is corrected to the page on screen"
    );
}

// Paging moves one page, clamps at both ends, and keeps the fragment, the
// counters, the buttons and the contents highlight in step.
{
    hashes.length = 0;
    const r = reader();
    r.player.next();
    assert.equal(shown(r), "page-2");
    assert.equal(r.find("[data-page-transcript]").textContent, "Two.");
    assert.equal(r.find("[data-page-current]").textContent, "02");
    const links = r.root.querySelectorAll("[data-page-link]");
    assert.deepEqual(
        links.map((link) => link.classList.contains("is-active")),
        [false, true, false],
        "the contents marks the page being read"
    );
    assert.equal(links[1].getAttribute("aria-current"), "page");
    assert.equal(links[0].getAttribute("aria-current"), null);
    r.player.next();
    r.player.next();
    assert.equal(shown(r), "page-3", "next stops at the last page");
    r.player.previous();
    r.player.previous();
    r.player.previous();
    assert.equal(shown(r), "page-1", "previous stops at the first page");
    assert.deepEqual(
        hashes,
        ["#page-2", "#page-3", "#page-3", "#page-2", "#page-1", "#page-1"],
        "every move records the page it landed on"
    );
}

// The art link follows the page: an explicit full-size export wins, a page
// without one falls back to the picture it shows.
{
    const r = reader();
    const art = r.find("[data-page-art]");
    assert.equal(art.href, "/art/page-1.png", "the picture is the fallback");
    assert.equal(art.hidden, false);
    r.player.next();
    r.player.next();
    assert.equal(art.href, "/art/page-3.full.png", "an export wins");
}

// The panel dialog is offered only on a page that exports panels, and its list
// is rebuilt only when the exports change.
{
    const r = reader();
    const button = r.find('[data-reader-dialog="panels"]');
    const list = r.find("[data-panel-image-list]");
    assert.equal(button.hidden, true, "page one exports no panels");
    r.player.next();
    assert.equal(button.hidden, false, "page two does");
    assert.equal(list.children.length, 1, "one entry per exported panel");
    assert.equal(
        list.children[0].textContent,
        "Panel 2A: Art SVG Lettered SVG "
    );
    assert.deepEqual(
        list.querySelectorAll("a").map((link) => link.href),
        ["/art/2a.svg", "/art/2a.lettered.svg"]
    );
    list.append(new FakeElement("li"));
    r.root.fire("comic:ready");
    assert.equal(
        list.children.length,
        2,
        "a re-render of the same page does not rebuild the list"
    );
    r.player.next();
    assert.equal(button.hidden, true, "page three exports no panels again");
    assert.equal(list.children.length, 0, "and its list is emptied");
}

// The dialog buttons open their dialog and hand focus back on close.
{
    const r = reader();
    const button = r.find('[data-reader-dialog="panels"]');
    const dialog = r.find("#panels-dialog");
    button.fire("click");
    assert.equal(dialog.modals, 1, "the button opens its dialog");
    r.find("[data-dialog-close]").fire("click");
    assert.equal(button.focused, 1, "closing returns focus to the opener");
}

// A wheel gesture turns ONE page however many events it fires.
{
    const r = reader();
    const viewport = r.find("[data-page-viewport]");
    for (let i = 0; i < 5; i++) viewport.fire("wheel", { deltaY: 30 });
    assert.equal(shown(r), "page-2", "a gesture is one page, not five");
    viewport.fire("wheel", { deltaY: 4 });
    assert.equal(shown(r), "page-2", "a nudge below the threshold is ignored");
    viewport.fire("wheel", { deltaY: 30, ctrlKey: true });
    assert.equal(shown(r), "page-2", "ctrl+wheel is left to the browser zoom");
}

// The keyboard pages, but not while the reader is inside a control.
{
    const r = reader();
    const viewport = r.find("[data-page-viewport]");
    viewport.fire("keydown", { key: "ArrowRight" });
    assert.equal(shown(r), "page-2");
    viewport.fire("keydown", { key: "ArrowLeft" });
    assert.equal(shown(r), "page-1");
    viewport.fire("keydown", { key: "Tab" });
    assert.equal(shown(r), "page-1", "an unbound key does nothing");
    viewport.fire("keydown", { key: " ", ctrlKey: true });
    assert.equal(shown(r), "page-1", "a shortcut is not a page turn");
    viewport.fire("keydown", {
        key: "ArrowRight",
        target: r.root.querySelectorAll("[data-page-link]")[0],
    });
    assert.equal(shown(r), "page-1", "arrows inside the contents navigate it");
}

// A swipe pages; a touch too short to be a swipe does not.
{
    const r = reader();
    const viewport = r.find("[data-page-viewport]");
    viewport.fire("touchstart", { touches: [{ clientY: 400 }] });
    viewport.fire("touchend", { changedTouches: [{ clientY: 300 }] });
    assert.equal(shown(r), "page-2", "an upward swipe turns the page");
    viewport.fire("touchstart", { touches: [{ clientY: 400 }] });
    viewport.fire("touchend", { changedTouches: [{ clientY: 380 }] });
    assert.equal(shown(r), "page-2", "20 px is a tap, not a swipe");
    viewport.fire("touchstart", { touches: [{ clientY: 400 }] });
    viewport.fire("touchcancel", {});
    viewport.fire("touchend", { changedTouches: [{ clientY: 100 }] });
    assert.equal(shown(r), "page-2", "a cancelled touch never lands");
}

// A page link jumps straight to its page.
{
    const r = reader();
    r.root.querySelectorAll("[data-page-link]")[2].fire("click");
    assert.equal(shown(r), "page-3", "the contents jumps");
}

// The contents pane opens with the reader on a wide screen and stays shut on a
// narrow one, where it would cover the page.
{
    const r = reader();
    assert.equal(r.root.dataset.contentsOpen, "true");
    assert.equal(r.find("#comic-contents").hidden, false);
    assert.equal(
        r.find("[data-contents-toggle]").getAttribute("aria-expanded"),
        "true"
    );
    r.find("[data-contents-toggle]").fire("click");
    assert.equal(r.find("#comic-contents").hidden, true, "the toggle shuts it");

    media.narrow = true;
    const narrow = reader();
    assert.equal(
        narrow.find("#comic-contents").hidden,
        true,
        "a phone opens the comic, not the contents"
    );
    narrow.find("[data-contents-toggle]").fire("click");
    assert.equal(narrow.find("#comic-contents").hidden, false);
    narrow.root.querySelectorAll("[data-page-link]")[1].fire("click");
    assert.equal(
        narrow.find("#comic-contents").hidden,
        true,
        "picking a page on a phone gets out of the way"
    );
    media.narrow = false;
}

// The art-only toggle is a pressed state the stylesheet reads off the root.
{
    const r = reader();
    const toggle = r.find("[data-art-toggle]");
    toggle.fire("click");
    assert.equal(r.root.dataset.artOnly, "true");
    assert.equal(toggle.getAttribute("aria-pressed"), "true");
    toggle.fire("click");
    assert.equal(r.root.dataset.artOnly, "false");
}

// A reader that cannot show what it is given refuses to be built. Half a
// reader - pages with no transcript beneath them - is worse than none.
{
    const good = reader();
    const build = (options: Record<string, unknown>) => () =>
        new ComicPlayer({
            root: good.root,
            pages: good.pages,
            transcripts: ["One.", "Two.", "Three."],
            ...options,
        } as unknown as ConstructorParameters<typeof ComicPlayer>[0]);
    const refused =
        /ComicPlayer needs a viewport, pages, and one transcript per page/;
    assert.throws(build({ pages: [] }), refused, "no pages");
    assert.throws(
        build({ transcripts: ["One."] }),
        refused,
        "too few transcripts"
    );
    assert.throws(
        build({ transcripts: ["One.", "  ", "Three."] }),
        refused,
        "a blank transcript"
    );
    assert.throws(build({ root: el("div") }), refused, "no viewport");
}

// eslint-disable-next-line no-console
console.log("story-reader.test.ts: all assertions passed");
