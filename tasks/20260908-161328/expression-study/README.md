# Reusable expression trial

Compare [comic colors](expressions-comic.svg) and [lore colors](expressions-lore.svg).
Open either SVG directly in a browser and zoom. Each row keeps the same head
identity and body pose while comparing original and new facial features.

- Rina: an amused, closed-mouth smile for the coffee-break offer.
- Jonah: a wry half-smile for his dry reply.

The [working opening](../comic-opening-poc/index.html) uses these two variants
on page 1 only. The user accepted this pair and requested its commit, not a new
default for every appearance. The [review](../EXPRESSION-REVIEW.md) records scope
and checks.

`generate.py` owns this comparison layout and its labels, not facial geometry.
Reusable features live in
[`scripts/nova_illustration/expressions.py`](../../../scripts/nova_illustration/expressions.py)
and are selected through the
[shared head and body APIs](../../../scripts/nova_illustration/README.md#facial-expressions).
Original renders, other characters, public portraits, the accepted three-shot
study, and pages 2-4 remain unchanged. No new story scene is added.

From the repository root:

```bash
python3 tasks/20260908-161328/expression-study/generate.py
python3 tasks/20260908-161328/expression-study/generate.py --check
```

The generator writes only the two comparison SVGs here. The lore sheet applies
the shared color-only shader to scene art, not labels. These task-local sheets
are not public website inputs. Do not hand-edit their generated SVGs.
