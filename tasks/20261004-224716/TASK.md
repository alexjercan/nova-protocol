# Fix master CI, rebase the active PR stack, and verify green mergeability

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.15.0, ci, pr

## User facts

- Record the follow-up now. Do not start CI fixes or PR rebases until the canister snapshot/bench-view change is finished and committed on master.
- Fix failing master CI, then rebase the active PR stack onto updated master, then ensure required CI is green and the stack is mergeable.

## Agent findings

- PR/branch topology and current failed CI jobs are not yet established for this request. Inspect live GitHub state and local Sprout branches after the prerequisite commit. Do not assume that an older reported run is still the active failure.
- A rebase rewrites branch history. Preserve and record old heads, verify that each branch has no uncommitted work, and do not force-update an unexpected or shared head.

## Delivery checklist

- [ ] After the canister change is committed, identify the current master CI failure(s); reproduce the affected check locally, fix only the cause on master, run focused proof, and push the verified fix.
- [ ] Identify all open PRs in the actual dependency stack and their current bases/heads. Rebase bottom-up onto the updated base, resolve conflicts with focused checks, and push only expected rebased branches with lease protection.
- [ ] Check each PR's required checks and mergeability against its new base. Address failures, rerun as needed, and record PR links, head SHAs, check-run URLs, and any external blocker.

## Verification and done when

- Master required CI is green at the relevant head, not just a local test.
- Each stacked PR reports the expected base/head, required checks green, and mergeable status; report any pending review or branch-protection requirement separately from CI.
- Do not claim the stack mergeable based on stale checks or exit status alone.
