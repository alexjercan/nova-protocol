# Review: NOVA OS persistent header + main + footer

- VERDICT: APPROVE

Final verdict after round 2. The per-round verdicts are in each section below.

## Round 1 (out-of-context reviewer) - REQUEST_CHANGES

Reviewed `git diff master...HEAD` (nova_os.rs, app.rs). check green, fmt clean,
breadcrumb unit test present. Core restructure judged sound: the app root is an
absolute-fill child of a `Relative` `<main>` that flex-grows between fixed-height
header/footer siblings, so it is geometrically contained in `<main>` and cannot
cover header/footer regardless of z; dropping the app safe-area padding +
footer-reserve margin is safe because `<main>` inherits the content root's
percentage safe-area inset and sits above the footer (RTT + headless both).

Findings:

- MINOR - `rebuild_nova_os_footer_hints` lacked the just-spawned override the new
  `reconcile_nova_os_header` has; a shell respawn whose mode matches the stale
  `Local` would skip refilling the new footer (latent, masked today because
  `spawn_nova_os_footer` seeds defaults and `reset_session` forces Prompt).
- MINOR - no integration test for `reconcile_nova_os_header` (breadcrumb swap +
  close-button visibility toggle - DoD 2 + 4 - only manually verified).
- MINOR - `NovaOsAppRuntime::title()` is now a required trait method with no
  production consumer (breadcrumb uses `id`).
- NIT - footer `flex_wrap: Wrap` + `overflow: clip()` on a fixed-height bar
  silently clips a wrapped overflow row (owner-accepted tradeoff; no action).
- NIT - inert `row_gap` left on the now-single-child terminal content node.

VERDICT: REQUEST_CHANGES

## Round 2 (fixes) - resolved

- MINOR footer: gave `rebuild_nova_os_footer_hints` the same
  `Added<NovaOsFooterHintsMarker>` override (`q_added`), mirroring the header, so
  both sibling reconcilers force a refresh on respawn.
- MINOR test: added `header_reconciles_breadcrumb_and_close_control_across_the_swap`
  - registers `reconcile_nova_os_header` in a live tree and asserts the brand text
  (`// SHELL` <-> `// APPS / SAMPLE`) and `NovaOsAppCloseMarker` visibility
  (Hidden <-> Inherited) across enter_app / exit_app.
- MINOR title: added a default impl (`fn title(&self) -> &'static str { self.id() }`)
  so apps need not supply an unused string; documented as informational.
- NIT: removed the inert `row_gap` from the terminal content node.
- NIT footer clip: left as the owner-accepted tradeoff (documented in code).

Re-verified: `cargo check -p nova_gameplay -p nova_os` green, `cargo fmt` clean,
6 affected/new tests pass, and the `screenshot_nova_os` reel re-rendered with the
header/main/footer intact (welcome/active `// SHELL`, map `// APPS / MAP` + close).

VERDICT: APPROVE
