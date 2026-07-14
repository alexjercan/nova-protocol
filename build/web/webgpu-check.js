// WebGPU gate for the in-browser game build.
//
// The web build ships bevy's `webgpu` render backend (bevy_hanabi's
// compute-shader particles require it - see task 20260714-233438). A `webgpu`
// build cannot initialize its renderer on a browser without working WebGPU: it
// panics at surface creation and leaves a dead black canvas. Roughly 15% of
// browsers as of 2026 (Firefox on Linux/Android/Intel-Mac, older OS/browser)
// are affected.
//
// This script is inlined into the game's index.html BEFORE trunk's wasm
// bootstrap. Trunk emits its loader as a `<script type="module">`, which is
// deferred (runs after the HTML is parsed), while this plain inline script runs
// synchronously during parsing. Placed after `.game-container` in the body, it
// runs while the container exists but before bevy ever boots.
//
// Two checks, because "has WebGPU" is not one boolean:
//   1. Synchronous: `navigator.gpu` absent -> WebGPU is off entirely (default
//      Firefox on Linux, older browsers). Show the message immediately, with no
//      black flash, and bevy's init then finds no `#bevy` canvas (WindowPlugin
//      binds `canvas: Some("#bevy")`) and fails quietly.
//   2. Asynchronous: `navigator.gpu` present but `requestAdapter()` yields no
//      adapter (flag half-enabled, unsupported GPU/driver, Firefox-on-Linux with
//      the pref flipped but no backend). Presence alone would sail past this and
//      let bevy panic. The probe races bevy's own init; whichever loses, the
//      user still ends on the message instead of a crashed canvas. On a working
//      WebGPU browser the adapter resolves and nothing is shown.
(function () {
    "use strict";

    function showFallback() {
        var container = document.querySelector(".game-container");
        if (!container) {
            return;
        }
        container.innerHTML =
            '<div class="webgpu-fallback">' +
            "<h1>WebGPU required</h1>" +
            "<p>Nova Protocol runs in the browser on <strong>WebGPU</strong>, which " +
            "this browser does not have working.</p>" +
            "<p>Try a recent <strong>Chrome</strong> or <strong>Edge</strong>, " +
            "<strong>Safari</strong> on macOS/iOS 26, or <strong>Firefox</strong> " +
            "on Windows. (Firefox on Linux does not ship WebGPU yet.)</p>" +
            '<p class="webgpu-fallback__back"><a href="../">&larr; Back to Nova Protocol</a></p>' +
            "</div>";
    }

    if (!navigator.gpu) {
        showFallback();
        return;
    }

    try {
        navigator.gpu.requestAdapter().then(
            function (adapter) {
                if (!adapter) {
                    showFallback();
                }
            },
            function () {
                showFallback();
            },
        );
    } catch (_e) {
        showFallback();
    }
})();
