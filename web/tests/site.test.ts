// Pure-logic checks for the easter-egg click accumulator. Run with `npm test`
// (compiles this file + src/site to CommonJS and runs it under node). CI stays
// build-only - see the `ci-skips-client-render` lesson; the DOM wiring in
// initEasterEgg is verified separately against the served build.
import { strict as assert } from "node:assert";
import { registerHit, initEasterEgg } from "../src/site";

const WINDOW = 1500;
const THRESHOLD = 5;

// Five clicks inside the window trigger on the fifth and reset the buffer.
{
    let hits: number[] = [];
    const times = [0, 100, 200, 300, 400];
    const results = times.map((t) => {
        const r = registerHit(hits, t, WINDOW, THRESHOLD);
        hits = r.hits;
        return r.triggered;
    });
    assert.deepEqual(
        results,
        [false, false, false, false, true],
        "fifth fires"
    );
    assert.deepEqual(hits, [], "buffer resets after trigger");
}

// A stale click ages out of the window, so the run never reaches threshold.
{
    let hits: number[] = [];
    // t=0 falls outside the window once we reach t=1600, so by the fifth recent
    // click only four are ever in range together.
    const times = [0, 1600, 1700, 1800, 1900];
    const triggered = times.map((t) => {
        const r = registerHit(hits, t, WINDOW, THRESHOLD);
        hits = r.hits;
        return r.triggered;
    });
    assert.deepEqual(
        triggered,
        [false, false, false, false, false],
        "stale ages out"
    );
    assert.deepEqual(hits, [1600, 1700, 1800, 1900], "only in-window kept");
}

// A slow drip (one click per window) never accumulates.
{
    let hits: number[] = [];
    for (let i = 0; i < 10; i++) {
        const r = registerHit(hits, i * WINDOW, WINDOW, THRESHOLD);
        hits = r.hits;
        assert.equal(r.triggered, false, "slow drip never triggers");
        assert.equal(hits.length, 1, "each click ages out the previous");
    }
}

// --- initEasterEgg wiring, driven against a fake DOM ---------------------------
// A minimal anchor that records its click handler, plus a monotonic clock and a
// fake `window.location` so we can assert the navigation the browser would do.
interface FakeBrand {
    handler:
        ((e: { timeStamp: number; preventDefault: () => void }) => void) | null;
    prevented: number;
    addEventListener: (type: string, h: FakeBrand["handler"]) => void;
    clickAt: (t: number) => void;
}
function makeBrand(): FakeBrand {
    // Methods close over `brand` rather than `this` so the type-checked lint
    // sees concrete types (a bare `this` here would be `any`).
    const brand: FakeBrand = {
        handler: null,
        prevented: 0,
        addEventListener(type, h) {
            if (type === "click") brand.handler = h;
        },
        clickAt(t) {
            brand.handler?.({
                timeStamp: t,
                preventDefault: () => (brand.prevented += 1),
            });
        },
    };
    return brand;
}

function withFakeWindow<T>(fn: (loc: { href: string }) => T): T {
    const location = { href: "" };
    const g = globalThis as unknown as {
        window?: { location: { href: string } };
    };
    const prev = g.window;
    g.window = { location };
    try {
        return fn(location);
    } finally {
        g.window = prev;
    }
}

// Armed (on the landing page, so root === current): five clicks navigate to the
// secret route, every click is swallowed, a sub-threshold burst does nothing.
// Local dev has an empty basePath, so root and current are both "".
withFakeWindow((location) => {
    const brand = makeBrand();
    initEasterEgg(brand as unknown as HTMLAnchorElement, "", "");
    for (let i = 0; i < 4; i++) brand.clickAt(i * 100);
    assert.equal(location.href, "", "four clicks do not navigate");
    brand.clickAt(400);
    assert.equal(location.href, "/nova-menu/", "fifth click opens the route");
    assert.equal(brand.prevented, 5, "every armed click is swallowed");
});

// Armed under a deploy subpath: the route is basePath-aware.
withFakeWindow((location) => {
    const brand = makeBrand();
    initEasterEgg(
        brand as unknown as HTMLAnchorElement,
        "/nova-protocol",
        "/nova-protocol"
    );
    for (let i = 0; i < 5; i++) brand.clickAt(i * 100);
    assert.equal(
        location.href,
        "/nova-protocol/nova-menu/",
        "basePath is honored"
    );
});

// Not armed (root is not the current page - e.g. an inner page): no listener is
// even attached, so normal brand navigation is left untouched.
withFakeWindow((location) => {
    const brand = makeBrand();
    initEasterEgg(brand as unknown as HTMLAnchorElement, "/wiki", "/home");
    assert.equal(brand.handler, null, "no click handler on inner pages");
    for (let i = 0; i < 6; i++) brand.clickAt(i * 100);
    assert.equal(location.href, "", "off-page brand never triggers");
    assert.equal(brand.prevented, 0, "off-page clicks are not swallowed");
});

// A null brand (header absent) is a no-op, not a crash.
initEasterEgg(null, "", "");

// eslint-disable-next-line no-console
console.log("site.test.ts: all assertions passed");
