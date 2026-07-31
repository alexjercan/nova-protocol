// Theme parity: the site's `:root` must carry the SAME NOVA OS tokens the game
// does. Both sides mirror one origin - the `:root` block of
// `examples/ui/nova_ui_rework_poc.html` (which `crates/nova_ui/src/theme.rs`
// also mirrors) - so this test parses that file as an INDEPENDENT source rather
// than comparing against a constant copied in here. Run with `npm test`.
//
// It also guards the port itself: no legacy navy/cyan token survives, the type
// stack is terminal-first, every `var()` read resolves, the PHOSPHOR skin
// vocabulary is actually CONSUMED by the main surfaces, and the light-3D
// HARDWARE vocabulary is consumed NOWHERE. The site wears one skin of the two
// the PoC ships; the hardware tokens stay in `:root` only to keep the mirror in
// check (a) exact.
import { strict as assert } from "node:assert";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

// npm runs a script with cwd at the package dir (`web/`), so the repo root is
// one level up. Deriving it from cwd keeps this independent of where tsc drops
// the compiled output.
const WEB = process.cwd();
const REPO = join(WEB, "..");
const CSS_PATH = join(WEB, "src", "style.css");
const POC_PATH = join(REPO, "examples", "ui", "nova_ui_rework_poc.html");
for (const p of [CSS_PATH, POC_PATH])
    assert.ok(existsSync(p), `expected to read ${p} - run this from web/`);

const css = readFileSync(CSS_PATH, "utf8");
const poc = readFileSync(POC_PATH, "utf8");

