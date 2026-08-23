# Broadside baseline harness, recovered from two sprouts

Archived 2026-08-23, when the `v10-baseline` and `v11-measure` sprouts were
removed. Both carried uncommitted work: git could have recovered neither
`broadside_baseline.rs`, since both were untracked.

Every commit in both worktrees was already reachable from `master`, so nothing
here is unmerged history - only the working-tree state each lane was measuring
with.

| Sprout | Detached HEAD | What it held |
| --- | --- | --- |
| `v10-baseline` | `19495a39` (`chore(release): v0.10.0`) | the v0.10.0 side of the comparison: a 168-line `examples/stress/broadside_baseline.rs` plus frametime instrumentation in `nova_probe` |
| `v11-measure` | `6feaba8d` (`Price the cracks material and reject the pipeline spike`) | the v0.11.0 side: a 239-line `examples/systems/broadside_baseline.rs` and its Cargo wiring |

Note the two examples sit in DIFFERENT categories - `stress` on the v0.10.0 side,
`systems` on the v0.11.0 side - so they are not two revisions of one file.

## Restoring one

The patches are plain `git diff` output against the HEAD in the table, so they
apply to a worktree at that commit:

```sh
sprout new v10-baseline
cd "$(sprout show v10-baseline)"
git checkout 19495a39
git apply /path/to/v10-baseline.patch
cp -r /path/to/v10-baseline/examples .
```

The untracked files are stored verbatim under `<sprout>/`, at the path they had
in the worktree, so the `cp` lands them where the patch expects.
