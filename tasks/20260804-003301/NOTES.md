# Notes: Move the design PoC HTML pages into web/design

## What changes

Before: `examples/ui/` holds three `.html` files that are not examples -
`nova_ui_rework_poc.html`, `hud_rework_poc.html`, `nova_os_terminal_poc.html`.
They are accepted design sources with live consumers: `web/webpack.config.js`
copies all three into the built site, and `web/tests/theme.test.ts` reads
`nova_ui_rework_poc.html`'s `:root` block as the single source of truth for
BOTH `crates/nova_ui/src/theme.rs` and `web/src/style.css`, failing when the
two drift. Eight Rust modules and two wiki pages cite them normatively.

After: they live in `web/design/`, every reference points there, and
`examples/ui/` holds only runnable examples - which is what lets
`20260802-120029` give the `ui/` category a contract.

No visual, token, or content change. Pure relocation.

## Surfaces

| File | Why |
| --- | --- |
| `examples/ui/{nova_ui_rework_poc,hud_rework_poc,nova_os_terminal_poc}.html` | The files moved. |
| `web/webpack.config.js:355,359,367` | Three `from:` copy entries. |
| `web/tests/theme.test.ts:23` | `POC_PATH` - the theme drift test's token source. |
| `web/src/style.css:16` | Header comment naming the token source. |
| `crates/nova_ui/src/theme.rs:5`, `crates/nova_ui/src/hud.rs:7` | Normative rustdoc citations. |
| `crates/nova_gameplay/src/hud/{emphasis.rs:4,situation.rs:4,objective_stack.rs:4}` | Same. |
| `crates/nova_gameplay/src/hud/nova_os/{content.rs:8,style.rs:96}` | Same. |
| `crates/nova_gameplay/src/hud/nova_os/tests/structure.rs:279` | Test named after the PoC's structure. |
| `web/src/wiki/dev/development.md:147,378`, `web/src/wiki/dev/keeping-docs-in-sync.md:60` | Docs pointing at the paths. |
| `Cargo.toml` | No change: the three `.html` files have no `[[example]]` entry (only `nova_os_rtt_poc.rs` does, and that is retired separately). |

## Data and interfaces

None. Path strings only.

## Sketches

Illustrative only.

```diff
 // web/webpack.config.js
-                        from: "../examples/ui/nova_ui_rework_poc.html",
+                        from: "../design/nova_ui_rework_poc.html",
```

```diff
 // web/tests/theme.test.ts
-const POC_PATH = join(REPO, "examples", "ui", "nova_ui_rework_poc.html");
+const POC_PATH = join(REPO, "web", "design", "nova_ui_rework_poc.html");
```

## Shape

```
web/design/
  nova_ui_rework_poc.html   --:root tokens--> nova_ui/src/theme.rs
        |                                 \-> web/src/style.css
        |                                        ^
        |                       web/tests/theme.test.ts (drift gate)
        |
  hud_rework_poc.html       --cited by--> nova_gameplay/hud/{emphasis,situation,objective_stack}
  nova_os_terminal_poc.html --cited by--> nova_gameplay/hud/nova_os/{content,style,tests/structure}
        |
        +--> webpack copy --> built site
```

## Consequences and open questions

- Cost: a `git mv` plus ~14 reference updates. The risk is a missed reference,
  which the DoD's repo-wide grep catches.
- `web/design/` is a new directory in the web workspace. It is NOT `web/src/`,
  so webpack's copy `from:` becomes `../design/...` relative to `web/src`;
  worth confirming against the existing copy-plugin context.
- Open: whether the wiki should keep calling them "PoC" now that they are
  permanent design sources. Renaming the files themselves is a bigger diff
  (every citation carries the name), so keep the names in this task.
- Open: whether `hud_rework_poc.html` and `nova_os_terminal_poc.html` should
  also be copied into the built site - they already are; that stays as-is.
