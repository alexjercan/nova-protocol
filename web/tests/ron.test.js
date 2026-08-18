// Tests for the semantic RON tokenizer in markdown.js (plain node, no tsc:
// markdown.js is CommonJS and the tokenizer has no DOM dependency).
const assert = require("assert");
const { highlightRon } = require("../markdown.js");

const escapeHtml = (s) =>
    s
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");

// Concatenating the span texts must reproduce the escaped source exactly:
// the tokenizer never drops, reorders, or double-escapes input.
const roundTrips = (code) => {
    const out = highlightRon(code);
    assert.strictEqual(
        out.replace(/<span class="ron-[a-z]+">|<\/span>/g, ""),
        escapeHtml(code),
        `round trip failed for: ${code}`
    );
    return out;
};

// Tagged types and variants are PascalCase idents.
let out = roundTrips('SpawnScenarioObject((base: (id: "rock_1")))');
assert.ok(out.includes('<span class="ron-type">SpawnScenarioObject</span>'));
assert.ok(out.includes('<span class="ron-key">base</span>'));
assert.ok(out.includes('<span class="ron-key">id</span>'));
assert.ok(out.includes('<span class="ron-string">&quot;rock_1&quot;</span>'));

// Option variants and bare unit variants are types too.
out = roundTrips("(name: OnStart, cap: Some(25.0), icon: None)");
assert.ok(out.includes('<span class="ron-type">OnStart</span>'));
assert.ok(out.includes('<span class="ron-type">Some</span>'));
assert.ok(out.includes('<span class="ron-type">None</span>'));
assert.ok(out.includes('<span class="ron-literal">25.0</span>'));

// A key is an ident followed by optional spaces and ":". A bare lowercase
// ident that is not a key stays unclassed.
out = roundTrips("other_type_name : x");
assert.ok(out.includes('<span class="ron-key">other_type_name</span>'));
assert.ok(!out.includes('">x</span>'));

// Numbers: negative, float, exponent, hex, underscores; true/false literals.
out = roundTrips("(-40.0, 1_000, 6.02e23, 0xFF, true, false)");
for (const lit of ["-40.0", "1_000", "6.02e23", "0xFF", "true", "false"]) {
    assert.ok(
        out.includes(`<span class="ron-literal">${lit}</span>`),
        `literal not tokenized: ${lit}`
    );
}

// Comments: // to end of line and /* block */; strings shield both.
out = roundTrips("a: 1, // trailing\n/* block */ b: 2");
assert.ok(out.includes('<span class="ron-comment">// trailing</span>'));
assert.ok(out.includes('<span class="ron-comment">/* block */</span>'));
out = roundTrips('(cubemap: "dep://base/textures/sky.png")');
assert.ok(out.includes("dep://base/textures/sky.png"));
assert.ok(!out.includes("ron-comment"));

// Strings: escapes stay inside the string token; HTML escapes exactly once.
out = roundTrips('(text: "say \\"hi\\" to <b> & co")');
assert.ok(
    out.includes(
        '<span class="ron-string">&quot;say \\&quot;hi\\&quot; to &lt;b&gt; &amp; co&quot;</span>'
    )
);
assert.ok(!out.includes("&amp;lt;"), "double-escaped HTML");
assert.ok(!out.includes("&amp;quot;"), "double-escaped quote");

// Punctuation runs are dimmed as one token.
out = roundTrips("Entity(())");
assert.ok(out.includes('<span class="ron-punct">(())</span>'));

// An unterminated string or block comment consumes to the end without error.
roundTrips('(text: "unterminated');
roundTrips("/* unterminated");

console.log("ron.test.js: all assertions passed");
