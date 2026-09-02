import "./style.css";
import { initSite } from "./site";
import { initWidgets } from "./widgets";
import { DOC_SECTIONS, DocPage, DocSection } from "./docs-manifest";

initSite();

// The whole doc chrome (for both /wiki/ and /create/) is rendered here from
// the manifest in docs-manifest.js, so the sidebar, search, tag chips,
// see-also and index all stay in sync with the generated pages. Each page
// supplies placeholder elements by id; we only fill the ones present, so the
// section indexes and sub-pages share this one script. Everything is scoped to
// the CURRENT section: the wiki sidebar lists only wiki pages, the create
// sidebar only creator pages.

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

// /wiki/... or /create/... -> the manifest section; anything else -> null.
function currentSection(base: string): DocSection | null {
    const path = window.location.pathname;
    const rel = path.startsWith(base) ? path.slice(base.length) : path;
    const root = rel.split("/").filter(Boolean)[0];
    return DOC_SECTIONS.find((s) => s.root === root) ?? null;
}

function currentSlug(base: string, section: DocSection): string | null {
    const path = window.location.pathname;
    const rel = path.startsWith(base) ? path.slice(base.length) : path;
    const segs = rel.split("/").filter(Boolean);
    // /wiki/<slug>/ -> slug (multi-segment for child pages, e.g.
    // /wiki/sections/hull/ -> "sections/hull"); the section root itself
    // (/wiki/, /create/) -> null (the index / landing).
    return segs[0] === section.root && segs[1] ? segs.slice(1).join("/") : null;
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

function pageUrl(base: string, section: DocSection, slug: string): string {
    return `${base}${section.root}/${slug}/`;
}

function bySlug(section: DocSection, slug: string): DocPage | undefined {
    return section.pages.find((p) => p.slug === slug);
}

function haystack(p: DocPage): string {
    return [p.title, p.summary, p.tags.join(" "), p.headings.join(" ")]
        .join(" ")
        .toLowerCase();
}

// ---- sidebar + search -----------------------------------------------------

// True when `slug` is the active page or one of its ancestors - the active
// trail. Drives both the active highlight and which subtrees stay expanded.
function onActiveTrail(
    section: DocSection,
    active: string | null,
    slug: string
): boolean {
    let page = active ? bySlug(section, active) : undefined;
    while (page) {
        if (page.slug === slug) return true;
        page = page.parent ? bySlug(section, page.parent) : undefined;
    }
    return false;
}

// One sidebar entry. Coming-soon pages have no HTML yet, so they render as a
// non-navigable span (a link would 404) - still searchable. A parent counts as
// active when the current page is one of its children.
function makeNavLink(
    p: DocPage,
    base: string,
    section: DocSection,
    active: string | null
): HTMLElement {
    const soon = !!p.comingSoon;
    const link = el(soon ? "span" : "a", "wiki-nav__link", p.title);
    if (!soon)
        (link as HTMLAnchorElement).href = pageUrl(base, section, p.slug);
    link.dataset.search = haystack(p);
    if (onActiveTrail(section, active, p.slug)) {
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
    section: DocSection,
    active: string | null
): void {
    const search = el("input", "wiki-search");
    search.type = "search";
    search.placeholder = section.searchPlaceholder;
    search.setAttribute("aria-label", section.searchPlaceholder);
    nav.appendChild(search);

    const home = el("a", "wiki-nav__home", section.homeLabel);
    home.href = `${base}${section.root}/`;
    if (active === null) {
        home.classList.add("is-active");
        home.setAttribute("aria-current", "page");
    }
    nav.appendChild(home);

    const groups: { heading: HTMLElement; items: HTMLElement[] }[] = [];

    for (const category of section.categories) {
        const pages = section.pages.filter((p) => p.category === category);
        if (pages.length === 0) continue;

        const group = el("div", "wiki-nav__group");
        const heading = el("p", "wiki-nav__cat", category);
        group.appendChild(heading);

        const items: HTMLElement[] = [];
        const appendPage = (
            page: DocPage,
            host: HTMLElement,
            depth: number
        ): void => {
            const link = makeNavLink(page, base, section, active);
            if (depth > 0) link.classList.add("wiki-nav__child");
            host.appendChild(link);
            items.push(link);

            const children = pages.filter((c) => c.parent === page.slug);
            if (children.length === 0) return;
            // A subtree is expanded while the reader is inside it (on the
            // parent or one of its descendants) and collapses everywhere else
            // to keep the tree shallow - unless the parent is marked expanded,
            // in which case its children are listed like any other group's
            // pages. Children stay in the DOM so an active search
            // (is-searching on the nav) can still reveal matches.
            const sub = el("div", "wiki-nav__sub");
            if (!page.expanded && !onActiveTrail(section, active, page.slug))
                sub.classList.add("is-collapsed");
            children.forEach((child) => appendPage(child, sub, depth + 1));
            host.appendChild(sub);
        };
        pages
            .filter((page) => !page.parent)
            .forEach((page) => appendPage(page, group, 0));
        nav.appendChild(group);
        groups.push({ heading, items });
    }

    const empty = el("p", "wiki-nav__empty", "No pages match.");
    empty.hidden = true;
    nav.appendChild(empty);

    const filter = (): void => {
        const q = search.value.trim().toLowerCase();
        const terms = q.split(/\s+/).filter(Boolean);
        // While a query is live, collapsed subtrees open (CSS keys on this
        // class) so a match inside one is never invisible.
        nav.classList.toggle("is-searching", terms.length > 0);
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

function renderTags(container: HTMLElement, page: DocPage): void {
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
    section: DocSection,
    page: DocPage
): void {
    const seen = new Set<string>([page.slug]);
    const picks: DocPage[] = [];

    const add = (slug: string): void => {
        if (seen.has(slug)) return;
        const p = bySlug(section, slug);
        // Only suggest pages you can actually open.
        if (!p || p.comingSoon) return;
        seen.add(slug);
        picks.push(p);
    };

    // Explicit related first, then pages that share a tag, capped.
    page.related.forEach(add);
    for (const other of section.pages) {
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
        link.href = pageUrl(base, section, p.slug);
        li.appendChild(link);
        li.appendChild(el("span", "wiki-seealso__sum", ` - ${p.summary}`));
        list.appendChild(li);
    }
    container.appendChild(list);
}

// ---- index page -----------------------------------------------------------

function renderIndex(
    container: HTMLElement,
    base: string,
    section: DocSection
): void {
    const topLevel = (category: string): DocPage[] =>
        section.pages.filter((p) => p.category === category && !p.parent);

    const card = (p: DocPage, child: boolean): HTMLElement => {
        const node = p.comingSoon
            ? el("div", "wiki-index__card is-soon")
            : el("a", "wiki-index__card");
        if (child) node.classList.add("wiki-index__card--child");
        if (!p.comingSoon) {
            (node as HTMLAnchorElement).href = pageUrl(base, section, p.slug);
        }
        const titleRow = el("div", "wiki-index__cardhead");
        if (child && p.icon) titleRow.appendChild(iconFrame(base, p));
        titleRow.appendChild(el("h3", "wiki-index__cardtitle", p.title));
        if (p.comingSoon) {
            titleRow.appendChild(el("span", "wiki-index__soon", "soon"));
        }
        node.appendChild(titleRow);
        node.appendChild(el("p", "wiki-index__cardsum", p.summary));
        return node;
    };

    for (const category of section.categories) {
        // Top-level pages, each followed by its children when it is marked
        // expanded; otherwise children live on their parent's overview page.
        const pages = topLevel(category);
        if (pages.length === 0) continue;

        container.appendChild(el("h3", "wiki-index__cat", category));
        const grid = el("div", "wiki-index__grid");
        for (const p of pages) {
            grid.appendChild(card(p, false));
            if (!p.expanded) continue;
            section.pages
                .filter((c) => c.parent === p.slug)
                .forEach((c) => grid.appendChild(card(c, true)));
        }
        container.appendChild(grid);
    }
}

// The hatched frame that stands in for a page's icon until the asset is
// captured; the real icon is dropped in on top only once it has loaded, so a
// not-yet-captured icon that 404s never flashes a broken-image glyph.
function iconFrame(base: string, page: DocPage): HTMLElement {
    const icon = el("span", "wiki-child__icon");
    if (!page.icon) return icon;
    icon.title = page.icon;
    const img = new Image();
    img.alt = page.title;
    img.decoding = "async";
    img.style.width = "100%";
    img.style.height = "100%";
    img.style.objectFit = "contain";
    img.onload = (): void => {
        icon.appendChild(img);
    };
    img.src = base + page.icon;
    return icon;
}

// ---- parent overview: child grid ------------------------------------------

// Renders a parent page's children as an icon+title grid (e.g. the five ship
// sections on the "Ship sections" page). The icon is a placeholder frame naming
// the asset to capture, until the real icon exists.
function renderChildrenGrid(
    container: HTMLElement,
    base: string,
    section: DocSection,
    parentSlug: string
): void {
    const kids = section.pages.filter((p) => p.parent === parentSlug);
    if (kids.length === 0) return;

    const grid = el("div", "wiki-children");
    for (const c of kids) {
        const card = el("a", "wiki-child");
        card.href = pageUrl(base, section, c.slug);

        card.appendChild(iconFrame(base, c));

        const body = el("span", "wiki-child__body");
        body.appendChild(el("span", "wiki-child__title", c.title));
        body.appendChild(el("span", "wiki-child__sum", c.summary));
        card.appendChild(body);

        grid.appendChild(card);
    }
    container.appendChild(grid);
}

// ---- mermaid diagrams -----------------------------------------------------

// Doc pages (rendered from markdown) may hold ```mermaid blocks, which
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
                // Phosphor skin: a diagram is drawn ON the screen, so its nodes
                // are green-tinted fills, not case-coloured boxes. Mermaid
                // derives further shades from these by colour maths, which
                // needs OPAQUE input - so these are the `--fill*` tints already
                // composited over the recess, not the rgba() tokens themselves.
                background: v("--screen-0", "#001304"),
                primaryColor: "#0c1f13",
                primaryBorderColor: v("--phosphor-dim", "#19a64f"),
                primaryTextColor: v("--text", "#b9ffc9"),
                lineColor: v("--phosphor-dim", "#19a64f"),
                secondaryColor: "#14301c",
                tertiaryColor: "#1d4526",
                // The `dark` base theme resolves flowchart node fill/stroke
                // from these, NOT from primaryColor, so setting only the
                // primaries above leaves the nodes grey.
                mainBkg: "#0c1f13",
                nodeBorder: v("--phosphor-dim", "#19a64f"),
                clusterBkg: "#04140a",
                clusterBorder: v("--phosphor-muted", "#0d6e35"),
                textColor: v("--text", "#b9ffc9"),
                nodeTextColor: v("--text", "#b9ffc9"),
                edgeLabelBackground: "#04140a",
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

// ---- folded prose (details.explain) ---------------------------------------

// Doc pages fold long detail into <details class="explain"> blocks (closed by
// default). A deep link or a search anchor must never land on hidden content:
// when the URL hash targets an element inside a closed fold, open the fold
// (nested folds included) and re-scroll, since the browser's own jump hit a
// hidden element. Runs on load and on every hashchange.
function revealHashTarget(): void {
    const hash = window.location.hash;
    if (hash.length < 2) return;
    let target: HTMLElement | null;
    try {
        target = document.getElementById(decodeURIComponent(hash.slice(1)));
    } catch {
        return;
    }
    if (!target) return;
    let fold = target.closest<HTMLDetailsElement>("details.explain");
    let opened = false;
    while (fold) {
        if (!fold.open) {
            fold.open = true;
            opened = true;
        }
        fold =
            fold.parentElement?.closest<HTMLDetailsElement>(
                "details.explain"
            ) ?? null;
    }
    if (opened) target.scrollIntoView();
}

// ---- drawer scroll persistence --------------------------------------------

// Each doc page is a full document, so the sidebar (#wiki-nav - its own
// overflow-y scroll container) re-renders at scrollTop 0 on every navigation and
// refresh. Persist its scroll position in sessionStorage (per-tab; survives
// same-tab navigations and reloads, clears on tab close) so the reader keeps
// their place. One key per section: the drawer is identical on every page of a
// section but differs between /wiki/ and /create/. Storage access is guarded
// for private mode / disabled storage, and on mobile the nav is not a scroll
// container so scrollTop stays 0 (a harmless no-op).
function persistNavScroll(nav: HTMLElement, section: DocSection): void {
    const key = `${section.root}-nav-scroll`;
    const read = (): string | null => {
        try {
            return sessionStorage.getItem(key);
        } catch {
            return null;
        }
    };
    const write = (v: number): void => {
        try {
            sessionStorage.setItem(key, String(v));
        } catch {
            // storage unavailable; scroll simply will not persist.
        }
    };

    const saved = read();
    if (saved !== null) nav.scrollTop = Number(saved) || 0;

    let queued = 0;
    nav.addEventListener("scroll", () => {
        if (queued) return;
        queued = requestAnimationFrame(() => {
            queued = 0;
            write(nav.scrollTop);
        });
    });
    // A navigation can fire between throttled saves; capture the final position.
    window.addEventListener("pagehide", () => write(nav.scrollTop));
}

// ---- boot -----------------------------------------------------------------

const base = basePath();
const section = currentSection(base);

void initMermaid();
// Interactive widgets declared in the rendered markdown (data-widget blocks).
initWidgets();
// Deep links into folded prose open the fold they land in.
revealHashTarget();
window.addEventListener("hashchange", revealHashTarget);

if (section) {
    const slug = currentSlug(base, section);

    const nav = document.getElementById("wiki-nav");
    if (nav) {
        renderSidebar(nav, base, section, slug);
        persistNavScroll(nav, section);
    }

    const indexHost = document.getElementById("wiki-index");
    if (indexHost) renderIndex(indexHost, base, section);

    if (slug) {
        const page = bySlug(section, slug);
        if (page) {
            const tagHost = document.getElementById("wiki-tags");
            if (tagHost) renderTags(tagHost, page);
            const seeAlsoHost = document.getElementById("wiki-seealso");
            if (seeAlsoHost) renderSeeAlso(seeAlsoHost, base, section, page);
        }
        // Overview pages (parents) render their children as a grid.
        const childrenHost = document.getElementById("wiki-children");
        if (childrenHost) renderChildrenGrid(childrenHost, base, section, slug);
    }
}
