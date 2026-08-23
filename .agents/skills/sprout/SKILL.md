---
name: sprout
description: Create, inspect, synchronize, land, and remove Nova's isolated Git worktrees when the user requests one.
---

# Sprout

Use Sprout only when the user requests an isolated worktree.

```bash
sprout new <feature> [--task <id>]
sprout ls
sprout show <feature>
sprout sync <feature> [--dry-run]
sprout land <feature> [--dry-run] [--remove] -m <subject>
sprout rm <feature>
```

Run work in the path printed by `sprout new`. Keep each worktree's own
`target/`; the dev shell shares compilation through sccache. Sync and re-verify
before landing. Land from the main checkout. Remove resources only with user
approval.
