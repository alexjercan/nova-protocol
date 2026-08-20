# Introduction

The Nova Protocol developer book: how to run the project, how to extend it,
and where things live. Source lives in `docs/` at the repo root and builds
with `nix develop --command mdbook build`.

This book is the LOOKUP layer: search a concept (the search icon, top left),
land on a [Concept index](concept-index.md) row, and leave knowing which
crate, files and entry symbol to open. For API detail past that, run
`cargo doc --open -p <crate>` locally - the book routes, rustdoc documents.

Reading order for a first visit:

1. [Building and running](development.md) - toolchain, everyday commands,
   examples, the web build, releases.
2. [Project tour](project-tour.md) - the crate map and where to change X.
3. The [Architecture](architecture.md) chapters for depth, then the
   [Extending](guide-add-section.md) guides for your change.

Before quoting a millisecond at anyone, read
[Measuring performance](performance.md).

What this book is not:

- API detail. That is rustdoc: `cargo doc --open`, run locally. Every crate
  exposes a `prelude`; the book names modules, rustdoc documents them.
- Player or creator documentation. Players read the
  [wiki](https://alexjercan.github.io/nova-protocol/wiki/); mod authors read
  [Create](https://alexjercan.github.io/nova-protocol/create/) on the site.
  [Keeping docs in sync](keeping-docs-in-sync.md) maps every surface.