/** Drop CSS block comments so they never masquerade as declarations. */
const stripComments = (s: string): string => s.replace(/\/\*[\s\S]*?\*\//g, "");

/** Collapse newlines/indent runs so a wrapped value compares equal to a flat one. */
const norm = (s: string): string => s.replace(/\s+/g, " ").trim();

/**
 * Parse the custom properties of the first `:root { ... }` block. Values here
 * never contain `{`, `}` or `;`, so a first-closing-brace scan is exact.
 */
function rootTokens(source: string, label: string): Map<string, string> {
    const text = stripComments(source);
    const open = text.indexOf(":root");
    assert.notEqual(open, -1, `${label}: no :root block`);
    const braceOpen = text.indexOf("{", open);
    const braceClose = text.indexOf("}", braceOpen);
    assert.notEqual(braceClose, -1, `${label}: unterminated :root block`);
    const body = text.slice(braceOpen + 1, braceClose);

    const tokens = new Map<string, string>();
    for (const decl of body.split(";")) {
        const at = decl.indexOf(":");
        if (at === -1) continue;
        const name = norm(decl.slice(0, at));
        if (!name.startsWith("--")) continue;
        tokens.set(name, norm(decl.slice(at + 1)));
    }
    assert.ok(tokens.size > 0, `${label}: :root parsed to zero tokens`);
    return tokens;
}

const siteTokens = rootTokens(css, "style.css");
const pocTokens = rootTokens(poc, "PoC");

// ---------------------------------------------------------------------------
// (a) Every token the PoC defines is defined here, with an identical value.
// ---------------------------------------------------------------------------
// The PoC names its typeface `--mono`; the site has carried a three-slot type
// system (`--font-display` / `--font-body` / `--font-mono`) since before the
// port, and check (c) below pins all three to the same JetBrains Mono stack.
// That naming difference is the ONLY sanctioned divergence.
const TOKEN_EXCEPTIONS = new Set(["--mono"]);

{
    const missing: string[] = [];
    const mismatched: string[] = [];
    for (const [name, pocValue] of pocTokens) {
        if (TOKEN_EXCEPTIONS.has(name)) continue;
        if (!siteTokens.has(name)) {
            missing.push(`${name} (PoC: ${pocValue})`);
            continue;
        }
        const siteValue = siteTokens.get(name);
        if (siteValue !== pocValue)
            mismatched.push(
                `${name}: site "${siteValue}" != PoC "${pocValue}"`
            );
    }
    assert.deepEqual(
        missing,
        [],
        `style.css :root is missing NOVA OS tokens:\n  ${missing.join("\n  ")}`
    );
    assert.deepEqual(
        mismatched,
        [],
        `style.css :root drifted from the PoC:\n  ${mismatched.join("\n  ")}`
    );
}

// ---------------------------------------------------------------------------
// (b) No legacy navy/cyan token name survives anywhere in the stylesheet.
// ---------------------------------------------------------------------------
{
    const RETIRED = [
        "--cyan",
        "--cyan-bright",
        "--cyan-deep",
        "--space-0",
        "--space-1",
        "--panel",
        "--panel-2",
        "--border-bright",
        "--amber-horizon",
    ];
    const survivors = RETIRED.filter((name) =>
        // Class/word boundary: `--cyan` must not match `--cyan-bright`, and
        // `--panel` must not match `--panel-radius`.
        new RegExp(`${name}(?![\\w-])`).test(stripComments(css))
    );
    assert.deepEqual(
        survivors,
        [],
        `retired navy/cyan tokens still in style.css: ${survivors.join(", ")}`
    );
}

// ---------------------------------------------------------------------------
// (c) Terminal-first typography: all three type slots are JetBrains Mono, and
//     the webfont request no longer pulls the proportional faces.
// ---------------------------------------------------------------------------
{
    for (const slot of ["--font-display", "--font-body", "--font-mono"]) {
        const value = siteTokens.get(slot);
        assert.ok(value, `${slot} is not defined in :root`);
        assert.match(
            value,
            /^"JetBrains Mono"\s*,/,
            `${slot} must name "JetBrains Mono" first, got: ${value}`
        );
    }

    const imports = stripComments(css).match(/@import[^;]*;/g) ?? [];
    assert.ok(
        imports.length > 0,
        "no @import found - webfonts must be fetched"
    );
    const fontImports = imports.join("\n");
    assert.ok(
        fontImports.includes("JetBrains+Mono"),
        "the @import must still fetch JetBrains Mono"
    );
    for (const retiredFace of ["Rajdhani", "Inter"])
        assert.ok(
            !fontImports.includes(retiredFace),
            `the @import still fetches the retired ${retiredFace} face`
        );
}

// ---------------------------------------------------------------------------
// (d) Every var(--x) read resolves to a :root definition (catches a rename that
//     stopped halfway through the file).
// ---------------------------------------------------------------------------
{
    const reads = new Set<string>();
    for (const m of stripComments(css).matchAll(/var\(\s*(--[\w-]+)/g))
        reads.add(m[1]);
    assert.ok(reads.size > 0, "no var() reads found - parser is broken");
    const dangling = [...reads].filter((name) => !siteTokens.has(name)).sort();
    assert.deepEqual(
        dangling,
        [],
        `var() reads with no :root definition: ${dangling.join(", ")}`
    );
}

/** Concatenate the bodies of every rule whose selector list matches. */
function rulesFor(match: RegExp): string {
    const text = stripComments(css);
    let bodies = "";
    // Rule bodies in this sheet contain no nested braces except inside
    // @media/@keyframes wrappers, whose inner rules this same scan reaches.
    for (const m of text.matchAll(/([^{}]+)\{([^{}]*)\}/g))
        if (match.test(m[1])) bodies += m[2] + "\n";
    return bodies;
}

/** A `var(--x)` READ of exactly this token, never of a longer name. */
const readOf = (token: string): RegExp =>
    new RegExp(`var\\(\\s*${token}(?![\\w-])`);

// ---------------------------------------------------------------------------
// (e) The PHOSPHOR vocabulary is CONSUMED, not merely declared: the main
//     surfaces each read at least one terminal token. Declaring the tokens
//     without using them would leave a green-but-styleless site.
// ---------------------------------------------------------------------------
{
    // CONSTRUCTION tokens only. `--phosphor`, `--screen*` and the rest of the
    // palette are deliberately absent: the retired hardware sheet read those
    // too, so including any of them makes this check pass on the very styling
    // it exists to reject.
    const PHOSPHOR = [
        "--edge",
        "--edge-soft",
        "--edge-faint",
        "--fill",
        "--fill-hot",
        "--fill-active",
        "--fill-amber",
        "--fill-amber-hot",
        "--panel-face",
        "--panel-face-hot",
        "--panel-shadow",
        "--recess",
        "--glow-text",
        "--glow-key",
        "--glow-text-amber",
    ];

    // `.btn` and its `--modifier` siblings, but never a different class.
    const family = (cls: string): RegExp =>
        new RegExp(`\\.${cls}(--[\\w-]+)?(?![\\w-])`);

    const SURFACES: Array<[string, RegExp]> = [
        ["buttons (.btn)", family("btn")],
        ["cards (.card)", family("card")],
        ["post cards (.post-card)", family("post-card")],
        ["wiki index cards (.wiki-index__card)", family("wiki-index__card")],
        ["code blocks (.prose pre)", /\.prose\s+pre(?![\w-])/],
    ];

    const bare: string[] = [];
    for (const [label, selector] of SURFACES) {
        const bodies = rulesFor(selector);
        assert.ok(bodies.length > 0, `no rules matched for ${label}`);
        if (!PHOSPHOR.some((t) => readOf(t).test(bodies))) bare.push(label);
    }
    assert.deepEqual(
        bare,
        [],
        `these surfaces read no phosphor-skin token: ${bare.join(", ")}`
    );
}

// ---------------------------------------------------------------------------
// (f) The light-3D HARDWARE vocabulary is consumed NOWHERE. The PoC ships two
//     skins and the site wears only the phosphor one; a bevelled face or a
//     moulded well anywhere means the hardware skin has crept back in.
//     `--drop` is exempt: the PoC's own PHOSPHOR panel reads it.
// ---------------------------------------------------------------------------
{
    const HARDWARE = [
        "--face",
        "--face-hot",
        "--rim",
        "--undercut",
        "--well",
        "--case-0",
        "--case-1",
        "--case-2",
        "--case-3",
        "--case-edge",
    ];
    // Everything after the `:root` block: `--face` legitimately reads
    // `--case-3` inside the mirrored definitions, so those are not rules.
    const text = stripComments(css);
    const afterRoot = text.slice(text.indexOf("}", text.indexOf(":root")) + 1);
    const consumed = HARDWARE.filter((t) => readOf(t).test(afterRoot));
    assert.deepEqual(
        consumed,
        [],
        `hardware-skin material is consumed outside :root: ${consumed.join(", ")}`
    );

    // `:root` is skipped above, and check (d) forces every read to resolve to a
    // `:root` definition - so `:root` is the one place an alias could smuggle
    // the material back in (`--btn-face: var(--face)`). Catch a site token that
    // both reads hardware AND is itself read by a rule.
    const aliases = [...siteTokens]
        .filter(([name]) => !HARDWARE.includes(name))
        .filter(([, value]) => HARDWARE.some((t) => readOf(t).test(value)))
        .filter(([name]) => readOf(name).test(afterRoot))
        .map(([name, value]) => `${name}: ${value}`);
    assert.deepEqual(
        aliases,
        [],
        `:root aliases hardware material into a consumed token:\n  ${aliases.join("\n  ")}`
    );
}

// eslint-disable-next-line no-console
console.log("theme.test.ts: all assertions passed");
