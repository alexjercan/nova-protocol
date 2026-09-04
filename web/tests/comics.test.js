const assert = require("node:assert/strict");
const { discoverComics, pagesOf } = require("../comic-build");

const comics = discoverComics();
const nova = comics.find((comic) => comic.path === "nova-protocol");
assert.ok(nova, "path-indexed Nova Protocol comic is discovered");
assert.equal(nova.chapters.length, 2, "manifest keeps two chapters");
assert.deepEqual(
    pagesOf(nova).map((page) => page.id),
    ["cover", "released", "no-fleet-code", "the-same-belt", "the-record"],
    "manifest order is the reading order"
);
assert.equal("slug" in nova, false, "directory path is the only comic index");

console.log("comics.test.js: all assertions passed");
