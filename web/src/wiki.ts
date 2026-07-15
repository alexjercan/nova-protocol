import "./style.css";
import { initSite } from "./site";
import { WIKI_PAGES, WIKI_CATEGORIES, WikiPage } from "./wiki-pages";

initSite();

// The whole wiki chrome is rendered here from the manifest, so the sidebar,
// search, tag chips, see-also and index all stay in sync with wiki-pages.ts.
// Each wiki page (index or sub-page) supplies placeholder elements by id; we
// only fill the ones present, so index and sub-pages share this one script.

// basePath is not available to bundled JS, so read it off the header brand link
// the same way site.ts does (works at "/" locally and "/nova-protocol/" on
// project pages).
function basePath(): string {
    const brand = document.querySelector<HTMLAnchorElement>(
        ".site-header__brand"
    );
    if (!brand) return "/";
    return new URL(brand.href).pathname.replace(/\/*$/, "/");
}

function currentSlug(base: string): string | null {
    const path = window.location.pathname;
    const rel = path.startsWith(base) ? path.slice(base.length) : path;
    const segs = rel.split("/").filter(Boolean);
    // /wiki/<slug>/ -> slug (multi-segment for child pages, e.g.
    // /wiki/sections/hull/ -> "sections/hull"); /wiki/ -> null (the index).
    return segs[0] === "wiki" && segs[1] ? segs.slice(1).join("/") : null;
}

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

function pageUrl(base: string, slug: string): string {
    return `${base}wiki/${slug}/`;
}

function bySlug(slug: string): WikiPage | undefined {
    return WIKI_PAGES.find((p) => p.slug === slug);
}

function haystack(p: WikiPage): string {
    return [p.title, p.summary, p.tags.join(" "), p.headings.join(" ")]
        .join(" ")
        .toLowerCase();
}

// ---- sidebar + search -----------------------------------------------------

// One sidebar entry. Coming-soon pages have no HTML yet, so they render as a
// non-navigable span (a link would 404) - still searchable. A parent counts as
// active when the current page is one of its children.
function makeNavLink(
    p: WikiPage,
    base: string,
    active: string | null
): HTMLElement {
    const soon = !!p.comingSoon;
    const link = el(soon ? "span" : "a", "wiki-nav__link", p.title);
    if (!soon) (link as HTMLAnchorElement).href = pageUrl(base, p.slug);
    link.dataset.search = haystack(p);
    const isActive =
        active === p.slug ||
        (active !== null && active.startsWith(p.slug + "/"));
    if (isActive) {
        link.classList.add("is-active");
        if (active === p.slug) link.setAttribute("aria-current", "page");
    }
    if (soon) {
        link.classList.add("is-soon");
        link.appendChild(el("span", "wiki-nav__soon", "soon"));
    }
    return link;
}

function renderSidebar(
    nav: HTMLElement,
    base: string,
    active: string | null
): void {
    const search = el("input", "wiki-search");
    search.type = "search";
    search.placeholder = "Search the wiki...";
    search.setAttribute("aria-label", "Search the wiki");
    nav.appendChild(search);

    const home = el("a", "wiki-nav__home", "Wiki index");
    home.href = `${base}wiki/`;
    if (active === null) {
        home.classList.add("is-active");
        home.setAttribute("aria-current", "page");
    }
    nav.appendChild(home);

    const groups: { heading: HTMLElement; items: HTMLElement[] }[] = [];

    for (const category of WIKI_CATEGORIES) {
        const pages = WIKI_PAGES.filter((p) => p.category === category);
        if (pages.length === 0) continue;

        const group = el("div", "wiki-nav__group");
        const heading = el("p", "wiki-nav__cat", category);
        group.appendChild(heading);

        const items: HTMLElement[] = [];
        // Top-level pages, then their children nested beneath.
        for (const p of pages.filter((x) => !x.parent)) {
            const link = makeNavLink(p, base, active);
            group.appendChild(link);
            items.push(link);

            const kids = WIKI_PAGES.filter((c) => c.parent === p.slug);
            if (kids.length > 0) {
                const sub = el("div", "wiki-nav__sub");
                for (const c of kids) {
                    const clink = makeNavLink(c, base, active);
                    clink.classList.add("wiki-nav__child");
                    sub.appendChild(clink);
                    items.push(clink);
                }
                group.appendChild(sub);
            }
        }
        nav.appendChild(group);
        groups.push({ heading, items });
    }

    const empty = el("p", "wiki-nav__empty", "No pages match.");
    empty.hidden = true;
    nav.appendChild(empty);

    const filter = (): void => {
        const q = search.value.trim().toLowerCase();
        const terms = q.split(/\s+/).filter(Boolean);
        let anyVisible = false;
        for (const { heading, items } of groups) {
            let groupVisible = false;
            for (const link of items) {
                const hay = link.dataset.search ?? "";
                const match =
                    terms.length === 0 || terms.every((t) => hay.includes(t));
                link.hidden = !match;
                if (match) groupVisible = true;
            }
            heading.hidden = !groupVisible;
            if (groupVisible) anyVisible = true;
        }
        empty.hidden = anyVisible;
    };
    search.addEventListener("input", filter);
}

// ---- current-page tag chips ----------------------------------------------

function renderTags(container: HTMLElement, page: WikiPage): void {
    if (page.tags.length === 0) return;
    for (const tag of page.tags) {
        const chip = el("span", "wiki-tag", tag);
        container.appendChild(chip);
    }
}

// ---- see also -------------------------------------------------------------

function renderSeeAlso(
    container: HTMLElement,
    base: string,
    page: WikiPage
): void {
    const seen = new Set<string>([page.slug]);
    const picks: WikiPage[] = [];

    const add = (slug: string): void => {
        if (seen.has(slug)) return;
        const p = bySlug(slug);
        // Only suggest pages you can actually open.
        if (!p || p.comingSoon) return;
        seen.add(slug);
        picks.push(p);
    };

    // Explicit related first, then pages that share a tag, capped.
    page.related.forEach(add);
    for (const other of WIKI_PAGES) {
        if (picks.length >= 5) break;
        if (other.tags.some((t) => page.tags.includes(t))) add(other.slug);
    }
    if (picks.length === 0) return;

    const heading = el("h2", "wiki-seealso__title", "See also");
    container.appendChild(heading);
    const list = el("ul", "wiki-seealso__list");
    for (const p of picks.slice(0, 5)) {
        const li = el("li");
        const link = el("a", undefined, p.title);
        link.href = pageUrl(base, p.slug);
        li.appendChild(link);
        li.appendChild(el("span", "wiki-seealso__sum", ` - ${p.summary}`));
        list.appendChild(li);
    }
    container.appendChild(list);
}

// ---- index page -----------------------------------------------------------

function renderIndex(container: HTMLElement, base: string): void {
    for (const category of WIKI_CATEGORIES) {
        // Top-level pages only - children live on their parent's overview page.
        const pages = WIKI_PAGES.filter(
            (p) => p.category === category && !p.parent
        );
        if (pages.length === 0) continue;

        container.appendChild(el("h2", "wiki-index__cat", category));
        const grid = el("div", "wiki-index__grid");
        for (const p of pages) {
            const card = p.comingSoon
                ? el("div", "wiki-index__card is-soon")
                : el("a", "wiki-index__card");
            if (!p.comingSoon) {
                (card as HTMLAnchorElement).href = pageUrl(base, p.slug);
            }
            const titleRow = el("div", "wiki-index__cardhead");
            titleRow.appendChild(el("h3", "wiki-index__cardtitle", p.title));
            if (p.comingSoon) {
                titleRow.appendChild(el("span", "wiki-index__soon", "soon"));
            }
            card.appendChild(titleRow);
            card.appendChild(el("p", "wiki-index__cardsum", p.summary));
            grid.appendChild(card);
        }
        container.appendChild(grid);
    }
}

// ---- parent overview: child grid ------------------------------------------

// Renders a parent page's children as an icon+title grid (e.g. the five ship
// sections on the "Ship sections" page). The icon is a placeholder frame naming
// the asset to capture, until the real icon exists.
function renderChildrenGrid(
    container: HTMLElement,
    base: string,
    parentSlug: string
): void {
    const kids = WIKI_PAGES.filter((p) => p.parent === parentSlug);
    if (kids.length === 0) return;

    const grid = el("div", "wiki-children");
    for (const c of kids) {
        const card = el("a", "wiki-child");
        card.href = pageUrl(base, c.slug);

        const icon = el("span", "wiki-child__icon");
        if (c.icon) {
            // The hatched span is the placeholder; drop the real icon in on top
            // only once it has loaded (mirrors upgradeFigures in site.ts). The
            // img is built detached and appended in onload, so a not-yet-captured
            // icon that 404s never flashes a broken-image glyph - the frame stays.
            icon.title = c.icon;
            const img = new Image();
            img.alt = c.title;
            img.decoding = "async";
            img.style.width = "100%";
            img.style.height = "100%";
            img.style.objectFit = "contain";
            img.onload = (): void => {
                icon.appendChild(img);
            };
            img.src = base + c.icon;
        }
        card.appendChild(icon);

        const body = el("span", "wiki-child__body");
        body.appendChild(el("span", "wiki-child__title", c.title));
        body.appendChild(el("span", "wiki-child__sum", c.summary));
        card.appendChild(body);

        grid.appendChild(card);
    }
    container.appendChild(grid);
}

// ---- mermaid diagrams -----------------------------------------------------

// Developer doc pages (rendered from markdown) may hold ```mermaid blocks, which
// markdown.js emits as <pre class="mermaid">. Mermaid needs the DOM, so it runs
// client-side here - and only when a diagram is present, so its (large) bundle is
// dynamically imported and never weighs on a page without one. Themed to the
// site palette so diagrams match the sharp house style.
async function initMermaid(): Promise<void> {
    const blocks = document.querySelectorAll<HTMLElement>(".mermaid");
    if (blocks.length === 0) return;
    try {
        const { default: mermaid } = await import("mermaid");
        const css = getComputedStyle(document.documentElement);
        const v = (name: string, fallback: string): string =>
            css.getPropertyValue(name).trim() || fallback;
        mermaid.initialize({
            startOnLoad: false,
            securityLevel: "strict",
            theme: "dark",
            fontFamily: v("--font-mono", "monospace"),
            themeVariables: {
                background: v("--panel", "#141a2e"),
                primaryColor: v("--panel-2", "#0f1424"),
                primaryBorderColor: v("--border-bright", "#3a4d7a"),
                primaryTextColor: v("--text", "#e8eefc"),
                lineColor: v("--cyan-deep", "#2a9fd6"),
                secondaryColor: v("--panel", "#141a2e"),
                tertiaryColor: v("--panel", "#141a2e"),
            },
        });
        await mermaid.run({ querySelector: ".mermaid" });
    } catch {
        // If the mermaid chunk fails to load or a diagram fails to parse, reveal
        // the raw source instead of leaving it invisible (the CSS hides the
        // pre until it is processed). Mark every still-unprocessed block.
        blocks.forEach((b) => {
            if (b.dataset.processed !== "true")
                b.classList.add("mermaid--failed");
        });
    }
}

// ---- boot -----------------------------------------------------------------

const base = basePath();
const slug = currentSlug(base);

void initMermaid();

const nav = document.getElementById("wiki-nav");
if (nav) renderSidebar(nav, base, slug);

const indexHost = document.getElementById("wiki-index");
if (indexHost) renderIndex(indexHost, base);

if (slug) {
    const page = bySlug(slug);
    if (page) {
        const tagHost = document.getElementById("wiki-tags");
        if (tagHost) renderTags(tagHost, page);
        const seeAlsoHost = document.getElementById("wiki-seealso");
        if (seeAlsoHost) renderSeeAlso(seeAlsoHost, base, page);
    }
    // Overview pages (parents) render their children as a grid.
    const childrenHost = document.getElementById("wiki-children");
    if (childrenHost) renderChildrenGrid(childrenHost, base, slug);
}
