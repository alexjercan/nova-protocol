// Progressive enhancement: every figure ships as a `.figure__placeholder`
// naming the asset to capture. Once that file exists in `assets/`, swap the
// placeholder for the real media; if it 404s (not captured yet) the placeholder
// stays. So a newly captured shot appears with no HTML edit - the capture
// pipelines drop the file into `web/src/assets/` (scripts/gen-web-screenshots.py
// for stills, scripts/capture-web-media.sh for webm loops) and it lights up.
// `base` is the deploy subpath (trailing slash), and the placeholder name is
// asset-root-relative (e.g. "assets/feature-gravity.png",
// "assets/loops/torpedo-blast.webm").
function upgradeFigures(base: string): void {
    const placeholders = document.querySelectorAll<HTMLElement>(
        ".figure__placeholder"
    );
    const reducedMotion = window.matchMedia(
        "(prefers-reduced-motion: reduce)"
    ).matches;
    const playbackObserver =
        !reducedMotion && "IntersectionObserver" in window
            ? new IntersectionObserver(
                  (entries) => {
                      for (const entry of entries) {
                          const video = entry.target as HTMLVideoElement;
                          if (entry.isIntersecting)
                              void video.play().catch(() => {});
                          else video.pause();
                      }
                  },
                  { threshold: 0.05 }
              )
            : null;

    const load = (placeholder: HTMLElement): void => {
        const name = placeholder
            .querySelector(".figure__placeholder-name")
            ?.textContent?.trim();
        if (!name) return;
        const note = placeholder
            .querySelector(".figure__placeholder-note")
            ?.textContent?.trim();
        const eager = placeholder.dataset.eager === "true";

        if (name.endsWith(".webm")) {
            const video = document.createElement("video");
            video.className = "figure__img";
            video.muted = true;
            video.loop = true;
            video.playsInline = true;
            video.setAttribute("aria-label", note ?? "");
            // Reduced motion never autoplays. The representative first frame
            // remains visible behind an explicit browser play control.
            if (reducedMotion) video.controls = true;
            else video.autoplay = true;
            video.preload = eager ? "auto" : "metadata";
            // Swap only once a frame is decodable, so a missing capture never
            // blanks the useful fallback carried by the placeholder.
            video.addEventListener("loadeddata", (): void => {
                placeholder.replaceWith(video);
                if (playbackObserver) playbackObserver.observe(video);
                else if (!reducedMotion) void video.play().catch(() => {});
            });
            video.src = base + name;
            return;
        }

        const img = new Image();
        img.className = "figure__img";
        img.alt = note ?? "";
        img.decoding = "async";
        img.onload = (): void => placeholder.replaceWith(img);
        img.src = base + name;
    };

    if (!("IntersectionObserver" in window)) {
        placeholders.forEach(load);
        return;
    }

    const loadObserver = new IntersectionObserver(
        (entries, observer) => {
            for (const entry of entries) {
                if (!entry.isIntersecting) continue;
                observer.unobserve(entry.target);
                load(entry.target as HTMLElement);
            }
        },
        { rootMargin: "500px 0px" }
    );
    placeholders.forEach((placeholder) => {
        if (placeholder.dataset.eager === "true") load(placeholder);
        else loadObserver.observe(placeholder);
    });
}

// Easter egg: the hidden UI-rework PoC chain (copied into the build at
// `/nova-menu/` -> `/nova-hud/` -> `/nova-os/`, see webpack.config.js) is opened
// by clicking the site brand/logo five times in quick succession; the click
// lands on the reworked main menu, which chains onward like the game
// (New Game -> HUD -> the NOVA OS button opens the CRT terminal).
// `registerHit` is the pure, testable core of
// that gesture: it keeps a rolling window of click timestamps, drops any older
// than `windowMs`, and reports `triggered` once `threshold` clicks land inside
// the window. Kept side-effect-free (no DOM, no `window`) so it can be exercised
// directly without a browser - the surrounding wiring in `initEasterEgg` is the
// only part that needs a runtime check. Returns a fresh array so callers reassign
// rather than mutate in place.
export function registerHit(
    hits: number[],
    now: number,
    windowMs: number,
    threshold: number
): { hits: number[]; triggered: boolean } {
    const recent = hits.filter((t) => now - t < windowMs).concat(now);
    if (recent.length >= threshold) return { hits: [], triggered: true };
    return { hits: recent, triggered: false };
}

const EGG_ROUTE = "nova-menu";
const EGG_THRESHOLD = 5;
const EGG_WINDOW_MS = 1500;

// Wire the brand-click gesture. The brand is a link to the site root (`root`),
// so this only ARMS when that root IS the current page - i.e. you are on the
// landing page, where re-clicking "home" is an otherwise-useless self-reload.
// There it swallows the click (preventDefault) and feeds it to `registerHit`;
// the fifth click within the window navigates to the secret route. On every
// other page the brand keeps its normal "go home" behavior untouched.
export function initEasterEgg(
    brand: HTMLAnchorElement | null,
    current: string,
    root: string
): void {
    if (!brand || root !== current) return;
    let hits: number[] = [];
    brand.addEventListener("click", (event: MouseEvent): void => {
        event.preventDefault();
        const result = registerHit(
            hits,
            event.timeStamp,
            EGG_WINDOW_MS,
            EGG_THRESHOLD
        );
        hits = result.hits;
        if (result.triggered) {
            window.location.href = `${root}/${EGG_ROUTE}/`;
        }
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

    // Arm the hidden NOVA OS terminal easter egg on the landing page.
    initEasterEgg(brand, current, root);

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
