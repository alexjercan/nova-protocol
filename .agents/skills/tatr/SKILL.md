---
name: tatr
description: Create, list, query, and edit Nova's Markdown tasks when tracked work is requested.
---

# Tatr

Tasks live at `tasks/<YYYYMMDD-HHMMSS>/TASK.md`.

```bash
tatr new "Title" -p 0 -t backlog
tatr ls --sort priority
tatr ls --filter ':status eq OPEN'
tatr edit <id> --status IN_PROGRESS
```

Valid statuses are `OPEN`, `IN_PROGRESS`, and `CLOSED`. Use `-r ROOT` for
another project. Edit an existing task body directly. Follow Nova's scheduling
and task-evidence rules in `AGENTS.md`.
