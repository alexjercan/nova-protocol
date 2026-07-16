import "./style.css";
import { initSite } from "./site";

initSite();

// Scroll-spy for the news post TOC: highlight the sidebar link whose section is
// currently in view. The TOC and its anchors are rendered at build time (see
// markdown.js newsPostShell), so this only wires up highlighting - the links
// already work with no JS. No-ops on the news index (no .news__toc there).
function initTocScrollSpy(): void {
    const toc = document.querySelector<HTMLElement>(".news__toc");
    if (!toc) return;

    const links = Array.from(
        toc.querySelectorAll<HTMLAnchorElement>(".news__toc-link")
    );
    if (!links.length) return;

    // Map each heading id -> its TOC link, and collect the heading elements.
    const linkById = new Map<string, HTMLAnchorElement>();
    const headings: HTMLElement[] = [];
    for (const link of links) {
        const id = decodeURIComponent(
            (link.getAttribute("href") || "").replace(/^#/, "")
        );
        const heading = id ? document.getElementById(id) : null;
        if (heading) {
            linkById.set(id, link);
            headings.push(heading);
        }
    }
    if (!headings.length) return;

    let activeId = "";
    const setActive = (id: string): void => {
        if (id === activeId) return;
        activeId = id;
        for (const link of links) link.classList.remove("is-active");
        const link = linkById.get(id);
        if (link) link.classList.add("is-active");
    };

    // Track which headings are above the reading line (a band near the top of
    // the viewport); the last one above it is the current section.
    const visible = new Set<string>();
    const observer = new IntersectionObserver(
        (entries) => {
            for (const entry of entries) {
                const id = entry.target.id;
                if (entry.isIntersecting) visible.add(id);
                else visible.delete(id);
            }
            // Pick the first heading (in document order) still in the band.
            const current = headings.find((h) => visible.has(h.id));
            if (current) setActive(current.id);
        },
        // A band from just under the sticky header down to ~60% of the viewport.
        { rootMargin: "-72px 0px -40% 0px", threshold: 0 }
    );
    for (const heading of headings) observer.observe(heading);

    // Seed the active link before any scroll happens.
    setActive(headings[0].id);
}

initTocScrollSpy();
