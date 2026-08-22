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

/**
 * The landing's two-page wheel handoff. Returns the absolute scroll target for
 * this wheel event, or `null` when native scrolling should own it.
 *
 * Down at document zero enters the feature page in one gesture. Up anywhere in
 * the hero, or on an event that would cross from content into it, returns to
 * zero. Down from the feature boundary and ordinary movement inside the long
 * feature section remain native, so there is no snap point to drag against.
 */
export function landingHandoffTarget(
    scrollY: number,
    deltaY: number,
    featureY: number,
    tolerance = 2
): number | null {
    if (deltaY > 0 && scrollY <= tolerance) return featureY;
    if (deltaY >= 0 || scrollY <= tolerance) return null;
    if (scrollY <= featureY + tolerance) return 0;
    if (scrollY + deltaY <= featureY + tolerance) return 0;
    return null;
}

/** Convert WheelEvent delta modes to the pixel geometry used by scrollY. */
function wheelPixels(event: WheelEvent): number {
    if (event.deltaMode === WheelEvent.DOM_DELTA_LINE) return event.deltaY * 16;
    if (event.deltaMode === WheelEvent.DOM_DELTA_PAGE)
        return event.deltaY * window.innerHeight;
    return event.deltaY;
}

/**
 * Give `/` one explicit hero-to-features wheel handoff without persistent CSS
 * snap points. Narrow and reduced-motion clients retain ordinary scrolling.
 */
export function initLandingHandoff(): void {
    if (!document.documentElement.classList.contains("landing")) return;
    if (!window.matchMedia("(min-width: 901px)").matches) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const features = document.getElementById("features");
    const header = document.querySelector<HTMLElement>(".site-header");
    if (!features || !header) return;

    let handoffTarget: number | null = null;
    let releaseTimer: number | null = null;
    const release = (): void => {
        handoffTarget = null;
        if (releaseTimer !== null) window.clearTimeout(releaseTimer);
        releaseTimer = null;
    };
    window.addEventListener(
        "scroll",
        () => {
            if (
                handoffTarget !== null &&
                Math.abs(window.scrollY - handoffTarget) <= 2
            ) {
                release();
            }
        },
        { passive: true }
    );

    window.addEventListener(
        "wheel",
        (event) => {
            // Keep trackpad momentum from interrupting the short page change.
            // Ordinary wheel input resumes as soon as the target settles.
            if (handoffTarget !== null) {
                if (event.cancelable) event.preventDefault();
                return;
            }

            const featureY =
                features.getBoundingClientRect().top +
                window.scrollY -
                header.getBoundingClientRect().height;
            const target = landingHandoffTarget(
                window.scrollY,
                wheelPixels(event),
                featureY
            );
            if (target === null || !event.cancelable) return;
            event.preventDefault();
            handoffTarget = target;
            window.scrollTo({ top: target, behavior: "smooth" });
            // Fallback for a browser that cancels smooth scrolling without
            // delivering the target scroll event.
            releaseTimer = window.setTimeout(release, 1200);
        },
        { passive: false }
    );
}

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
