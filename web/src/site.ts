// Progressive enhancement: every figure ships as a `.figure__placeholder`
// naming the screenshot to capture. Once that PNG exists in `assets/`, swap the
// placeholder for the real image; if it 404s (not captured yet) the placeholder
// stays. So a newly captured shot appears with no HTML edit - the capture
// pipeline drops the file into `web/src/assets/` (scripts/gen-web-screenshots.py)
// and it lights up. `base` is the deploy subpath (trailing slash), and the
// placeholder name is asset-root-relative (e.g. "assets/feature-gravity.png").
function upgradeFigures(base: string): void {
    const placeholders =
        document.querySelectorAll<HTMLElement>(".figure__placeholder");
    placeholders.forEach((placeholder) => {
        const name = placeholder
            .querySelector(".figure__placeholder-name")
            ?.textContent?.trim();
        if (!name) return;
        const note = placeholder
            .querySelector(".figure__placeholder-note")
            ?.textContent?.trim();

        const img = new Image();
        img.className = "figure__img";
        img.alt = note ?? "";
        img.decoding = "async";
        // NB: no `loading="lazy"` - the image is detached (not in the DOM) until
        // it loads, and a lazy detached image never starts loading, so `onload`
        // would never fire and the swap would never happen.
        // Only replace once the real image has decoded, so a missing asset never
        // blanks the placeholder.
        img.onload = (): void => placeholder.replaceWith(img);
        img.src = base + name;
    });
}

// Shared page bootstrap. Marks the current top-level nav link as active so the
// header reflects where you are, and upgrades figure placeholders to the real
// screenshots where the asset exists. Runs on every page (see the per-page
// entries).
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

    // Upgrade figures using the same basePath (trailing slash) the images need.
    upgradeFigures(root === "" ? "/" : root + "/");

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
