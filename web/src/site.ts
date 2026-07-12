// Shared page bootstrap. Marks the current top-level nav link as active so the
// header reflects where you are. Runs on every page (see the per-page entries).
export function initSite(): void {
    const strip = (p: string): string => p.replace(/\/+$/, "");
    const pathOf = (a: HTMLAnchorElement): string =>
        strip(new URL(a.href, window.location.origin).pathname);

    const current = strip(window.location.pathname);

    // The site root is wherever the brand link points (basePath), e.g. "" at
    // local dev or "/nova-protocol" on project pages. It is only "active" on an
    // exact match, so it does not light up as a prefix of every other page.
    const brand = document.querySelector<HTMLAnchorElement>(
        ".site-header__brand"
    );
    const root = brand ? pathOf(brand) : "";

    const links = document.querySelectorAll<HTMLAnchorElement>(".site-nav a");
    links.forEach((link) => {
        if (link.classList.contains("is-cta")) return;
        const target = pathOf(link);
        const active =
            current === target ||
            (target !== root && current.startsWith(target + "/"));
        if (active) {
            link.setAttribute("aria-current", "page");
            link.style.color = "var(--text)";
        }
    });
}
