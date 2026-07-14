// Landing-page courtesy warning for the "Play in browser" CTA.
//
// The in-browser game runs on bevy's WebGPU backend (task 20260714-233443), so
// browsers without WebGPU cannot launch it. The game page itself shows a full
// "WebGPU required" message to anyone who navigates in (build/web/webgpu-check.js
// is the authoritative gate, and it also covers people who deep-link straight to
// /play/). Here we only add a small heads-up under the Play button so the user is
// warned before clicking; the link stays clickable, since the destination now
// explains the requirement clearly.
//
// `"gpu" in navigator` is the feature test (the TS lib has no `navigator.gpu`
// type and the site pulls in no @webgpu/types, so the property is checked by
// name rather than accessed).
export function warnIfNoWebGpu(): void {
    if ("gpu" in navigator) {
        return;
    }

    const cta = document.querySelector(".hero__cta");
    if (!cta) {
        return;
    }

    // Idempotent: never insert the note twice.
    if (cta.nextElementSibling?.classList.contains("hero__cta-note")) {
        return;
    }

    const note = document.createElement("p");
    note.className = "hero__cta-note";
    note.textContent =
        "Playing in the browser needs a WebGPU browser - Chrome, Edge, Safari on macOS/iOS 26, or Firefox on Windows.";
    cta.insertAdjacentElement("afterend", note);
}
