// Unit test for the shipped webgpu-check.js gate. Runs the actual file in a vm
// context with a stubbed DOM, so it verifies the code that trunk inlines into
// the game index, not a copy. Run: node --test build/web/webgpu-check.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import vm from "node:vm";

const here = dirname(fileURLToPath(import.meta.url));
const source = readFileSync(join(here, "webgpu-check.js"), "utf8");

// A minimal `.game-container` stub that records what the gate writes into it.
function fakeContainer() {
    return { innerHTML: "" };
}

// Run the gate source against a stubbed navigator/document, then flush the
// microtask/timer queue so the async requestAdapter() branch resolves.
async function run({ navigator, container }) {
    const document = {
        querySelector(sel) {
            assert.equal(sel, ".game-container");
            return container;
        },
    };
    vm.runInNewContext(source, { navigator, document });
    await new Promise((resolve) => setTimeout(resolve, 0));
}

const adapterGpu = { requestAdapter: () => Promise.resolve({}) };
const noAdapterGpu = { requestAdapter: () => Promise.resolve(null) };
const throwingGpu = {
    requestAdapter: () => Promise.reject(new Error("no backend")),
};

test("WebGPU absent -> fallback shown synchronously", async () => {
    const container = fakeContainer();
    await run({ navigator: {}, container });
    assert.match(container.innerHTML, /WebGPU required/);
    assert.match(container.innerHTML, /href="\.\.\/"/, "keeps the /play/ -> site back link");
    assert.match(container.innerHTML, /webgpu-fallback/);
});

test("WebGPU present with a real adapter -> canvas left untouched", async () => {
    const container = fakeContainer();
    await run({ navigator: { gpu: adapterGpu }, container });
    assert.equal(container.innerHTML, "", "a working adapter must not trigger the fallback");
});

test("WebGPU present but no adapter -> fallback shown (the Firefox/Linux case)", async () => {
    const container = fakeContainer();
    await run({ navigator: { gpu: noAdapterGpu }, container });
    assert.match(container.innerHTML, /WebGPU required/);
});

test("WebGPU present but requestAdapter rejects -> fallback shown", async () => {
    const container = fakeContainer();
    await run({ navigator: { gpu: throwingGpu }, container });
    assert.match(container.innerHTML, /WebGPU required/);
});

test("WebGPU absent and no container -> does not throw", async () => {
    await run({ navigator: {}, container: null });
});
