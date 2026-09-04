import { strict as assert } from "node:assert";
import { relativePageIndex } from "../src/story-reader";

assert.equal(relativePageIndex(1, 1, 5), 2, "next page advances");
assert.equal(relativePageIndex(0, -1, 5), 0, "previous stops at cover");
assert.equal(relativePageIndex(4, 1, 5), 4, "next stops at final page");
assert.equal(relativePageIndex(2, -1, 5), 1, "previous page retreats");
assert.equal(relativePageIndex(0, 1, 0), 0, "empty reader is stable");

// eslint-disable-next-line no-console
console.log("story-reader.test.ts: all assertions passed");
